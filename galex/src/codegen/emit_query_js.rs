//! Query → JavaScript reactive wrapper code generation.
//!
//! For each GaleX [`QueryDecl`], generates an ES module that exports a
//! factory function wrapping the runtime `query()` with typed URL
//! interpolation and reactive parameter tracking.
//!
//! Generated output for `query UserById = "/api/users/${id}" -> User`:
//! ```js
//! import { query } from '/_gale/runtime.js';
//!
//! export function UserById(id) {
//!   return query(
//!     () => `/api/users/${typeof id?.get === 'function' ? id.get() : id}`,
//!   );
//! }
//! ```

use std::collections::HashSet;

use crate::ast::*;
use crate::codegen::js_emitter::JsEmitter;
use crate::codegen::types::to_module_name;

// ── Metadata ───────────────────────────────────────────────────────────

/// Metadata about a generated JS query module.
#[derive(Debug, Clone)]
pub struct QueryJsMeta {
    /// PascalCase query name (e.g. `UserById`).
    pub query_name: String,
    /// Snake_case module file name (e.g. `user_by_id`).
    pub module_name: String,
    /// Parameter names extracted from the URL template.
    pub params: Vec<String>,
}

// ── Public entry point ─────────────────────────────────────────────────

/// Emit a complete query JS wrapper module.
pub fn emit_query_js_file(e: &mut JsEmitter, decl: &QueryDecl) -> QueryJsMeta {
    e.emit_file_header(&format!("Query: `{}`.", decl.name));

    // Import the runtime query function
    e.emit_import(&["query"], "/_gale/runtime.js");
    e.newline();

    // Extract parameter names from URL template interpolation expressions
    let params = extract_url_params(&decl.url_pattern);

    // Build the factory function
    let param_list: Vec<&str> = params.iter().map(|s| s.as_str()).collect();
    let url_js = url_pattern_to_js(&decl.url_pattern);

    e.emit_export_fn(&decl.name, &param_list, |e| {
        if params.is_empty() {
            // Static URL — pass as string
            e.writeln(&format!("return query({url_js});"));
        } else {
            // Dynamic URL — pass as function for reactive tracking
            e.writeln(&format!("return query(() => {url_js});"));
        }
    });

    QueryJsMeta {
        query_name: decl.name.to_string(),
        module_name: to_module_name(&decl.name),
        params,
    }
}

// ── URL pattern conversion ─────────────────────────────────────────────

/// Convert a GaleX URL pattern expression to a JS template literal.
///
/// Template literal parts with interpolations become reactive-aware:
/// `${id}` → `${typeof id?.get === 'function' ? id.get() : id}`
///
/// This allows passing either a raw value or a signal, and the runtime's
/// `effect()` wrapper will track signal reads automatically.
fn url_pattern_to_js(expr: &Expr) -> String {
    match expr {
        Expr::StringLit { value, .. } => {
            format!("{:?}", value.as_str())
        }
        Expr::TemplateLit { parts, .. } => {
            let mut out = String::from("`");
            for part in parts {
                match part {
                    TemplatePart::Text(text) => {
                        out.push_str(&text.replace('`', "\\`").replace("${", "\\${"));
                    }
                    TemplatePart::Expr(e) => {
                        let name = expr_ident_name(e);
                        out.push_str("${");
                        if let Some(name) = name {
                            // Signal-aware unwrap: check at runtime if it's a signal
                            out.push_str(&format!(
                                "typeof {name}?.get === 'function' ? {name}.get() : {name}"
                            ));
                        } else {
                            // Complex expression — emit as-is
                            out.push_str(&expr_to_simple_js(e));
                        }
                        out.push('}');
                    }
                }
            }
            out.push('`');
            out
        }
        // Fallback: just convert to a JS string
        _ => format!("{:?}", ""),
    }
}

/// Extract parameter names from URL template interpolation expressions.
fn extract_url_params(expr: &Expr) -> Vec<String> {
    let mut params = Vec::new();
    if let Expr::TemplateLit { parts, .. } = expr {
        for part in parts {
            if let TemplatePart::Expr(e) = part {
                if let Some(name) = expr_ident_name(e) {
                    if !params.contains(&name) {
                        params.push(name);
                    }
                }
            }
        }
    }
    params
}

/// Get the identifier name from a simple expression, if it's just an ident.
fn expr_ident_name(expr: &Expr) -> Option<String> {
    if let Expr::Ident { name, .. } = expr {
        Some(name.to_string())
    } else {
        None
    }
}

/// Simple JS expression conversion for non-reactive URL parts.
fn expr_to_simple_js(expr: &Expr) -> String {
    // Reuse the full expression converter with no signal names
    crate::codegen::js_expr::expr_to_js(expr, &HashSet::new())
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::js_emitter::JsEmitter;
    use crate::span::Span;

    fn s() -> Span {
        Span::dummy()
    }

    fn make_query(name: &str, url: Expr) -> QueryDecl {
        QueryDecl {
            name: name.into(),
            url_pattern: url,
            ret_ty: None,
            span: s(),
        }
    }

    fn str_url(url: &str) -> Expr {
        Expr::StringLit {
            value: url.into(),
            span: s(),
        }
    }

    fn template_url(parts: Vec<TemplatePart>) -> Expr {
        Expr::TemplateLit { parts, span: s() }
    }

    fn text(s: &str) -> TemplatePart {
        TemplatePart::Text(s.into())
    }

    fn interp(name: &str) -> TemplatePart {
        TemplatePart::Expr(Expr::Ident {
            name: name.into(),
            span: s(),
        })
    }

    // ── Static URL ─────────────────────────────────────────────

    #[test]
    fn query_static_url() {
        let decl = make_query("Users", str_url("/api/users"));
        let mut e = JsEmitter::new();
        let meta = emit_query_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(out.contains("export function Users()"), "fn: {out}");
        assert!(
            out.contains("return query(\"/api/users\")"),
            "static url: {out}"
        );
        assert!(meta.params.is_empty());
    }

    // ── Dynamic URL with params ────────────────────────────────

    #[test]
    fn query_dynamic_url_single_param() {
        let decl = make_query(
            "UserById",
            template_url(vec![text("/api/users/"), interp("id")]),
        );
        let mut e = JsEmitter::new();
        let meta = emit_query_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(
            out.contains("export function UserById(id)"),
            "fn with param: {out}"
        );
        assert!(out.contains("return query(() =>"), "reactive url: {out}");
        assert!(out.contains("id.get()"), "signal unwrap: {out}");
        assert_eq!(meta.params, vec!["id"]);
    }

    #[test]
    fn query_dynamic_url_multiple_params() {
        let decl = make_query(
            "SearchUsers",
            template_url(vec![
                text("/api/users?q="),
                interp("query"),
                text("&page="),
                interp("page"),
            ]),
        );
        let mut e = JsEmitter::new();
        let meta = emit_query_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(
            out.contains("export function SearchUsers(query, page)"),
            "multi params: {out}"
        );
        assert_eq!(meta.params, vec!["query", "page"]);
    }

    // ── Imports ────────────────────────────────────────────────

    #[test]
    fn query_imports_runtime() {
        let decl = make_query("Q", str_url("/api"));
        let mut e = JsEmitter::new();
        emit_query_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(out.contains("import { query } from '/_gale/runtime.js'"));
    }

    // ── Metadata ───────────────────────────────────────────────

    #[test]
    fn query_meta() {
        let decl = make_query(
            "UserPosts",
            template_url(vec![text("/api/users/"), interp("userId"), text("/posts")]),
        );
        let mut e = JsEmitter::new();
        let meta = emit_query_js_file(&mut e, &decl);

        assert_eq!(meta.query_name, "UserPosts");
        assert_eq!(meta.module_name, "user_posts");
        assert_eq!(meta.params, vec!["userId"]);
    }

    #[test]
    fn query_header() {
        let decl = make_query("Q", str_url("/api"));
        let mut e = JsEmitter::new();
        emit_query_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(out.contains("Query: `Q`."));
        assert!(out.contains("Generated by GaleX compiler"));
    }

    #[test]
    fn query_dedup_params() {
        // Same param used twice in URL
        let decl = make_query(
            "Q",
            template_url(vec![text("/api/"), interp("id"), text("/"), interp("id")]),
        );
        let mut e = JsEmitter::new();
        let meta = emit_query_js_file(&mut e, &decl);

        assert_eq!(meta.params, vec!["id"], "param should be deduped");
    }
}
