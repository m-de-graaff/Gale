//! `gale login` — authenticate with the package registry.

/// Run the `gale login` command.
pub fn run() -> i32 {
    let token: String = match dialoguer::Input::new()
        .with_prompt("API token (from registry.get-gale.vercel.app)")
        .interact_text()
    {
        Ok(t) => t,
        Err(_) => {
            eprintln!("  error: failed to read token");
            return 1;
        }
    };

    if token.trim().is_empty() {
        eprintln!("  error: token cannot be empty");
        return 1;
    }

    // Store in ~/.gale/credentials
    let home = match dirs_next::home_dir() {
        Some(h) => h,
        None => {
            eprintln!("  error: could not determine home directory");
            return 1;
        }
    };

    let creds_dir = home.join(".gale");
    if let Err(e) = std::fs::create_dir_all(&creds_dir) {
        eprintln!("  error creating ~/.gale: {e}");
        return 1;
    }

    if let Err(e) = std::fs::write(creds_dir.join("credentials"), token.trim()) {
        eprintln!("  error saving token: {e}");
        return 1;
    }

    eprintln!("  Token saved to ~/.gale/credentials");
    0
}
