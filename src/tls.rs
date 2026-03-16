use std::process;
use std::time::{Duration, SystemTime};

use axum::extract::Request;
use axum::http::Uri;
use axum::response::{IntoResponse, Redirect, Response};
use axum::Router;

use crate::config::TlsConfig;

pub fn validate_tls_config(config: &TlsConfig) {
    if !config.enabled {
        return;
    }

    if config.acme {
        // ACME mode: email and domain required, cert/key paths NOT required
        if config.acme_email.is_empty() {
            tracing::error!("ACME enabled but no email configured (tls.acme_email)");
            process::exit(1);
        }
        if config.acme_domain.is_empty() {
            tracing::error!("ACME enabled but no domain configured (tls.acme_domain)");
            process::exit(1);
        }
    } else {
        // Static cert mode: cert/key paths required
        if config.cert.is_empty() {
            tracing::error!("TLS enabled but no certificate path configured (tls.cert)");
            process::exit(1);
        }
        if config.key.is_empty() {
            tracing::error!("TLS enabled but no private key path configured (tls.key)");
            process::exit(1);
        }
        if !std::path::Path::new(&config.cert).exists() {
            tracing::error!(path = %config.cert, "TLS certificate file not found");
            process::exit(1);
        }
        if !std::path::Path::new(&config.key).exists() {
            tracing::error!(path = %config.key, "TLS private key file not found");
            process::exit(1);
        }
    }
}

pub async fn build_rustls_config(
    config: &TlsConfig,
) -> axum_server::tls_rustls::RustlsConfig {
    axum_server::tls_rustls::RustlsConfig::from_pem_file(&config.cert, &config.key)
        .await
        .unwrap_or_else(|e| {
            tracing::error!(%e, "failed to load TLS certificate/key");
            process::exit(1);
        })
}

/// Build a RustlsConfig backed by ACME automatic certificate management.
///
/// Returns the axum-server RustlsConfig and a JoinHandle for the background
/// ACME renewal task.
pub async fn build_acme_rustls_config(
    config: &TlsConfig,
) -> (
    axum_server::tls_rustls::RustlsConfig,
    tokio::task::JoinHandle<()>,
) {
    use rustls_acme::caches::DirCache;
    use rustls_acme::AcmeConfig;

    // Ensure cache directory exists
    if let Err(e) = std::fs::create_dir_all(&config.acme_cache_dir) {
        tracing::error!(
            path = %config.acme_cache_dir,
            %e,
            "cannot create ACME cache directory"
        );
        process::exit(1);
    }

    let domain = config.acme_domain.clone();
    let email = format!("mailto:{}", config.acme_email);
    let cache_dir = config.acme_cache_dir.clone();
    let production = config.acme_production;

    let acme_config = AcmeConfig::new([domain])
        .contact_push(email)
        .cache(DirCache::new(cache_dir))
        .directory_lets_encrypt(production);

    let mut acme_state = acme_config.state();
    let server_config = acme_state.default_rustls_config();

    let axum_rustls_config =
        axum_server::tls_rustls::RustlsConfig::from_config(server_config);

    if config.acme_production {
        tracing::info!(
            domain = %config.acme_domain,
            "ACME enabled (Let's Encrypt production)"
        );
    } else {
        tracing::info!(
            domain = %config.acme_domain,
            "ACME enabled (Let's Encrypt staging)"
        );
    }

    // Spawn background task to drive ACME renewal
    let renewal_task = tokio::spawn(async move {
        use futures_util::StreamExt;

        loop {
            match acme_state.next().await {
                Some(Ok(ok)) => tracing::info!("ACME event: {ok:?}"),
                Some(Err(err)) => tracing::warn!("ACME error: {err:?}"),
                None => break,
            }
        }
    });

    (axum_rustls_config, renewal_task)
}

/// Spawn a background task that polls cert/key file mtimes every 10 seconds.
/// When a change is detected, calls reload_from_pem_file().
pub fn spawn_cert_reload_task(
    config: &TlsConfig,
    rustls_config: axum_server::tls_rustls::RustlsConfig,
) -> tokio::task::JoinHandle<()> {
    let cert_path = config.cert.clone();
    let key_path = config.key.clone();

    tokio::spawn(async move {
        let mut last_cert_mtime = file_mtime(&cert_path);
        let mut last_key_mtime = file_mtime(&key_path);

        let mut interval = tokio::time::interval(Duration::from_secs(10));
        interval.tick().await; // consume the immediate first tick

        loop {
            interval.tick().await;

            let current_cert_mtime = file_mtime(&cert_path);
            let current_key_mtime = file_mtime(&key_path);

            if current_cert_mtime != last_cert_mtime || current_key_mtime != last_key_mtime {
                tracing::info!("TLS certificate/key file change detected, reloading");
                match rustls_config
                    .reload_from_pem_file(&cert_path, &key_path)
                    .await
                {
                    Ok(()) => {
                        tracing::info!("TLS certificate reloaded successfully");
                        last_cert_mtime = current_cert_mtime;
                        last_key_mtime = current_key_mtime;
                    }
                    Err(e) => {
                        tracing::warn!(
                            %e,
                            "failed to reload TLS certificate, continuing with previous"
                        );
                    }
                }
            }
        }
    })
}

fn file_mtime(path: &str) -> Option<SystemTime> {
    std::fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
}

pub fn redirect_router(https_port: u16) -> Router {
    Router::new().fallback(move |req: Request| async move {
        let host = req
            .headers()
            .get("host")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("localhost");
        redirect_to_https(host, req.uri(), https_port)
    })
}

fn redirect_to_https(host: &str, uri: &Uri, https_port: u16) -> Response {
    // Strip any existing port from the host
    let host_without_port = host.split(':').next().unwrap_or(host);

    let authority = if https_port == 443 {
        host_without_port.to_string()
    } else {
        format!("{host_without_port}:{https_port}")
    };

    let path_and_query = uri
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/");

    let location = format!("https://{authority}{path_and_query}");

    Redirect::permanent(&location).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    fn response_location(response: &Response) -> &str {
        response
            .headers()
            .get("location")
            .unwrap()
            .to_str()
            .unwrap()
    }

    #[test]
    fn redirect_to_https_standard_port() {
        let uri = "/page".parse::<Uri>().unwrap();
        let resp = redirect_to_https("example.com", &uri, 443);
        assert_eq!(resp.status(), StatusCode::PERMANENT_REDIRECT);
        assert_eq!(response_location(&resp), "https://example.com/page");
    }

    #[test]
    fn redirect_to_https_custom_port() {
        let uri = "/page".parse::<Uri>().unwrap();
        let resp = redirect_to_https("example.com", &uri, 8443);
        assert_eq!(resp.status(), StatusCode::PERMANENT_REDIRECT);
        assert_eq!(response_location(&resp), "https://example.com:8443/page");
    }

    #[test]
    fn redirect_strips_source_port() {
        let uri = "/".parse::<Uri>().unwrap();
        let resp = redirect_to_https("example.com:8080", &uri, 443);
        assert_eq!(response_location(&resp), "https://example.com/");
    }

    #[test]
    fn redirect_preserves_path_and_query() {
        let uri = "/a/b?x=1&y=2".parse::<Uri>().unwrap();
        let resp = redirect_to_https("example.com", &uri, 443);
        assert_eq!(
            response_location(&resp),
            "https://example.com/a/b?x=1&y=2"
        );
    }

    #[test]
    fn validate_tls_config_disabled_is_noop() {
        let config = TlsConfig {
            enabled: false,
            cert: String::new(),
            key: String::new(),
            redirect_port: 80,
            acme: false,
            acme_email: String::new(),
            acme_domain: String::new(),
            acme_cache_dir: String::new(),
            acme_production: false,
        };
        validate_tls_config(&config);
    }

    #[test]
    fn file_mtime_missing_file_returns_none() {
        assert!(file_mtime("/nonexistent/path/to/file.pem").is_none());
    }

    #[test]
    fn file_mtime_existing_file_returns_some() {
        assert!(file_mtime("Cargo.toml").is_some());
    }
}
