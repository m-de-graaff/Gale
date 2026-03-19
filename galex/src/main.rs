//! GaleX CLI — the `gale` command.
//!
//! Commands:
//! - `gale build` — compile `.gx` files into a production-ready server
//! - `gale serve` — run the production build
//! - `gale dev` — development server with hot reload
//! - `gale check` — type-check without code generation
//! - `gale new` — create a new project
//! - `gale fmt` — format .gx source files
//! - `gale lint` — static analysis
//! - `gale test` — run test blocks
//! - `gale add` — add a package from the registry

use std::path::{Path, PathBuf};
use std::process;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "gale",
    version,
    about = "GaleX compiler — build type-safe, reactive web applications"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Compile .gx source files into a production-ready server binary
    Build {
        #[arg(long, default_value = "app")]
        app_dir: PathBuf,
        #[arg(long, default_value = "gale_build")]
        output_dir: PathBuf,
        #[arg(long, default_value = "gale_app")]
        name: String,
        #[arg(long)]
        release: bool,
        /// Generate a Dockerfile in the dist/ output
        #[arg(long)]
        docker: bool,
    },
    /// Run the production build from dist/
    Serve {
        /// Path to the dist directory
        #[arg(long, default_value = "dist")]
        dist_dir: PathBuf,
        /// Override the server port
        #[arg(short, long)]
        port: Option<u16>,
    },
    /// Start development server with hot reload
    Dev {
        #[arg(long, default_value = "app")]
        app_dir: PathBuf,
        #[arg(short, long, default_value = "3000")]
        port: u16,
    },
    /// Type-check .gx files without code generation
    Check {
        #[arg(long, default_value = "app")]
        app_dir: PathBuf,
    },
    /// Create a new GaleX project
    New {
        /// Project name (interactive prompt if omitted)
        name: Option<String>,
    },
    /// Format .gx source files
    Fmt {
        #[arg(long, default_value = "app")]
        app_dir: PathBuf,
        /// Check formatting without writing changes (CI mode)
        #[arg(long)]
        check: bool,
    },
    /// Run static analysis lints
    Lint {
        #[arg(long, default_value = "app")]
        app_dir: PathBuf,
    },
    /// Run test blocks from .gx files
    Test {
        #[arg(long, default_value = "app")]
        app_dir: PathBuf,
        /// Filter tests by name
        #[arg(long)]
        filter: Option<String>,
    },
    /// Add a package from the registry
    Add {
        /// Package name (e.g. "db/postgres")
        package: String,
    },
    /// Remove a package
    Remove {
        /// Package name to remove
        package: String,
    },
    /// Update packages to latest matching versions
    Update {
        /// Specific package to update (all if omitted)
        package: Option<String>,
    },
    /// Search the package registry
    Search {
        /// Search query
        query: String,
    },
    /// Publish a package to the registry
    Publish,
    /// Authenticate with the package registry
    Login,
    /// Download and install the latest version of gale
    SelfUpdate,
    /// Upgrade project to the current GaleX version
    Migrate,
    /// Install or manage editor extensions
    Editor {
        #[command(subcommand)]
        command: EditorCommand,
    },
}

#[derive(Subcommand)]
enum EditorCommand {
    /// Download and install an editor extension
    Install {
        /// Editor to install extension for: vscode, zed
        editor: String,
    },
}

fn main() {
    let cli = Cli::parse();
    let exit_code = match cli.command {
        Command::Build {
            app_dir,
            output_dir,
            name,
            release,
            docker,
        } => {
            cmd_build(&app_dir, &output_dir, &name, release, docker);
            0
        }
        Command::Serve { dist_dir, port } => {
            cmd_serve(&dist_dir, port);
            0
        }
        Command::Dev { app_dir, port } => {
            cmd_dev(&app_dir, port);
            0
        }
        Command::Check { app_dir } => galex::commands::check::run(&app_dir),
        Command::New { name } => galex::commands::new::run(name),
        Command::Fmt { app_dir, check } => galex::commands::fmt::run(&app_dir, check),
        Command::Lint { app_dir } => galex::commands::lint_cmd::run(&app_dir),
        Command::Test { app_dir, filter } => {
            galex::commands::test::run(&app_dir, filter.as_deref())
        }
        Command::Add { package } => galex::commands::add::run(&package),
        Command::Remove { package } => galex::commands::remove::run(&package),
        Command::Update { package } => galex::commands::update::run(package.as_deref()),
        Command::Search { query } => galex::commands::search::run(&query),
        Command::Publish => galex::commands::publish::run(),
        Command::Login => galex::commands::login::run(),
        Command::SelfUpdate => galex::commands::self_update::run(),
        Command::Migrate => galex::commands::migrate::run(),
        Command::Editor { command } => match command {
            EditorCommand::Install { editor } => galex::commands::editor::run_install(&editor),
        },
    };
    process::exit(exit_code);
}

/// `gale build` — full compilation pipeline.
///
/// Steps:
/// 1. Discover routes
/// 2. Parse .gx files
/// 3. Type check
/// 4. Generate project (Rust + JS/CSS)
/// 5. Copy public/ assets
/// 6. Build CSS (Tailwind)
/// 7. Optimize assets (minify JS, hash, manifest) — release only
/// 8. Build with cargo
/// 9. Assemble dist/ — release only
fn cmd_build(
    app_dir: &PathBuf,
    output_dir: &PathBuf,
    project_name: &str,
    release: bool,
    docker: bool,
) {
    let total_steps = if release { 9 } else { 8 };

    // ── Step 1: Discover routes ────────────────────────────────
    eprintln!(
        "[1/{total_steps}] Discovering routes in {}...",
        app_dir.display()
    );
    let routes = match galex::router::discovery::discover_routes(app_dir) {
        Ok(routes) => routes,
        Err(errors) => {
            for err in &errors {
                eprintln!("  error: {err}");
            }
            process::exit(1);
        }
    };
    eprintln!("  Found {} route(s)", routes.len());
    for route in &routes {
        eprintln!("    {} -> {}", route.url_path, route.page_file.display());
    }

    // ── Step 2: Parse .gx files ────────────────────────────────
    eprintln!("[2/{total_steps}] Parsing .gx files...");
    let mut compiler = galex::compiler::Compiler::new();

    for route in &routes {
        let files = std::iter::once(&route.page_file)
            .chain(route.layouts.iter())
            .chain(route.guards.iter())
            .chain(route.middleware.iter());

        for path in files {
            if let Err(e) = compiler.add_file_dedup(path) {
                eprintln!("  error reading {}: {e}", path.display());
                process::exit(1);
            }
        }
    }

    let error_count = compiler.parse_all();
    if error_count > 0 {
        eprintln!("  {} parse error(s):", error_count);
        for err in &compiler.parse_errors {
            eprintln!("    {err}");
        }
        process::exit(1);
    }
    eprintln!("  Parsed {} file(s) successfully", compiler.sources.len());

    // ── Step 3: Type check ─────────────────────────────────────
    eprintln!("[3/{total_steps}] Type checking...");
    let type_errors = compiler.check();
    if !type_errors.is_empty() {
        eprintln!("  {} type error(s):", type_errors.len());
        for err in &type_errors {
            eprintln!("    {err}");
        }
        process::exit(1);
    }
    eprintln!("  No type errors");

    // ── Step 4: Code generation ────────────────────────────────
    eprintln!(
        "[4/{total_steps}] Generating project in {}...",
        output_dir.display()
    );
    compiler.set_routes(routes);
    // Compute the gale crate path relative to the output directory.
    // Count how many directory components deep the output_dir is from CWD,
    // then go up that many levels.
    let gale_crate_path = {
        let depth = output_dir.components().count();
        let mut rel = String::new();
        for _ in 0..depth {
            rel.push_str("../");
        }
        if rel.is_empty() {
            "./".to_string()
        } else {
            rel
        }
    };
    if let Err(e) = compiler.generate(project_name, output_dir, Some(&gale_crate_path), false) {
        eprintln!("  error: {e}");
        process::exit(1);
    }
    eprintln!("  Code generation complete");

    // ── Step 5: Copy public/ assets ────────────────────────────
    eprintln!("[5/{total_steps}] Copying public assets...");
    let project_dir = app_dir.parent().unwrap_or(app_dir);
    let user_public = project_dir.join("public");
    let dest_public = output_dir.join("public");
    if user_public.is_dir() {
        match copy_public_assets(&user_public, &dest_public) {
            Ok(count) => eprintln!("  Copied {count} file(s) from public/"),
            Err(e) => eprintln!("  warning: failed to copy public/: {e}"),
        }
    } else {
        eprintln!("  No public/ directory found (skipped)");
    }

    // ── Step 6: Build CSS (Tailwind) ───────────────────────────
    eprintln!("[6/{total_steps}] Building CSS...");
    match compiler.generate_css(project_dir, app_dir, output_dir, release) {
        Ok(true) => eprintln!("  Tailwind CSS generated"),
        Ok(false) => eprintln!("  Tailwind CSS disabled (no [tailwind] in galex.toml)"),
        Err(e) => {
            eprintln!("  warning: CSS generation failed: {e}");
            eprintln!("  (continuing without Tailwind CSS)");
        }
    }

    // ── Step 7: Optimize assets (release only) ─────────────────
    if release {
        eprintln!("[7/{total_steps}] Optimizing assets...");
        optimize_assets(output_dir);
    } else {
        eprintln!("[7/{total_steps}] Skipping asset optimization (debug build)");
    }

    // ── Step 8: Build with cargo ───────────────────────────────
    eprintln!("[8/{total_steps}] Building with cargo...");
    let mut cmd = process::Command::new("cargo");
    cmd.arg("build");
    if release {
        cmd.arg("--release");
    }
    cmd.current_dir(output_dir);
    match cmd.status() {
        Ok(status) if status.success() => {
            eprintln!("  Build successful!");
        }
        Ok(status) => {
            eprintln!("  cargo build failed with status: {status}");
            process::exit(1);
        }
        Err(e) => {
            eprintln!("  Failed to run cargo: {e}");
            process::exit(1);
        }
    }

    // ── Step 9: Assemble dist/ (release only) ──────────────────
    if release {
        eprintln!("[9/{total_steps}] Assembling dist/...");
        assemble_dist(output_dir, project_name, docker);
    } else {
        let binary = "target/debug";
        eprintln!("  Binary: {}/{binary}", output_dir.display());
    }
}

/// Minify JS assets and generate content-hashed filenames + manifest.
fn optimize_assets(output_dir: &Path) {
    use galex::codegen::minify;

    // Read all JS files from the output public/_gale/ directory and minify them
    let gale_dir = output_dir.join("public/_gale");
    if gale_dir.is_dir() {
        let mut minified_count = 0u32;
        let mut total_saved = 0usize;
        if let Ok(entries) = walk_js_files(&gale_dir) {
            for path in entries {
                if let Ok(source) = std::fs::read_to_string(&path) {
                    let original_len = source.len();
                    let minified = minify::minify_js_production(&source);
                    let saved = original_len.saturating_sub(minified.len());
                    total_saved += saved;
                    if let Err(e) = std::fs::write(&path, &minified) {
                        eprintln!(
                            "  warning: failed to write minified {}: {e}",
                            path.display()
                        );
                    } else {
                        minified_count += 1;
                    }
                }
            }
        }
        if minified_count > 0 {
            eprintln!(
                "  Minified {minified_count} JS file(s) (saved {:.1} KB)",
                total_saved as f64 / 1024.0
            );
        }
    }

    // Hash all framework assets and generate the manifest
    let public_dir = output_dir.join("public/_gale");
    if public_dir.is_dir() {
        let manifest = hash_assets_on_disk(output_dir);
        let manifest_count = manifest.len();

        // Write the asset_manifest.rs with real hashed entries
        let manifest_rs = manifest.generate_rust_module();
        let manifest_path = output_dir.join("src/asset_manifest.rs");
        if let Err(e) = std::fs::write(&manifest_path, &manifest_rs) {
            eprintln!("  warning: failed to write asset manifest: {e}");
        }

        // Also write a JSON manifest for debugging
        let json_path = output_dir.join("public/_gale/manifest.json");
        let _ = std::fs::write(&json_path, manifest.to_json());

        if manifest_count > 0 {
            eprintln!("  Hashed {manifest_count} asset(s) for cache busting");
        }
    }
}

/// Copy user public assets to the build output, skipping dotfiles
/// and avoiding overwriting framework files in `_gale/`.
fn copy_public_assets(src: &Path, dst: &Path) -> std::io::Result<usize> {
    let mut count = 0;
    copy_public_recursive(src, dst, &mut count)?;
    Ok(count)
}

fn copy_public_recursive(src: &Path, dst: &Path, count: &mut usize) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Skip dotfiles and framework directory
        if name_str.starts_with('.') || name_str == "_gale" {
            continue;
        }

        let src_path = entry.path();
        let dst_path = dst.join(&name);

        if src_path.is_dir() {
            copy_public_recursive(&src_path, &dst_path, count)?;
        } else if src_path.is_file() {
            std::fs::copy(&src_path, &dst_path)?;
            *count += 1;
        }
    }
    Ok(())
}

/// Walk a directory tree and collect all .js file paths.
fn walk_js_files(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut result = Vec::new();
    walk_js_files_recursive(dir, &mut result)?;
    Ok(result)
}

fn walk_js_files_recursive(dir: &Path, result: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walk_js_files_recursive(&path, result)?;
        } else if path.extension().is_some_and(|ext| ext == "js") {
            result.push(path);
        }
    }
    Ok(())
}

/// Hash assets on disk and rename files in place.
///
/// Returns an `AssetManifest` mapping logical paths to hashed filenames.
fn hash_assets_on_disk(output_dir: &Path) -> galex::codegen::asset_hash::AssetManifest {
    use galex::codegen::asset_hash::AssetManifest;

    let mut manifest = AssetManifest::new();
    let public_dir = output_dir.join("public");
    let gale_dir = public_dir.join("_gale");

    if !gale_dir.is_dir() {
        return manifest;
    }

    hash_dir_recursive(&gale_dir, &public_dir, &mut manifest);
    manifest
}

fn hash_dir_recursive(
    dir: &Path,
    public_root: &Path,
    manifest: &mut galex::codegen::asset_hash::AssetManifest,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            // Skip manifest.json itself
            hash_dir_recursive(&path, public_root, manifest);
        } else if path
            .extension()
            .is_some_and(|ext| ext == "js" || ext == "css")
        {
            if let Ok(content) = std::fs::read(&path) {
                let hash = galex::codegen::asset_hash::hash_content(&content);
                let filename = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                let hashed_filename = galex::codegen::asset_hash::insert_hash(&filename, &hash);

                // Logical path relative to public/
                let logical = path
                    .strip_prefix(public_root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");

                let hashed_logical = if let Some(parent) = Path::new(&logical).parent() {
                    let parent_str = parent.to_string_lossy().replace('\\', "/");
                    format!("{parent_str}/{hashed_filename}")
                } else {
                    hashed_filename.clone()
                };

                manifest.insert(logical, hashed_logical);

                // Rename the file on disk
                let hashed_path = path.with_file_name(&hashed_filename);
                if let Err(e) = std::fs::rename(&path, &hashed_path) {
                    eprintln!(
                        "  warning: failed to rename {} -> {}: {e}",
                        path.display(),
                        hashed_path.display()
                    );
                }
            }
        }
    }
}

/// Assemble the `dist/` output directory.
///
/// - Copy the release binary
/// - Copy the public/ assets (already hashed)
/// - Optionally generate a Dockerfile
fn assemble_dist(output_dir: &Path, project_name: &str, docker: bool) {
    let dist_dir = Path::new("dist");

    // Clean previous dist/
    if dist_dir.exists() {
        let _ = std::fs::remove_dir_all(dist_dir);
    }
    if let Err(e) = std::fs::create_dir_all(dist_dir) {
        eprintln!("  error: failed to create dist/: {e}");
        process::exit(1);
    }

    // Copy binary
    let binary_name = if cfg!(windows) {
        format!("{project_name}.exe")
    } else {
        project_name.to_string()
    };
    let binary_src = output_dir.join("target/release").join(&binary_name);
    let binary_dst = dist_dir.join(&binary_name);

    if binary_src.exists() {
        match std::fs::copy(&binary_src, &binary_dst) {
            Ok(size) => {
                eprintln!(
                    "  Binary: {} ({:.1} MB)",
                    binary_dst.display(),
                    size as f64 / (1024.0 * 1024.0)
                );
            }
            Err(e) => {
                eprintln!("  error: failed to copy binary: {e}");
                process::exit(1);
            }
        }
    } else {
        eprintln!(
            "  warning: release binary not found at {}",
            binary_src.display()
        );
    }

    // Copy public/ directory to dist/public/
    let public_src = output_dir.join("public");
    let public_dst = dist_dir.join("public");
    if public_src.is_dir() {
        match copy_dir_all(&public_src, &public_dst) {
            Ok(count) => {
                eprintln!("  Assets: {} ({count} files)", public_dst.display());
            }
            Err(e) => {
                eprintln!("  warning: failed to copy public/: {e}");
            }
        }
    }

    // Generate Dockerfile if requested
    if docker {
        generate_dockerfile(dist_dir, project_name);
    }

    // Generate a health check endpoint reminder
    eprintln!();
    eprintln!("  Build complete! To run:");
    if cfg!(windows) {
        eprintln!("    .\\dist\\{binary_name} --root .\\dist\\public --port 8080");
    } else {
        eprintln!("    ./dist/{binary_name} --root ./dist/public --port 8080");
    }
    if docker {
        eprintln!();
        eprintln!("  Docker:");
        eprintln!("    docker build -t {project_name} dist/");
        eprintln!("    docker run -p 8080:8080 {project_name}");
    }
}

/// Recursively copy a directory tree. Returns file count.
fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<usize> {
    let mut count = 0;
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            count += copy_dir_all(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
            count += 1;
        }
    }
    Ok(count)
}

/// Generate a minimal Dockerfile for the production build.
fn generate_dockerfile(dist_dir: &Path, project_name: &str) {
    let binary_name = project_name;
    let dockerfile = format!(
        r#"# Generated by GaleX — production Dockerfile
# Build: docker build -t {project_name} .
# Run:   docker run -p 8080:8080 {project_name}

FROM scratch

COPY {binary_name} /{binary_name}
COPY public/ /public/

EXPOSE 8080

LABEL org.opencontainers.image.title="{project_name}"

ENTRYPOINT ["/{binary_name}"]
CMD ["--root", "/public", "--port", "8080"]
"#
    );

    let path = dist_dir.join("Dockerfile");
    if let Err(e) = std::fs::write(&path, &dockerfile) {
        eprintln!("  warning: failed to write Dockerfile: {e}");
    } else {
        eprintln!("  Dockerfile: {}", path.display());
    }

    // Also generate .dockerignore
    let dockerignore = "*.md\n.git\n.gitignore\n";
    let _ = std::fs::write(dist_dir.join(".dockerignore"), dockerignore);
}

/// `gale serve` — run the production build from dist/.
fn cmd_serve(dist_dir: &PathBuf, port: Option<u16>) {
    if !dist_dir.exists() {
        eprintln!("error: dist directory not found at {}", dist_dir.display());
        eprintln!("  Run `gale build --release` first.");
        process::exit(1);
    }

    // Find the binary in dist/
    let binary = find_binary_in_dist(dist_dir);
    let binary = match binary {
        Some(b) => b,
        None => {
            eprintln!("error: no executable found in {}", dist_dir.display());
            process::exit(1);
        }
    };

    eprintln!("Starting server: {}", binary.display());

    let mut cmd = process::Command::new(&binary);
    cmd.arg("--root").arg(dist_dir.join("public"));
    if let Some(p) = port {
        cmd.arg("--port").arg(p.to_string());
    }

    // Forward stdio
    cmd.stdin(process::Stdio::inherit())
        .stdout(process::Stdio::inherit())
        .stderr(process::Stdio::inherit());

    match cmd.status() {
        Ok(status) => {
            if !status.success() {
                process::exit(status.code().unwrap_or(1));
            }
        }
        Err(e) => {
            eprintln!("error: failed to start server: {e}");
            process::exit(1);
        }
    }
}

/// Find an executable file in the dist directory.
fn find_binary_in_dist(dist_dir: &Path) -> Option<PathBuf> {
    let entries = std::fs::read_dir(dist_dir).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = path.file_name()?.to_string_lossy().to_string();
        // Skip non-binary files
        if name == "Dockerfile"
            || name == ".dockerignore"
            || name.ends_with(".json")
            || name.ends_with(".md")
        {
            continue;
        }
        // On Windows, look for .exe
        if cfg!(windows) {
            if name.ends_with(".exe") {
                return Some(path);
            }
        } else {
            // On Unix, check if it lacks a common non-binary extension
            if !name.contains('.') {
                return Some(path);
            }
        }
    }
    None
}

/// `gale dev` — development server with hot reload.
fn cmd_dev(app_dir: &PathBuf, port: u16) {
    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    rt.block_on(async {
        if let Err(e) = galex::dev::run_dev_server(app_dir, port).await {
            eprintln!("error: {e}");
            process::exit(1);
        }
    });
}
