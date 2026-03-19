//! `gale new` — interactive project scaffolding.

use super::scaffold::{ScaffoldOptions, TemplateChoice};

/// Run the `gale new` command with interactive prompts.
pub fn run(name: Option<String>) -> i32 {
    let project_name = name.unwrap_or_else(|| {
        dialoguer::Input::new()
            .with_prompt("Project name")
            .default("my-gale-app".into())
            .interact_text()
            .unwrap_or_else(|_| "my-gale-app".into())
    });

    let template_options = &["Default (Recommended)", "E-commerce", "Chat App"];
    let template_idx = dialoguer::Select::new()
        .with_prompt("Template")
        .items(template_options)
        .default(0)
        .interact()
        .unwrap_or(0);
    let template = match template_idx {
        1 => TemplateChoice::Ecommerce,
        2 => TemplateChoice::ChatApp,
        _ => TemplateChoice::Default,
    };

    let include_tailwind = dialoguer::Confirm::new()
        .with_prompt("Include Tailwind CSS?")
        .default(true)
        .interact()
        .unwrap_or(true);

    eprintln!();
    eprintln!("  Creating project '{project_name}'...");

    let opts = ScaffoldOptions {
        name: project_name.clone(),
        tailwind: include_tailwind,
        template,
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

    // Pre-build the project so `gale dev` starts fast (seconds not minutes).
    eprintln!("  Building project...");
    let project_path = std::path::Path::new(&project_name);
    let app_dir = project_path.join("app");
    let output_dir = project_path.join(".gale_dev");

    match prebuild_project(&app_dir, &output_dir, &project_name) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("  warning: pre-build failed: {e}");
            eprintln!("  The project will be built on first `gale dev` run.");
        }
    }

    eprintln!("  Project created successfully!");
    eprintln!();

    // Windows Defender exclusion hint — real-time scanning of build artifacts
    // adds 30-60s to a full build. Show this once so users can opt in.
    #[cfg(windows)]
    {
        let output_abs = std::path::Path::new(&project_name)
            .join(".gale_dev")
            .canonicalize()
            .ok();
        if let Some(dev_dir) = output_abs {
            let dev_path = dev_dir.display();
            eprintln!("  Tip: Exclude build directories from Windows Defender for faster builds:");
            eprintln!("    powershell -Command \"Add-MpPreference -ExclusionPath '{dev_path}'\"");
            eprintln!();
        }
    }

    eprintln!("  Next steps:");
    eprintln!("    cd {project_name}");
    eprintln!("    gale dev");
    eprintln!();
    0
}

/// Run the full build pipeline during `gale new` so `gale dev` starts fast.
fn prebuild_project(
    app_dir: &std::path::Path,
    output_dir: &std::path::Path,
    project_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    use crate::compiler::Compiler;
    use crate::router;

    // Route discovery
    let routes = router::discovery::discover_routes(app_dir).map_err(|errs| {
        let msg = errs
            .iter()
            .map(|e| e.message.clone())
            .collect::<Vec<_>>()
            .join("; ");
        msg
    })?;

    // Parse + check
    let mut compiler = Compiler::new();
    for route in &routes {
        let files = std::iter::once(&route.page_file)
            .chain(route.layouts.iter())
            .chain(route.guards.iter())
            .chain(route.middleware.iter());
        for path in files {
            let _ = compiler.add_file_dedup(path);
        }
    }
    compiler.parse_all();
    compiler.check();

    // Codegen
    compiler.set_routes(routes);
    compiler.generate(
        &format!("{project_name}_dev_app"),
        output_dir,
        None,
        true, // dev mode
    )?;

    // CSS generation (non-fatal)
    let project_dir = app_dir.parent().unwrap_or(std::path::Path::new("."));
    if let Err(e) = compiler.generate_css(project_dir, app_dir, output_dir, true) {
        eprintln!("  warning: CSS generation failed: {e}");
    }

    // Cargo build
    let status = std::process::Command::new("cargo")
        .arg("build")
        .current_dir(output_dir)
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()?;

    if !status.success() {
        return Err("cargo build failed".into());
    }

    Ok(())
}
