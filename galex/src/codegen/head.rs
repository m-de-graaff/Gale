//! Head metadata rendering — converts `head { }` blocks to HTML `<head>` content.
//!
//! Generates a Rust function that builds the `<head>` HTML string from
//! a component's [`HeadBlock`](crate::ast::HeadBlock) declaration.

use super::expr::expr_to_rust;
use super::rust_emitter::RustEmitter;
use crate::ast::{Expr, HeadBlock};

/// Emit a `fn render_head() -> String` function from a HeadBlock.
///
/// If `head` is `None`, emits an empty-string returning function.
pub fn emit_head_fn(e: &mut RustEmitter, head: Option<&HeadBlock>) {
    e.emit_fn(
        "",
        false,
        "render_head",
        &[],
        Some("String"),
        |e| match head {
            None => {
                e.writeln("String::new()");
            }
            Some(head) => {
                e.writeln("let mut head = String::new();");
                for field in &head.fields {
                    emit_head_field(e, &field.key, &field.value);
                }
                e.writeln("head");
            }
        },
    );
    e.newline();
}

/// Emit a single head field as HTML pushed into the `head` string.
fn emit_head_field(e: &mut RustEmitter, key: &str, value: &Expr) {
    match key {
        "title" => {
            let val = expr_to_rust(value);
            e.writeln(&format!(
                "head.push_str(&format!(\"<title>{{}}</title>\", crate::gale_ssr::escape_html(&{val})));"
            ));
        }
        "description" => {
            let val = expr_to_rust(value);
            e.writeln(&format!(
                "head.push_str(&format!(\"<meta name=\\\"description\\\" content=\\\"{{}}\\\">\", crate::gale_ssr::escape_html(&{val})));"
            ));
        }
        "charset" => {
            let val = expr_to_rust(value);
            e.writeln(&format!(
                "head.push_str(&format!(\"<meta charset=\\\"{{}}\\\">\", crate::gale_ssr::escape_html(&{val})));"
            ));
        }
        "viewport" => {
            let val = expr_to_rust(value);
            e.writeln(&format!(
                "head.push_str(&format!(\"<meta name=\\\"viewport\\\" content=\\\"{{}}\\\">\", crate::gale_ssr::escape_html(&{val})));"
            ));
        }
        "canonical" => {
            let val = expr_to_rust(value);
            e.writeln(&format!(
                "head.push_str(&format!(\"<link rel=\\\"canonical\\\" href=\\\"{{}}\\\">\", crate::gale_ssr::escape_html(&{val})));"
            ));
        }
        "favicon" => {
            let val = expr_to_rust(value);
            e.writeln(&format!(
                "head.push_str(&format!(\"<link rel=\\\"icon\\\" href=\\\"{{}}\\\">\", crate::gale_ssr::escape_html(&{val})));"
            ));
        }
        "robots" => {
            let val = expr_to_rust(value);
            e.writeln(&format!(
                "head.push_str(&format!(\"<meta name=\\\"robots\\\" content=\\\"{{}}\\\">\", crate::gale_ssr::escape_html(&{val})));"
            ));
        }
        // Open Graph object: og: { title: "...", image: "..." }
        "og" => {
            if let Expr::ObjectLit { fields, .. } = value {
                for field in fields {
                    let val = expr_to_rust(&field.value);
                    let prop = &field.key;
                    e.writeln(&format!(
                        "head.push_str(&format!(\"<meta property=\\\"og:{prop}\\\" content=\\\"{{}}\\\">\", crate::gale_ssr::escape_html(&{val})));"
                    ));
                }
            }
        }
        // Twitter cards: twitter: { card: "...", title: "..." }
        "twitter" => {
            if let Expr::ObjectLit { fields, .. } = value {
                for field in fields {
                    let val = expr_to_rust(&field.value);
                    let prop = &field.key;
                    e.writeln(&format!(
                        "head.push_str(&format!(\"<meta name=\\\"twitter:{prop}\\\" content=\\\"{{}}\\\">\", crate::gale_ssr::escape_html(&{val})));"
                    ));
                }
            }
        }
        // Generic meta name=value for other known string properties
        _ => {
            let val = expr_to_rust(value);
            e.writeln(&format!(
                "head.push_str(&format!(\"<meta name=\\\"{key}\\\" content=\\\"{{}}\\\">\", crate::gale_ssr::escape_html(&{val})));"
            ));
        }
    }
}
