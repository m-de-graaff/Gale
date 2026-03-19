//! Project template files for `gale new`.

use std::path::Path;

/// Options for project scaffolding.
pub struct ScaffoldOptions {
    pub name: String,
    pub tailwind: bool,
    pub template: TemplateChoice,
}

/// Template choice for project scaffolding.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TemplateChoice {
    Default,
    Ecommerce,
    ChatApp,
}

/// Generate the project directory structure and write all files.
pub fn generate_project(opts: &ScaffoldOptions) -> std::io::Result<()> {
    let root = Path::new(&opts.name);
    std::fs::create_dir_all(root.join("app"))?;
    std::fs::create_dir_all(root.join("public"))?;

    // Core config files
    std::fs::write(root.join("galex.toml"), galex_toml(opts))?;
    std::fs::write(root.join(".gitignore"), GITIGNORE)?;

    // Layout (shared across all templates)
    std::fs::write(root.join("app").join("layout.gx"), LAYOUT_TEMPLATE)?;

    // Template-specific pages
    match opts.template {
        TemplateChoice::Default => write_default_template(root, &opts.name)?,
        TemplateChoice::Ecommerce => write_ecommerce_template(root, &opts.name)?,
        TemplateChoice::ChatApp => write_chat_template(root, &opts.name)?,
    }

    // Tailwind setup
    if opts.tailwind {
        std::fs::create_dir_all(root.join("styles"))?;
        std::fs::write(root.join("styles").join("global.css"), GLOBAL_CSS)?;
        std::fs::write(root.join("package.json"), package_json(&opts.name))?;
    }

    // public/favicon.ico (placeholder)
    std::fs::write(root.join("public").join("favicon.ico"), b"")?;

    Ok(())
}

// ── Template writers ───────────────────────────────────────────────────

fn write_default_template(root: &Path, name: &str) -> std::io::Result<()> {
    // / — Landing page with interactive counter
    std::fs::write(root.join("app").join("page.gx"), default_index(name))?;
    // /server — Server action example
    std::fs::create_dir_all(root.join("app").join("server"))?;
    std::fs::write(
        root.join("app").join("server").join("page.gx"),
        DEFAULT_SERVER_PAGE,
    )?;
    Ok(())
}

fn write_ecommerce_template(root: &Path, name: &str) -> std::io::Result<()> {
    // / — Product catalog
    std::fs::write(root.join("app").join("page.gx"), ecommerce_index(name))?;
    // /cart — Shopping cart
    std::fs::create_dir_all(root.join("app").join("cart"))?;
    std::fs::write(
        root.join("app").join("cart").join("page.gx"),
        ECOMMERCE_CART_PAGE,
    )?;
    Ok(())
}

fn write_chat_template(root: &Path, name: &str) -> std::io::Result<()> {
    // / — Chat room
    std::fs::write(root.join("app").join("page.gx"), chat_index(name))?;
    Ok(())
}

// ── Config ─────────────────────────────────────────────────────────────

fn galex_toml(opts: &ScaffoldOptions) -> String {
    let mut toml = format!(
        r#"# GaleX project configuration
# Documentation: https://get-gale.vercel.app/docs/config

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
input_css = "styles/global.css"
"#,
        );
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

// ── Shared layout ──────────────────────────────────────────────────────

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

// ── Default template ───────────────────────────────────────────────────

fn default_index(name: &str) -> String {
    format!(
        r#"out ui HomePage {{
  head {{
    title: "{name}"
  }}

  signal count = 0

  <main class="min-h-screen bg-gradient-to-b from-gray-950 to-gray-900 text-white flex items-center justify-center">
    <div class="text-center space-y-8 px-4">
      <div class="space-y-2">
        <h1 class="text-5xl font-bold tracking-tight bg-gradient-to-r from-blue-400 to-violet-400 bg-clip-text text-transparent">
          {name}
        </h1>
        <p class="text-gray-400 text-lg">
          Built with Gale
        </p>
      </div>

      <div class="bg-gray-800/50 rounded-2xl p-8 border border-gray-700/50 backdrop-blur-sm space-y-4">
        <p class="text-gray-300 text-sm uppercase tracking-wider">Counter</p>
        <p class="text-6xl font-mono font-bold tabular-nums">{{count}}</p>
        <div class="flex gap-3 justify-center">
          <button
            on:click={{count = count - 1}}
            class="px-5 py-2.5 bg-gray-700 hover:bg-gray-600 rounded-lg font-medium transition-colors"
          >
            -
          </button>
          <button
            on:click={{count = count + 1}}
            class="px-5 py-2.5 bg-blue-600 hover:bg-blue-500 rounded-lg font-medium transition-colors"
          >
            +
          </button>
        </div>
      </div>

      <div class="flex gap-4 justify-center text-sm">
        <a href="/server" class="text-blue-400 hover:text-blue-300 underline underline-offset-4">
          Server Example
        </a>
        <a href="https://get-gale.vercel.app/docs" class="text-gray-500 hover:text-gray-400 underline underline-offset-4">
          Documentation
        </a>
      </div>
    </div>
  </main>
}}
"#
    )
}

const DEFAULT_SERVER_PAGE: &str = r#"out ui ServerPage {
  head {
    title: "Server Example"
  }

  <main class="min-h-screen bg-gradient-to-b from-gray-950 to-gray-900 text-white flex items-center justify-center">
    <div class="text-center space-y-8 px-4 max-w-lg">
      <div class="space-y-2">
        <h1 class="text-4xl font-bold tracking-tight">Server Rendering</h1>
        <p class="text-gray-400">
          This page is server-side rendered. The HTML is generated on the
          server and sent to the browser as a complete document.
        </p>
      </div>

      <div class="bg-gray-800/50 rounded-2xl p-8 border border-gray-700/50 space-y-4">
        <p class="text-gray-300 text-sm uppercase tracking-wider">Server Info</p>
        <div class="text-left space-y-2 text-sm">
          <div class="flex justify-between">
            <span class="text-gray-400">Runtime</span>
            <span class="text-white font-mono">Gale (Rust/Axum)</span>
          </div>
          <div class="flex justify-between">
            <span class="text-gray-400">Rendering</span>
            <span class="text-white font-mono">SSR</span>
          </div>
        </div>
      </div>

      <a href="/" class="inline-block text-blue-400 hover:text-blue-300 underline underline-offset-4 text-sm">
        Back home
      </a>
    </div>
  </main>
}
"#;

// ── E-commerce template ────────────────────────────────────────────────

fn ecommerce_index(name: &str) -> String {
    format!(
        r#"out ui HomePage {{
  head {{
    title: "{name}"
  }}

  <main class="min-h-screen bg-gray-50">
    <header class="bg-white border-b border-gray-200 px-6 py-4">
      <div class="max-w-6xl mx-auto flex items-center justify-between">
        <h1 class="text-xl font-bold text-gray-900">{name}</h1>
        <a href="/cart" class="text-gray-600 hover:text-gray-900">Cart (0)</a>
      </div>
    </header>

    <div class="max-w-6xl mx-auto px-6 py-8">
      <h2 class="text-2xl font-bold text-gray-900 mb-6">Products</h2>
      <div class="grid grid-cols-1 md:grid-cols-3 gap-6">

        <div class="bg-white rounded-xl border border-gray-200 overflow-hidden hover:shadow-lg transition-shadow">
          <div class="h-48 bg-gradient-to-br from-blue-100 to-blue-200"></div>
          <div class="p-4 space-y-2">
            <h3 class="font-semibold text-gray-900">Minimal Desk Lamp</h3>
            <p class="text-sm text-gray-500">Clean design, warm light</p>
            <div class="flex items-center justify-between pt-2">
              <span class="text-lg font-bold text-gray-900">$49</span>
              <button class="px-4 py-2 bg-gray-900 text-white text-sm rounded-lg hover:bg-gray-800 transition-colors">
                Add to Cart
              </button>
            </div>
          </div>
        </div>

        <div class="bg-white rounded-xl border border-gray-200 overflow-hidden hover:shadow-lg transition-shadow">
          <div class="h-48 bg-gradient-to-br from-amber-100 to-amber-200"></div>
          <div class="p-4 space-y-2">
            <h3 class="font-semibold text-gray-900">Ceramic Mug Set</h3>
            <p class="text-sm text-gray-500">Handcrafted, set of 4</p>
            <div class="flex items-center justify-between pt-2">
              <span class="text-lg font-bold text-gray-900">$32</span>
              <button class="px-4 py-2 bg-gray-900 text-white text-sm rounded-lg hover:bg-gray-800 transition-colors">
                Add to Cart
              </button>
            </div>
          </div>
        </div>

        <div class="bg-white rounded-xl border border-gray-200 overflow-hidden hover:shadow-lg transition-shadow">
          <div class="h-48 bg-gradient-to-br from-emerald-100 to-emerald-200"></div>
          <div class="p-4 space-y-2">
            <h3 class="font-semibold text-gray-900">Notebook & Pen</h3>
            <p class="text-sm text-gray-500">Premium leather bound</p>
            <div class="flex items-center justify-between pt-2">
              <span class="text-lg font-bold text-gray-900">$28</span>
              <button class="px-4 py-2 bg-gray-900 text-white text-sm rounded-lg hover:bg-gray-800 transition-colors">
                Add to Cart
              </button>
            </div>
          </div>
        </div>

      </div>
    </div>
  </main>
}}
"#
    )
}

const ECOMMERCE_CART_PAGE: &str = r#"out ui CartPage {
  head {
    title: "Shopping Cart"
  }

  <main class="min-h-screen bg-gray-50">
    <header class="bg-white border-b border-gray-200 px-6 py-4">
      <div class="max-w-6xl mx-auto flex items-center justify-between">
        <a href="/" class="text-xl font-bold text-gray-900">Store</a>
        <span class="text-gray-600">Cart</span>
      </div>
    </header>

    <div class="max-w-2xl mx-auto px-6 py-8">
      <h2 class="text-2xl font-bold text-gray-900 mb-6">Your Cart</h2>

      <div class="bg-white rounded-xl border border-gray-200 p-8 text-center">
        <p class="text-gray-500 mb-4">Your cart is empty</p>
        <a href="/" class="inline-block px-6 py-2.5 bg-gray-900 text-white text-sm rounded-lg hover:bg-gray-800 transition-colors">
          Continue Shopping
        </a>
      </div>
    </div>
  </main>
}
"#;

// ── Chat App template ──────────────────────────────────────────────────

fn chat_index(name: &str) -> String {
    format!(
        r#"out ui ChatPage {{
  head {{
    title: "{name}"
  }}

  signal message = ""

  <main class="min-h-screen bg-gray-950 text-white flex flex-col">
    <header class="bg-gray-900 border-b border-gray-800 px-6 py-4">
      <h1 class="text-lg font-semibold">{name}</h1>
    </header>

    <div class="flex-1 overflow-y-auto p-6 space-y-4">
      <div class="flex gap-3">
        <div class="w-8 h-8 rounded-full bg-blue-600 flex-shrink-0 flex items-center justify-center text-xs font-bold">
          G
        </div>
        <div class="bg-gray-800 rounded-2xl rounded-tl-sm px-4 py-2.5 max-w-md">
          <p class="text-sm">Welcome to the chat! This template uses GaleX channels for real-time messaging.</p>
        </div>
      </div>

      <div class="flex gap-3">
        <div class="w-8 h-8 rounded-full bg-violet-600 flex-shrink-0 flex items-center justify-center text-xs font-bold">
          A
        </div>
        <div class="bg-gray-800 rounded-2xl rounded-tl-sm px-4 py-2.5 max-w-md">
          <p class="text-sm">Connect a channel to enable live messaging between clients.</p>
        </div>
      </div>
    </div>

    <div class="border-t border-gray-800 p-4">
      <div class="flex gap-3 max-w-4xl mx-auto">
        <input
          type="text"
          bind:value={{message}}
          placeholder="Type a message..."
          class="flex-1 bg-gray-800 border border-gray-700 rounded-xl px-4 py-3 text-sm text-white placeholder-gray-500 focus:outline-none focus:border-blue-500 transition-colors"
        />
        <button class="px-6 py-3 bg-blue-600 hover:bg-blue-500 rounded-xl text-sm font-medium transition-colors">
          Send
        </button>
      </div>
    </div>
  </main>
}}
"#
    )
}

// ── Shared files ───────────────────────────────────────────────────────

fn package_json(name: &str) -> String {
    format!(
        r#"{{
  "name": "{name}",
  "private": true,
  "devDependencies": {{
    "@tailwindcss/cli": "^4"
  }}
}}
"#
    )
}

const GLOBAL_CSS: &str = r#"@import "tailwindcss";
@source "../app/**/*.gx";
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
            template: TemplateChoice::Default,
        };
        let toml = galex_toml(&opts);
        assert!(toml.contains("name = \"my-app\""));
        assert!(toml.contains("[tailwind]"));
    }

    #[test]
    fn galex_toml_no_tailwind() {
        let opts = ScaffoldOptions {
            name: "test".into(),
            tailwind: false,
            template: TemplateChoice::Default,
        };
        let toml = galex_toml(&opts);
        assert!(!toml.contains("[tailwind]"));
    }

    #[test]
    fn default_template_has_counter() {
        let page = default_index("test");
        assert!(page.contains("signal count = 0"));
        assert!(page.contains("on:click"));
    }
}
