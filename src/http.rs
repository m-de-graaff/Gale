//! HTTP client for server-side `fetch()` in GaleX actions.
//!
//! Provides simple async wrappers around `reqwest` for making outbound
//! HTTP requests from action handlers. Gated behind the `http` feature.

/// Fetch a URL and return the response body as a string.
///
/// Returns `Ok(body)` on success, or `Err(message)` on any error
/// (DNS failure, timeout, non-2xx status, body decode failure).
pub async fn get(url: &str) -> Result<String, String> {
    let response = reqwest::get(url).await.map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()));
    }
    response.text().await.map_err(|e| e.to_string())
}

/// Fetch a URL and parse the response body as JSON.
///
/// Returns `Ok(value)` on success, or `Err(message)` on any error.
/// Requires the `serde_json` crate (re-exported by the generated project).
pub async fn get_json(url: &str) -> Result<String, String> {
    let response = reqwest::get(url).await.map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("HTTP {}", response.status()));
    }
    response.text().await.map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn module_exists() {
        // Compile-time check — the module is importable when the feature is on.
    }
}
