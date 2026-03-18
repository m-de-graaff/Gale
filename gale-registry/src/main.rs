//! GaleX Package Registry Server.
//!
//! A lightweight registry for GaleX packages, backed by SQLite
//! and filesystem tarball storage.
//!
//! API:
//! - `GET  /api/packages/:name`            — package metadata (latest)
//! - `GET  /api/packages/:name/:version`   — specific version metadata
//! - `GET  /api/packages/:name/:version/download` — download tarball
//! - `POST /api/packages`                  — publish (auth required)
//! - `GET  /api/search?q=...`              — search packages

mod db;

use std::path::PathBuf;
use std::sync::Mutex;

use axum::extract::{Multipart, Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Shared application state.
struct AppState {
    db: Mutex<Connection>,
    storage_dir: PathBuf,
    base_url: String,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let data_dir = PathBuf::from(
        std::env::var("REGISTRY_DATA").unwrap_or_else(|_| "./registry_data".into()),
    );
    let storage_dir = data_dir.join("tarballs");
    std::fs::create_dir_all(&storage_dir).expect("failed to create storage dir");

    let db_path = data_dir.join("registry.db");
    let conn = Connection::open(&db_path).expect("failed to open database");
    db::init(&conn).expect("failed to initialize database");

    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(4873);
    let base_url = std::env::var("BASE_URL")
        .unwrap_or_else(|_| format!("http://localhost:{port}"));

    let state = std::sync::Arc::new(AppState {
        db: Mutex::new(conn),
        storage_dir,
        base_url,
    });

    let app = Router::new()
        .route("/api/packages/:name", get(get_package))
        .route("/api/packages/:name/:version/download", get(download_package))
        .route("/api/packages", post(publish_package))
        .route("/api/search", get(search_packages))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
        .await
        .unwrap();
    tracing::info!("Registry server listening on port {port}");
    axum::serve(listener, app).await.unwrap();
}

/// Package metadata response.
#[derive(Serialize)]
struct PackageResponse {
    name: String,
    version: String,
    checksum: String,
    signature: Option<String>,
    dependencies: Vec<String>,
    download_url: String,
}

// ── Handlers ───────────────────────────────────────────────────────────

/// GET /api/packages/:name — get latest version metadata.
async fn get_package(
    State(state): State<std::sync::Arc<AppState>>,
    Path(name): Path<String>,
) -> impl IntoResponse {
    let db = state.db.lock().unwrap();
    match db::get_latest_version(&db, &name) {
        Ok(Some(ver)) => {
            let deps: Vec<String> =
                serde_json::from_str(&ver.dependencies).unwrap_or_default();
            Json(PackageResponse {
                name: name.clone(),
                version: ver.version.clone(),
                checksum: ver.checksum,
                signature: None,
                dependencies: deps,
                download_url: format!(
                    "{}/api/packages/{}/{}/download",
                    state.base_url, name, ver.version
                ),
            })
            .into_response()
        }
        Ok(None) => (StatusCode::NOT_FOUND, "package not found").into_response(),
        Err(e) => {
            tracing::error!("DB error: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "internal error").into_response()
        }
    }
}

/// GET /api/packages/:name/:version/download — download tarball.
async fn download_package(
    State(state): State<std::sync::Arc<AppState>>,
    Path((name, version)): Path<(String, String)>,
) -> impl IntoResponse {
    let tarball_path = state
        .storage_dir
        .join(format!("{}_{}.tar.gz", name.replace('/', "_"), version));

    match std::fs::read(&tarball_path) {
        Ok(data) => (
            [(
                axum::http::header::CONTENT_TYPE,
                "application/gzip",
            )],
            data,
        )
            .into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "tarball not found").into_response(),
    }
}

/// POST /api/packages — publish a new version.
async fn publish_package(
    State(state): State<std::sync::Arc<AppState>>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> impl IntoResponse {
    // Verify auth token
    let token = match headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
    {
        Some(t) => t.to_string(),
        None => return (StatusCode::UNAUTHORIZED, "missing auth token").into_response(),
    };

    let token_hash = format!("{:x}", Sha256::digest(token.as_bytes()));
    {
        let db = state.db.lock().unwrap();
        match db::verify_token(&db, &token_hash) {
            Ok(Some(_username)) => {} // Valid token
            _ => return (StatusCode::UNAUTHORIZED, "invalid token").into_response(),
        }
    }

    // Parse multipart form
    let mut name = String::new();
    let mut version = String::new();
    let mut description = String::new();
    let mut checksum = String::new();
    let mut tarball_data = Vec::new();

    while let Ok(Some(field)) = multipart.next_field().await {
        let field_name = field.name().unwrap_or("").to_string();
        match field_name.as_str() {
            "name" => name = field.text().await.unwrap_or_default(),
            "version" => version = field.text().await.unwrap_or_default(),
            "description" => description = field.text().await.unwrap_or_default(),
            "checksum" => checksum = field.text().await.unwrap_or_default(),
            "tarball" => tarball_data = field.bytes().await.unwrap_or_default().to_vec(),
            _ => {}
        }
    }

    if name.is_empty() || version.is_empty() || tarball_data.is_empty() {
        return (StatusCode::BAD_REQUEST, "missing required fields").into_response();
    }

    // Verify checksum
    let actual_checksum = format!("{:x}", Sha256::digest(&tarball_data));
    if actual_checksum != checksum {
        return (StatusCode::BAD_REQUEST, "checksum mismatch").into_response();
    }

    // Save tarball
    let tarball_filename = format!("{}_{}.tar.gz", name.replace('/', "_"), version);
    let tarball_path = state.storage_dir.join(&tarball_filename);
    if let Err(e) = std::fs::write(&tarball_path, &tarball_data) {
        tracing::error!("Failed to write tarball: {e}");
        return (StatusCode::INTERNAL_SERVER_ERROR, "storage error").into_response();
    }

    // Insert into database
    let db = state.db.lock().unwrap();
    let pkg_id = match db::upsert_package(&db, &name, &description, "", "") {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("DB error: {e}");
            return (StatusCode::INTERNAL_SERVER_ERROR, "database error").into_response();
        }
    };
    if let Err(e) = db::insert_version(
        &db,
        pkg_id,
        &version,
        &checksum,
        "",
        "[]",
        &tarball_filename,
    ) {
        tracing::error!("DB error: {e}");
        return (StatusCode::INTERNAL_SERVER_ERROR, "database error").into_response();
    }
    let _ = db::update_search_index(&db, &name, &description);

    (StatusCode::CREATED, "published").into_response()
}

/// GET /api/search?q=... — search packages.
#[derive(Deserialize)]
struct SearchQuery {
    q: String,
}

async fn search_packages(
    State(state): State<std::sync::Arc<AppState>>,
    Query(query): Query<SearchQuery>,
) -> impl IntoResponse {
    let db = state.db.lock().unwrap();
    match db::search_packages(&db, &query.q) {
        Ok(results) => {
            let response: Vec<serde_json::Value> = results
                .into_iter()
                .map(|r| {
                    serde_json::json!({
                        "name": r.name,
                        "version": r.version,
                        "description": r.description,
                        "checksum": "",
                        "signature": null,
                        "dependencies": [],
                        "download_url": ""
                    })
                })
                .collect();
            Json(response).into_response()
        }
        Err(e) => {
            tracing::error!("Search error: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, "search error").into_response()
        }
    }
}
