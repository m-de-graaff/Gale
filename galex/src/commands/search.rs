//! `gale search` — search the package registry.

use crate::registry::client::RegistryClient;

/// Run the `gale search` command.
pub fn run(query: &str) -> i32 {
    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    rt.block_on(async {
        let client = RegistryClient::new();
        match client.search(query).await {
            Ok(results) => {
                if results.is_empty() {
                    eprintln!("  No packages found for '{query}'");
                } else {
                    eprintln!(
                        "  Found {} package{}:",
                        results.len(),
                        if results.len() != 1 { "s" } else { "" }
                    );
                    eprintln!();
                    for pkg in &results {
                        eprintln!("  {:<30} v{}", pkg.name, pkg.version);
                    }
                }
                0
            }
            Err(e) => {
                eprintln!("  error: {e}");
                1
            }
        }
    })
}
