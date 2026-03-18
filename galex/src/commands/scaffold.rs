//! Project template files for `gale new`.

use std::path::Path;

/// Options for project scaffolding.
pub struct ScaffoldOptions {
    pub name: String,
    pub tailwind: bool,
    pub example: bool,
    pub db: DbChoice,
    pub auth: AuthChoice,
}

/// Database adapter choice.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DbChoice {
    None,
    Postgres,
    Sqlite,
}

/// Authentication choice.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AuthChoice {
    None,
    Session,
    Jwt,
}

/// Generate the project directory structure and write all files.
pub fn generate_project(opts: &ScaffoldOptions) -> Result<(), std::io::Error> {
    let root = Path::new(&opts.name);
    std::fs::create_dir_all(root.join("app").join("about"))?;
    std::fs::create_dir_all(root.join("public"))?;

    // galex.toml
    std::fs::write(root.join("galex.toml"), galex_toml(opts))?;

    // .gitignore
    std::fs::write(root.join(".gitignore"), GITIGNORE)?;

    // app/layout.gx
    std::fs::write(root.join("app").join("layout.gx"), LAYOUT_TEMPLATE)?;

    // app/page.gx
    std::fs::write(root.join("app").join("page.gx"), index_page(&opts.name))?;

    // app/about/page.gx (if example)
    if opts.example {
        std::fs::write(root.join("app").join("about").join("page.gx"), ABOUT_PAGE)?;
    }

    // styles/global.css (if Tailwind)
    if opts.tailwind {
        std::fs::create_dir_all(root.join("styles"))?;
        std::fs::write(root.join("styles").join("global.css"), GLOBAL_CSS)?;
    }

    // public/favicon.ico (placeholder)
    std::fs::write(root.join("public").join("favicon.ico"), b"")?;

    Ok(())
}

/// Generate galex.toml content.
fn galex_toml(opts: &ScaffoldOptions) -> String {
    let mut toml = format!(
        r#"# GaleX project configuration
# Documentation: https://gale.dev/docs/config

[project]
name = "{}"
"#,
        opts.name
    );

    if opts.tailwind {
        toml.push_str(
            r#"
[tailwind]
enabled = true
"#,
        );
    }

    if opts.db != DbChoice::None {
        let adapter = match opts.db {
            DbChoice::Postgres => "postgres",
            DbChoice::Sqlite => "sqlite",
            DbChoice::None => unreachable!(),
        };
        toml.push_str(&format!(
            r#"
[database]
adapter = "{adapter}"
"#,
        ));
    }

    if opts.auth != AuthChoice::None {
        let strategy = match opts.auth {
            AuthChoice::Session => "session",
            AuthChoice::Jwt => "jwt",
            AuthChoice::None => unreachable!(),
        };
        toml.push_str(&format!(
            r#"
[auth]
strategy = "{strategy}"
"#,
        ));
    }

    toml
}

const GITIGNORE: &str = r#"# Build output
gale_build/
.gale_dev/
target/

# Dependencies
node_modules/
gale_modules/

# Environment
.env
.env.local
"#;

const LAYOUT_TEMPLATE: &str = r#"out layout Root {
  <html lang="en">
    <head>
      <meta charset="utf-8" />
      <meta name="viewport" content="width=device-width, initial-scale=1" />
    </head>
    <body>
      slot
    </body>
  </html>
}
"#;

fn index_page(project_name: &str) -> String {
    format!(
        r#"out ui HomePage {{
  head {{
    title: "{project_name}"
  }}

  <main class="flex items-center justify-center min-h-screen">
    <div class="text-center">
      <h1 class="text-4xl font-bold mb-4">{project_name}</h1>
      <p class="text-gray-600">Edit app/page.gx to get started.</p>
    </div>
  </main>
}}
"#
    )
}

const ABOUT_PAGE: &str = r#"out ui AboutPage {
  head {
    title: "About"
  }

  <main class="max-w-2xl mx-auto py-12 px-4">
    <h1 class="text-3xl font-bold mb-4">About</h1>
    <p class="text-gray-600">This is an example page.</p>
    <a href="/" class="text-blue-500 hover:underline mt-4 inline-block">Back home</a>
  </main>
}
"#;

const GLOBAL_CSS: &str = r#"@import "tailwindcss";
"#;

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn galex_toml_basic() {
        let opts = ScaffoldOptions {
            name: "my-app".into(),
            tailwind: true,
            example: true,
            db: DbChoice::None,
            auth: AuthChoice::None,
        };
        let toml = galex_toml(&opts);
        assert!(toml.contains("name = \"my-app\""));
        assert!(toml.contains("[tailwind]"));
        assert!(toml.contains("enabled = true"));
    }

    #[test]
    fn galex_toml_with_db_and_auth() {
        let opts = ScaffoldOptions {
            name: "app".into(),
            tailwind: false,
            example: false,
            db: DbChoice::Postgres,
            auth: AuthChoice::Jwt,
        };
        let toml = galex_toml(&opts);
        assert!(toml.contains("adapter = \"postgres\""));
        assert!(toml.contains("strategy = \"jwt\""));
        assert!(!toml.contains("[tailwind]"));
    }

    #[test]
    fn index_page_includes_name() {
        let page = index_page("My App");
        assert!(page.contains("My App"));
        assert!(page.contains("out ui HomePage"));
    }

    #[test]
    fn scaffold_creates_files() {
        let dir = std::env::temp_dir().join("gale_scaffold_test");
        let _ = std::fs::remove_dir_all(&dir);
        let opts = ScaffoldOptions {
            name: dir.display().to_string(),
            tailwind: true,
            example: true,
            db: DbChoice::None,
            auth: AuthChoice::None,
        };
        generate_project(&opts).unwrap();
        assert!(dir.join("galex.toml").exists());
        assert!(dir.join("app/layout.gx").exists());
        assert!(dir.join("app/page.gx").exists());
        assert!(dir.join("app/about/page.gx").exists());
        assert!(dir.join("styles/global.css").exists());
        assert!(dir.join(".gitignore").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
