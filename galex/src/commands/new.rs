//! `gale new` — interactive project scaffolding.

use super::scaffold::{AuthChoice, DbChoice, ScaffoldOptions};

/// Run the `gale new` command with interactive prompts.
pub fn run(name: Option<String>) -> i32 {
    let project_name = name.unwrap_or_else(|| {
        dialoguer::Input::new()
            .with_prompt("Project name")
            .default("my-gale-app".into())
            .interact_text()
            .unwrap_or_else(|_| "my-gale-app".into())
    });

    let include_tailwind = dialoguer::Confirm::new()
        .with_prompt("Include Tailwind CSS?")
        .default(true)
        .interact()
        .unwrap_or(true);

    let include_example = dialoguer::Confirm::new()
        .with_prompt("Include example pages?")
        .default(true)
        .interact()
        .unwrap_or(true);

    let db_options = &["None", "PostgreSQL", "SQLite"];
    let db_idx = dialoguer::Select::new()
        .with_prompt("Database adapter")
        .items(db_options)
        .default(0)
        .interact()
        .unwrap_or(0);
    let db = match db_idx {
        1 => DbChoice::Postgres,
        2 => DbChoice::Sqlite,
        _ => DbChoice::None,
    };

    let auth_options = &["None", "Session-based", "JWT"];
    let auth_idx = dialoguer::Select::new()
        .with_prompt("Authentication")
        .items(auth_options)
        .default(0)
        .interact()
        .unwrap_or(0);
    let auth = match auth_idx {
        1 => AuthChoice::Session,
        2 => AuthChoice::Jwt,
        _ => AuthChoice::None,
    };

    eprintln!();
    eprintln!("  Creating project '{project_name}'...");

    let opts = ScaffoldOptions {
        name: project_name.clone(),
        tailwind: include_tailwind,
        example: include_example,
        db,
        auth,
    };

    if let Err(e) = super::scaffold::generate_project(&opts) {
        eprintln!("  error: {e}");
        return 1;
    }

    // Install Tailwind dependencies if enabled
    if include_tailwind {
        eprintln!("  Installing dependencies...");
        let npm = if cfg!(windows) { "cmd" } else { "npm" };
        let args: &[&str] = if cfg!(windows) {
            &["/c", "npm", "install"]
        } else {
            &["install"]
        };
        match std::process::Command::new(npm)
            .args(args)
            .current_dir(&project_name)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::inherit())
            .status()
        {
            Ok(s) if s.success() => {}
            Ok(_) => {
                eprintln!("  warning: npm install failed — run it manually:");
                eprintln!("    cd {project_name} && npm install");
            }
            Err(_) => {
                eprintln!("  warning: npm not found — install Node.js, then run:");
                eprintln!("    cd {project_name} && npm install");
            }
        }
    }

    eprintln!("  Project created successfully!");
    eprintln!();
    eprintln!("  Next steps:");
    eprintln!("    cd {project_name}");
    eprintln!("    gale dev");
    eprintln!();
    0
}
