use criterion::{Criterion, black_box, criterion_group, criterion_main};

use gale_lib::cache::extract_extension;
use gale_lib::compression::ShouldCompress;
use gale_lib::config::CompressionConfig;
use gale_lib::logging::{days_to_civil, format_clf_timestamp};
use gale_lib::mime_types;
use gale_lib::security::path::{PathSecurityState, validate_path};
use gale_lib::static_files::health_handler;

use axum::body::Body;
use axum::http::{Response, StatusCode, header};

// ---------------------------------------------------------------------------
// Group 1: Path security validation
// ---------------------------------------------------------------------------

fn bench_path_security(c: &mut Criterion) {
    let state = PathSecurityState {
        canonical_root: std::env::current_dir().expect("cwd"),
        block_dotfiles: true,
    };

    let mut group = c.benchmark_group("path_security");

    group.bench_function("clean_short", |b| {
        b.iter(|| validate_path(black_box("/index.html"), &state))
    });

    group.bench_function("clean_deep", |b| {
        b.iter(|| {
            validate_path(
                black_box("/assets/css/vendor/bootstrap/main.css"),
                &state,
            )
        })
    });

    group.bench_function("percent_encoded", |b| {
        b.iter(|| validate_path(black_box("/hello%20world%21.html"), &state))
    });

    group.bench_function("rejected_dotfile", |b| {
        b.iter(|| validate_path(black_box("/.env"), &state))
    });

    group.bench_function("rejected_traversal", |b| {
        b.iter(|| validate_path(black_box("/../etc/passwd"), &state))
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Group 2: MIME type lookup
// ---------------------------------------------------------------------------

fn bench_mime_lookup(c: &mut Criterion) {
    let mut group = c.benchmark_group("mime_lookup");

    for ext in &["html", "css", "js", "png", "jpg", "woff2", "xyz"] {
        group.bench_function(*ext, |b| {
            b.iter(|| mime_types::from_extension(black_box(ext)))
        });
    }

    group.finish();
}

// ---------------------------------------------------------------------------
// Group 3: Compression decision
// ---------------------------------------------------------------------------

fn bench_compression_decision(c: &mut Criterion) {
    let config = CompressionConfig {
        enabled: true,
        min_size: 1024,
        algorithms: vec!["br".into(), "gzip".into()],
        pre_compressed: true,
        skip_extensions: vec![
            "png", "jpg", "jpeg", "gif", "webp", "avif", "woff2", "woff", "mp4", "webm", "ogg",
            "zip", "gz", "br", "zst",
        ]
        .into_iter()
        .map(String::from)
        .collect(),
    };
    let predicate = ShouldCompress::from_config(&config);

    let mut group = c.benchmark_group("compression_decision");

    // Compressible HTML 2KB
    let html_resp = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
        .header(header::CONTENT_LENGTH, "2048")
        .body(Body::empty())
        .unwrap();
    group.bench_function("html_2kb_compress", |b| {
        b.iter(|| {
            use tower_http::compression::predicate::Predicate;
            predicate.should_compress(black_box(&html_resp))
        })
    });

    // Image/png 50KB — skip list
    let png_resp = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "image/png")
        .header(header::CONTENT_LENGTH, "51200")
        .body(Body::empty())
        .unwrap();
    group.bench_function("png_50kb_skip", |b| {
        b.iter(|| {
            use tower_http::compression::predicate::Predicate;
            predicate.should_compress(black_box(&png_resp))
        })
    });

    // Small text 512B — below min_size
    let small_resp = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "text/html")
        .header(header::CONTENT_LENGTH, "512")
        .body(Body::empty())
        .unwrap();
    group.bench_function("small_text_below_min", |b| {
        b.iter(|| {
            use tower_http::compression::predicate::Predicate;
            predicate.should_compress(black_box(&small_resp))
        })
    });

    // No Content-Type
    let no_ct_resp = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_LENGTH, "2048")
        .body(Body::empty())
        .unwrap();
    group.bench_function("no_content_type", |b| {
        b.iter(|| {
            use tower_http::compression::predicate::Predicate;
            predicate.should_compress(black_box(&no_ct_resp))
        })
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Group 4: Cache extension extraction
// ---------------------------------------------------------------------------

fn bench_cache_extension(c: &mut Criterion) {
    let mut group = c.benchmark_group("cache_extension");

    group.bench_function("index_html", |b| {
        b.iter(|| extract_extension(black_box("/index.html")))
    });

    group.bench_function("assets_css", |b| {
        b.iter(|| extract_extension(black_box("/assets/style.css")))
    });

    group.bench_function("directory_trailing_slash", |b| {
        b.iter(|| extract_extension(black_box("/about/")))
    });

    group.bench_function("no_extension", |b| {
        b.iter(|| extract_extension(black_box("/README")))
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Group 5: Logging timestamp formatting
// ---------------------------------------------------------------------------

fn bench_logging_timestamp(c: &mut Criterion) {
    let mut group = c.benchmark_group("logging_timestamp");

    group.bench_function("format_clf_timestamp", |b| {
        b.iter(format_clf_timestamp)
    });

    group.bench_function("days_to_civil", |b| {
        b.iter(|| days_to_civil(black_box(20528)))
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Group 6: Health handler (async)
// ---------------------------------------------------------------------------

fn bench_health_handler(c: &mut Criterion) {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio runtime");

    let mut group = c.benchmark_group("health_handler");

    group.bench_function("health", |b| {
        b.iter(|| rt.block_on(async { health_handler().await }))
    });

    group.finish();
}

// ---------------------------------------------------------------------------
// Criterion harness
// ---------------------------------------------------------------------------

criterion_group!(
    benches,
    bench_path_security,
    bench_mime_lookup,
    bench_compression_decision,
    bench_cache_extension,
    bench_logging_timestamp,
    bench_health_handler,
);
criterion_main!(benches);
