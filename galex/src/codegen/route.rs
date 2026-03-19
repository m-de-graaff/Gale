//! Route handler generation — converts components to Axum handlers.
//!
//! Each exported component becomes a route handler that:
//! 1. Loads server data (from server block bindings)
//! 2. Renders head metadata
//! 3. Renders body HTML via SSR
//! 4. Wraps in the layout
//! 5. Returns `Html<String>`

use super::expr::expr_to_rust;
use super::head;
use super::hydration::HydrationCtx;
use super::rust_emitter::RustEmitter;
use super::ssr;
use crate::ast::*;
use crate::codegen::types::to_snake_case;

/// Emit a complete route handler module for a component.
///
/// Generates `src/routes/{name}.rs` with:
/// - `pub async fn handler(...)` — the Axum handler
/// - `fn render_head()` — head metadata builder
/// - `fn render_body(...)` — SSR template renderer
pub fn emit_route_module(decl: &ComponentDecl) -> String {
    let mut e = RustEmitter::new();
    e.emit_file_header(&format!("Route handler for component: `{}`.", decl.name));
    e.newline();

    e.emit_use("axum::response::Html");
    e.newline();

    // Detect server data bindings (let statements in component body)
    let server_bindings = collect_server_bindings(&decl.body.stmts);
    let path_params = extract_path_params(&decl.name);

    // --- Handler function ---
    emit_handler_fn(&mut e, decl, &server_bindings, &path_params);

    // --- JSON handler (for client-side navigation) ---
    emit_json_handler_fn(&mut e, decl, &server_bindings, &path_params);

    // --- Head renderer ---
    head::emit_head_fn(&mut e, decl.body.head.as_ref());

    // --- Body renderer ---
    emit_body_fn(&mut e, decl, &server_bindings);

    e.finish()
}

/// Extract path parameters from component name.
///
/// Convention: `UserById[id]` → `vec!["id"]`, `BlogPost[slug]` → `vec!["slug"]`.
fn extract_path_params(name: &str) -> Vec<String> {
    let mut params = Vec::new();
    let mut remaining = name;
    while let Some(start) = remaining.find('[') {
        if let Some(end) = remaining[start..].find(']') {
            let param = &remaining[start + 1..start + end];
            if !param.is_empty() {
                params.push(param.to_string());
            }
            remaining = &remaining[start + end + 1..];
        } else {
            break;
        }
    }
    params
}

/// Convert a component name to a URL route path.
///
/// - `HomePage` → `"/"`
/// - `About` → `"/about"`
/// - `UserProfile` → `"/user-profile"`
/// - `UserById[id]` → `"/user-by-id/:id"`
pub fn component_name_to_path(name: &str) -> String {
    // Strip parameters first
    let base_name = if let Some(bracket) = name.find('[') {
        &name[..bracket]
    } else {
        name
    };

    // Special case: "HomePage" or "Home" → "/"
    if base_name == "HomePage" || base_name == "Home" || base_name == "Index" {
        let params = extract_path_params(name);
        if params.is_empty() {
            return "/".to_string();
        }
    }

    // Convert PascalCase to kebab-case path segments
    let kebab = pascal_to_kebab(base_name);
    let mut path = format!("/{kebab}");

    // Append path parameters
    for param in extract_path_params(name) {
        path.push_str(&format!("/:{param}"));
    }

    path
}

/// Extract parameter names from a URL path pattern.
///
/// E.g. `/user/:id/posts/:postId` → `vec!["id", "postId"]`.
pub fn extract_route_params(path: &str) -> Vec<String> {
    path.split('/')
        .filter(|s| s.starts_with(':'))
        .map(|s| s[1..].to_string())
        .collect()
}

/// Convert PascalCase to kebab-case.
fn pascal_to_kebab(name: &str) -> String {
    let mut result = String::with_capacity(name.len() + 4);
    for (i, ch) in name.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 {
                result.push('-');
            }
            result.push(ch.to_lowercase().next().unwrap_or(ch));
        } else {
            result.push(ch);
        }
    }
    result
}

// ── Internal generators ────────────────────────────────────────────────

/// Collect `Let` and `Mut` bindings from component body statements.
///
/// These become the "server data" that is loaded before rendering.
fn collect_server_bindings(stmts: &[Stmt]) -> Vec<(String, Option<String>)> {
    let mut bindings = Vec::new();
    for stmt in stmts {
        match stmt {
            Stmt::Let { name, init, .. } | Stmt::Mut { name, init, .. } => {
                let rust_name = to_snake_case(name);
                let init_expr = expr_to_rust(init);
                bindings.push((rust_name, Some(init_expr)));
            }
            Stmt::Frozen { name, init, .. } => {
                let rust_name = to_snake_case(name);
                let init_expr = expr_to_rust(init);
                bindings.push((rust_name, Some(init_expr)));
            }
            // Signals are client-side reactive state, but SSR needs their
            // initial value so template expressions like {count} can render.
            Stmt::Signal { name, init, .. } => {
                let rust_name = to_snake_case(name);
                let init_expr = expr_to_rust(init);
                bindings.push((rust_name, Some(init_expr)));
            }
            _ => {} // Skip derives, refs, effects, etc.
        }
    }
    bindings
}

/// Emit the Axum handler function.
fn emit_handler_fn(
    e: &mut RustEmitter,
    decl: &ComponentDecl,
    server_bindings: &[(String, Option<String>)],
    path_params: &[String],
) {
    let mut params: Vec<(&str, &str)> = Vec::new();

    // If there are path parameters, add the Path extractor
    let path_type;
    let path_param_str;
    if !path_params.is_empty() {
        if path_params.len() == 1 {
            path_type = "axum::extract::Path<String>".to_string();
            path_param_str = format!("axum::extract::Path({})", path_params[0]);
        } else {
            let types = vec!["String"; path_params.len()].join(", ");
            path_type = format!("axum::extract::Path<({types})>");
            let names = path_params.join(", ");
            path_param_str = format!("axum::extract::Path(({names}))");
        }
        params.push((&path_param_str, &path_type));
    }

    e.emit_doc_comment(&format!("Axum route handler for `{}`.", decl.name));
    e.emit_fn("pub", true, "handler", &params, Some("Html<String>"), |e| {
        // Load server data
        for (name, init_expr) in server_bindings {
            if let Some(init) = init_expr {
                e.writeln(&format!("let {name} = {init};"));
            }
        }

        if !server_bindings.is_empty() {
            e.newline();
        }

        // Render head and body
        e.writeln("let head_html = render_head();");

        // Build render_body call with server data as arguments
        if server_bindings.is_empty() {
            e.writeln("let body_html = render_body();");
        } else {
            let args: Vec<String> = server_bindings
                .iter()
                .map(|(name, _)| format!("&{name}"))
                .collect();
            e.writeln(&format!(
                "let body_html = render_body({});",
                args.join(", ")
            ));
        }

        e.newline();
        e.writeln("Html(crate::layout::render(&head_html, &body_html))");
    });
    e.newline();
}

/// Emit a JSON endpoint handler for client-side navigation.
///
/// Returns `{ "html": "<rendered body>" }` for the router to swap into
/// the `[data-gale-slot]` region without a full page reload.
fn emit_json_handler_fn(
    e: &mut RustEmitter,
    _decl: &ComponentDecl,
    server_bindings: &[(String, Option<String>)],
    path_params: &[String],
) {
    let mut params: Vec<(&str, &str)> = Vec::new();

    let path_type;
    let path_param_str;
    if !path_params.is_empty() {
        if path_params.len() == 1 {
            path_type = "axum::extract::Path<String>".to_string();
            path_param_str = format!("axum::extract::Path({})", path_params[0]);
        } else {
            let types = vec!["String"; path_params.len()].join(", ");
            path_type = format!("axum::extract::Path<({types})>");
            let names = path_params.join(", ");
            path_param_str = format!("axum::extract::Path(({names}))");
        }
        params.push((&path_param_str, &path_type));
    }

    e.emit_doc_comment("JSON handler for client-side navigation.");
    e.emit_fn(
        "pub",
        true,
        "json_handler",
        &params,
        Some("axum::Json<serde_json::Value>"),
        |e| {
            // Load server data
            for (name, init_expr) in server_bindings {
                if let Some(init) = init_expr {
                    e.writeln(&format!("let {name} = {init};"));
                }
            }
            if !server_bindings.is_empty() {
                e.newline();
            }

            // Render body only (no layout wrapping)
            if server_bindings.is_empty() {
                e.writeln("let body_html = render_body();");
            } else {
                let args: Vec<String> = server_bindings
                    .iter()
                    .map(|(name, _)| format!("&{name}"))
                    .collect();
                e.writeln(&format!(
                    "let body_html = render_body({});",
                    args.join(", ")
                ));
            }

            e.newline();
            e.writeln("axum::Json(serde_json::json!({ \"html\": body_html }))");
        },
    );
    e.newline();
}

/// Emit the body renderer function.
fn emit_body_fn(
    e: &mut RustEmitter,
    decl: &ComponentDecl,
    server_bindings: &[(String, Option<String>)],
) {
    // Build parameter list from server bindings
    let param_strs: Vec<(String, String)> = server_bindings
        .iter()
        .map(|(name, _)| (name.clone(), "&serde_json::Value".to_string()))
        .collect();
    let params: Vec<(&str, &str)> = param_strs
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();

    e.emit_fn("", false, "render_body", &params, Some("String"), |e| {
        e.writeln("let mut html = String::with_capacity(2048);");

        let mut hydration = HydrationCtx::new();

        // Add server data keys for hydration
        for (name, _) in server_bindings {
            hydration.add_server_data(name);
        }

        ssr::emit_template_nodes(e, &decl.body.template, &mut hydration, None);

        // Emit hydration script if any interactive elements
        hydration.emit_script(e);

        // Embed PUBLIC_ env vars for client-side access (if env block declared)
        let env_keys = crate::codegen::expr::get_declared_env_keys();
        let has_public = env_keys.iter().any(|k| k.starts_with("PUBLIC_"));
        if has_public {
            e.writeln("// Embed public env vars for client-side hydration");
            e.writeln("html.push_str(\"<script type=\\\"gale-env\\\">\");");
            e.writeln("html.push_str(&crate::env_config::ENV.public_vars_json());");
            e.writeln("html.push_str(\"</script>\");");
        }

        // Inject client runtime and per-page script tags if this page has interactive code
        // Paths are resolved via the asset manifest for content-hashed filenames
        let has_client = crate::codegen::emit_client::component_has_client_code(decl);
        if has_client {
            let base_name = if let Some(bracket) = decl.name.find('[') {
                &decl.name[..bracket]
            } else {
                &decl.name
            };
            let page_mod = crate::codegen::types::to_module_name(base_name);
            e.writeln("// Client runtime and per-page hydration script (manifest-resolved paths)");
            e.writeln(
                "html.push_str(&format!(\"<script type=\\\"module\\\" src=\\\"/{}\\\"></script>\"",
            );
            e.indent();
            e.writeln(", crate::asset_manifest::resolve(\"_gale/runtime.js\")));");
            e.dedent();
            e.writeln(&format!(
                "html.push_str(&format!(\"<script type=\\\"module\\\" src=\\\"/{{}}\\\"></script>\"",
            ));
            e.indent();
            e.writeln(&format!(
                ", crate::asset_manifest::resolve(\"_gale/pages/{page_mod}.js\")));",
            ));
            e.dedent();
        }

        e.writeln("html");
    });
    e.newline();
}
