//! Layout composition — generates the layout rendering function.
//!
//! A layout wraps pages with shared chrome (nav, footer, etc.).
//! The layout's `<slot/>` is replaced by the page's body HTML.
//! The layout's `<head>` receives the page's head metadata.

use super::hydration::HydrationCtx;
use super::rust_emitter::RustEmitter;
use super::ssr;
use crate::ast::LayoutDecl;

/// Emit the `src/layout.rs` file from a LayoutDecl.
///
/// Generates a `pub fn render(head_html: &str, body_html: &str) -> String`
/// function that renders the layout template, injecting page content
/// at the `<slot/>` position and head metadata into the document head.
pub fn emit_layout_module(layout: &LayoutDecl) -> String {
    let mut e = RustEmitter::new();
    e.emit_file_header(&format!("Layout: `{}`.", layout.name));
    e.newline();

    // The render function takes head and body HTML from the page
    e.emit_fn(
        "pub",
        false,
        "render",
        &[("head_html", "&str"), ("body_html", "&str")],
        Some("String"),
        |e| {
            e.writeln("let mut html = String::with_capacity(4096);");
            // Prepend import map + framework CSS links to page head content
            e.writeln("let __import_map = crate::asset_manifest::import_map_tag();");
            e.writeln("let __fw_head = format!(\"{}<link rel=\\\"stylesheet\\\" href=\\\"/{}\\\"><link rel=\\\"stylesheet\\\" href=\\\"/{}\\\">{}\"");
            e.indent();
            e.writeln(", __import_map");
            e.writeln(", crate::asset_manifest::resolve(\"_gale/styles.css\")");
            e.writeln(", crate::asset_manifest::resolve(\"_gale/transitions.css\")");
            e.writeln(", head_html);");
            e.dedent();
            e.writeln("let head_html = __fw_head.as_str();");
            e.writeln("html.push_str(\"<!DOCTYPE html>\");");

            let mut hydration = HydrationCtx::new();
            ssr::emit_template_nodes(e, &layout.body.template, &mut hydration, Some("body_html"));

            // Hydration script (if any interactive elements in layout)
            hydration.emit_script(e);

            e.writeln("html");
        },
    );
    e.newline();

    // Note: we do NOT emit render_head() here. The layout's <head> content
    // comes from the template literal.  render_head() is only needed in
    // per-route modules where page-level `head { title: "..." }` blocks
    // exist.

    e.finish()
}

/// Generate a default HTML5 shell layout (when no explicit layout is declared).
///
/// This is a minimal document wrapper that injects head metadata and body
/// content into a standard HTML5 structure.
pub fn emit_default_layout() -> String {
    let mut e = RustEmitter::new();
    e.emit_file_header("Default page layout (no explicit layout declared).");
    e.newline();

    e.emit_fn(
        "pub",
        false,
        "render",
        &[("head_html", "&str"), ("body_html", "&str")],
        Some("String"),
        |e| {
            e.writeln("let mut html = String::with_capacity(4096);");
            e.writeln("html.push_str(\"<!DOCTYPE html>\");");
            e.writeln("html.push_str(\"<html lang=\\\"en\\\">\");");
            e.writeln("html.push_str(\"<head>\");");
            e.writeln("html.push_str(\"<meta charset=\\\"utf-8\\\">\");");
            e.writeln(
                "html.push_str(\"<meta name=\\\"viewport\\\" content=\\\"width=device-width, initial-scale=1\\\">\");",
            );
            // Import map for resolving hashed JS module paths (no-op in dev mode)
            e.writeln("html.push_str(&crate::asset_manifest::import_map_tag());");
            // Framework CSS — Tailwind output + transition animations (manifest-resolved paths)
            e.writeln("html.push_str(&format!(\"<link rel=\\\"stylesheet\\\" href=\\\"/{}\\\">\"");
            e.indent();
            e.writeln(", crate::asset_manifest::resolve(\"_gale/styles.css\")));");
            e.dedent();
            e.writeln("html.push_str(&format!(\"<link rel=\\\"stylesheet\\\" href=\\\"/{}\\\">\"");
            e.indent();
            e.writeln(", crate::asset_manifest::resolve(\"_gale/transitions.css\")));");
            e.dedent();
            e.writeln("html.push_str(head_html);");
            e.writeln("html.push_str(\"</head>\");");
            e.writeln("html.push_str(\"<body>\");");
            e.writeln("html.push_str(body_html);");
            e.writeln("html.push_str(\"</body>\");");
            e.writeln("html.push_str(\"</html>\");");
            e.writeln("html");
        },
    );

    e.finish()
}
