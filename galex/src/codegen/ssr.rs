//! Template → HTML server-side rendering.
//!
//! Walks [`TemplateNode`] trees and emits Rust code that builds an HTML
//! `String` via `push_str()` calls. This is the core SSR engine.
//!
//! The generated code assumes a local `html: String` variable is in scope
//! and a `crate::gale_ssr::escape_html()` function is available.

use super::expr::{expr_to_display_string, expr_to_rust};
use super::hydration::HydrationCtx;
use super::rust_emitter::RustEmitter;
use crate::ast::*;
use crate::codegen::types::to_snake_case;

// ── HTML void elements (self-closing by spec) ──────────────────────────

/// HTML void elements that must not have a closing tag.
const VOID_ELEMENTS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

fn is_void_element(tag: &str) -> bool {
    VOID_ELEMENTS.contains(&tag)
}

// ── Public API ─────────────────────────────────────────────────────────

/// Emit Rust code that renders a list of template nodes into the `html` String.
///
/// The generated code expects `html: String` to be in scope.
/// `slot_param` is the name of the slot content parameter (if inside a layout).
pub fn emit_template_nodes(
    e: &mut RustEmitter,
    nodes: &[TemplateNode],
    hydration: &mut HydrationCtx,
    slot_param: Option<&str>,
) {
    for node in nodes {
        emit_template_node(e, node, hydration, slot_param);
    }
}

// ── Node rendering ─────────────────────────────────────────────────────

fn emit_template_node(
    e: &mut RustEmitter,
    node: &TemplateNode,
    hydration: &mut HydrationCtx,
    slot_param: Option<&str>,
) {
    match node {
        TemplateNode::Text { value, .. } => {
            if !value.is_empty() {
                // Static text — safe to embed directly (no escaping needed)
                let escaped = html_escape_static(value);
                e.writeln(&format!("html.push_str({:?});", escaped));
            }
        }

        TemplateNode::Element {
            tag,
            attributes,
            directives,
            children,
            ..
        } => {
            emit_open_tag(e, tag, attributes, directives, hydration);

            if is_void_element(tag) {
                // Void elements — no children, no closing tag
                return;
            }

            // Children
            for child in children {
                emit_template_node(e, child, hydration, slot_param);
            }

            // In a layout context, inject framework head content (CSS links,
            // import map, page head metadata) before </head>.  The variable
            // `head_html` is prepared by `emit_layout_module()`.
            if tag == "head" && slot_param.is_some() {
                e.writeln("html.push_str(head_html);");
            }

            // Close tag
            e.writeln(&format!("html.push_str(\"</{tag}>\");"));

            // form:guard wiring script — inject after closing tag
            emit_form_wiring_if_needed(e, directives);
        }

        TemplateNode::SelfClosing {
            tag,
            attributes,
            directives,
            ..
        } => {
            emit_open_tag(e, tag, attributes, directives, hydration);
        }

        TemplateNode::ExprInterp { expr, .. } => {
            let expr_str = expr_to_display_string(expr);
            // Wrap in a <span data-gale-text="N"> so client hydration can target it
            let text_id = hydration.mark_text_expr();
            e.writeln(&format!(
                "html.push_str(\"<span data-gale-text=\\\"{text_id}\\\">\");"
            ));
            e.writeln(&format!(
                "html.push_str(&crate::gale_ssr::escape_html(&{expr_str}));"
            ));
            e.writeln("html.push_str(\"</span>\");");
        }

        TemplateNode::When {
            condition,
            body,
            else_branch,
            ..
        } => {
            // Allocate a hydration marker for client-side re-rendering
            let when_id = hydration.mark_when();
            e.writeln(&format!("html.push_str(\"<!--gx-when:{when_id}-->\");"));

            let cond = expr_to_rust(condition);
            e.block(&format!("if {cond}"), |e| {
                emit_template_nodes(e, body, hydration, slot_param);
            });
            if let Some(else_b) = else_branch {
                match else_b {
                    WhenElse::Else(nodes) => {
                        e.block("else", |e| {
                            emit_template_nodes(e, nodes, hydration, slot_param);
                        });
                    }
                    WhenElse::ElseWhen(node) => {
                        e.write("else ");
                        emit_template_node(e, node, hydration, slot_param);
                    }
                }
            }

            e.writeln(&format!("html.push_str(\"<!--/gx-when:{when_id}-->\");"));
        }

        TemplateNode::Each {
            binding,
            index,
            iterable,
            body,
            empty,
            ..
        } => {
            // Allocate a hydration marker for client-side list reconciliation
            let each_id = hydration.mark_each();
            e.writeln(&format!("html.push_str(\"<!--gx-each:{each_id}-->\");"));

            let iter_expr = expr_to_rust(iterable);
            let binding_name = to_snake_case(binding);

            if let Some(empty_nodes) = empty {
                // Emit empty check
                e.block(&format!("if {iter_expr}.is_empty()"), |e| {
                    emit_template_nodes(e, empty_nodes, hydration, slot_param);
                });
                e.block("else", |e| {
                    emit_each_loop(
                        e,
                        &iter_expr,
                        &binding_name,
                        index,
                        body,
                        hydration,
                        slot_param,
                    );
                });
            } else {
                emit_each_loop(
                    e,
                    &iter_expr,
                    &binding_name,
                    index,
                    body,
                    hydration,
                    slot_param,
                );
            }

            e.writeln(&format!("html.push_str(\"<!--/gx-each:{each_id}-->\");"));
        }

        TemplateNode::Slot { default, .. } => {
            // In a layout, slot is replaced by the page content parameter
            if let Some(param) = slot_param {
                e.writeln(&format!("html.push_str({param});"));
            } else if let Some(default_nodes) = default {
                // Render default slot content
                emit_template_nodes(e, default_nodes, hydration, None);
            }
        }

        TemplateNode::Suspend { fallback, body, .. } => {
            // In SSR, render the body directly (async data resolved server-side).
            // Fallback is ignored because the server has the data.
            emit_template_nodes(e, body, hydration, slot_param);
            // Suppress unused fallback warning
            let _ = fallback;
        }
    }
}

// ── Helpers ────────────────────────────────────────────────────────────

/// Emit the opening tag with attributes and directives.
fn emit_open_tag(
    e: &mut RustEmitter,
    tag: &str,
    attributes: &[Attribute],
    directives: &[Directive],
    hydration: &mut HydrationCtx,
) {
    // Collect hydration markers from interactive directives
    let mut hydration_ids = Vec::new();
    let mut class_toggles = Vec::new();

    for directive in directives {
        match directive {
            Directive::Bind { field, .. } => {
                hydration_ids.push(hydration.mark_bind(field));
            }
            Directive::On {
                event, modifiers, ..
            } => {
                hydration_ids.push(hydration.mark_event(event, modifiers));
            }
            Directive::Ref { name, .. } => {
                hydration_ids.push(hydration.mark_ref(name));
            }
            Directive::Transition { kind, .. } => {
                hydration_ids.push(hydration.mark_transition(kind));
            }
            Directive::Class {
                name, condition, ..
            } => {
                hydration_ids.push(hydration.mark_class_toggle(name));
                class_toggles.push((name.clone(), condition.clone()));
            }
            _ => {} // Other directives handled inline
        }
    }

    // Start building the open tag
    e.writeln(&format!("html.push_str(\"<{tag}\");"));

    // Collect static class values and dynamic class expressions for merging
    let static_class_attr = attributes.iter().find(|a| a.name == "class");
    let has_class_sources = static_class_attr.is_some() || !class_toggles.is_empty();

    // Emit non-class attributes
    for attr in attributes {
        if attr.name == "class" {
            continue; // Handled below in merged class emission
        }
        emit_attribute(e, attr);
    }

    // Emit merged class attribute (static classes + class: toggles)
    if has_class_sources {
        e.writeln("{");
        e.indent();
        e.writeln("let mut __classes = Vec::new();");

        // Add static class values
        if let Some(attr) = static_class_attr {
            match &attr.value {
                AttrValue::String(value) => {
                    let escaped = html_escape_static(value);
                    e.writeln(&format!("__classes.push({escaped:?}.to_string());"));
                }
                AttrValue::Expr(expr) => {
                    let val = expr_to_display_string(expr);
                    e.writeln(&format!("__classes.push({val}.to_string());"));
                }
                AttrValue::Bool => {} // bare `class` with no value — skip
            }
        }

        // Add class: toggle values
        for (name, cond) in &class_toggles {
            let cond_str = expr_to_rust(cond);
            e.writeln(&format!(
                "if {cond_str} {{ __classes.push({name:?}.to_string()); }}"
            ));
        }

        e.block("if !__classes.is_empty()", |e| {
            e.writeln("html.push_str(&format!(\" class=\\\"{}\\\"\", __classes.join(\" \")));");
        });
        e.dedent();
        e.writeln("}");
    }

    // form: directives — Pass 1: attributes (emitted before closing `>`)
    for directive in directives {
        match directive {
            Directive::FormAction {
                action: Expr::Ident { name, .. },
                ..
            } => {
                let action_path = to_snake_case(name);
                e.writeln(&format!(
                    "html.push_str(\" action=\\\"/api/{action_path}\\\" method=\\\"post\\\"\");"
                ));
            }
            Directive::FormGuard {
                guard: Expr::Ident { name, .. },
                ..
            } => {
                // data-gale-guard attribute for JS form wiring
                e.writeln(&format!(
                    "html.push_str(\" data-gale-guard=\\\"{name}\\\"\");"
                ));
            }
            _ => {}
        }
    }

    // Hydration markers
    if !hydration_ids.is_empty() {
        // Use the first ID as the element's marker
        let id = hydration_ids[0];
        e.writeln(&format!("html.push_str(\" data-gx-id=\\\"{id}\\\"\");"));
    }

    // Close open tag
    if is_void_element(tag) {
        e.writeln("html.push_str(\" />\");");
    } else {
        e.writeln("html.push_str(\">\");");
    }

    // form: directives — Pass 2: child elements (emitted after closing `>`)
    for directive in directives {
        match directive {
            Directive::FormGuard {
                guard: Expr::Ident { name, .. },
                ..
            } => {
                // Hidden input for server-side guard identification
                e.writeln(&format!(
                    "html.push_str(\"<input type=\\\"hidden\\\" name=\\\"__guard\\\" value=\\\"{name}\\\">\");"
                ));
            }
            Directive::FormError { field, .. } => {
                e.writeln(&format!(
                    "html.push_str(\"<div data-gale-error=\\\"{field}\\\"></div>\");"
                ));
            }
            _ => {}
        }
    }
}

/// Emit a single HTML attribute.
fn emit_attribute(e: &mut RustEmitter, attr: &Attribute) {
    match &attr.value {
        AttrValue::String(value) => {
            let escaped = html_escape_static(value);
            e.writeln(&format!(
                "html.push_str(\" {}=\\\"{}\\\"\");",
                attr.name, escaped
            ));
        }
        AttrValue::Expr(expr) => {
            let val = expr_to_display_string(expr);
            e.writeln(&format!(
                "html.push_str(&format!(\" {}=\\\"{{}}\\\"\", crate::gale_ssr::escape_html(&{val})));",
                attr.name
            ));
        }
        AttrValue::Bool => {
            e.writeln(&format!("html.push_str(\" {}\");", attr.name));
        }
    }
}

/// Emit a for loop over an iterable.
fn emit_each_loop(
    e: &mut RustEmitter,
    iter_expr: &str,
    binding_name: &str,
    index: &Option<smol_str::SmolStr>,
    body: &[TemplateNode],
    hydration: &mut HydrationCtx,
    slot_param: Option<&str>,
) {
    if let Some(idx_name) = index {
        let idx = to_snake_case(idx_name);
        e.block(
            &format!("for ({idx}, {binding_name}) in {iter_expr}.iter().enumerate()"),
            |e| {
                emit_template_nodes(e, body, hydration, slot_param);
            },
        );
    } else {
        e.block(&format!("for {binding_name} in {iter_expr}.iter()"), |e| {
            emit_template_nodes(e, body, hydration, slot_param);
        });
    }
}

/// Emit a `<script type="module">` wiring block if the element has `form:guard`.
///
/// This injects the client-side validation wiring that imports the guard's
/// JS validator and connects it to the form via the `gale-forms.js` runtime.
fn emit_form_wiring_if_needed(e: &mut RustEmitter, directives: &[Directive]) {
    // Find the guard name from form:guard directive
    let guard_name = directives.iter().find_map(|d| {
        if let Directive::FormGuard {
            guard: Expr::Ident { name, .. },
            ..
        } = d
        {
            Some(name.as_str())
        } else {
            None
        }
    });

    let guard_name = match guard_name {
        Some(name) => name,
        None => return,
    };

    // Derive JS identifiers from the guard name
    let module_name = to_snake_case(guard_name);
    let validate_fn = format!("validate{guard_name}");
    let sanitize_fn = format!("sanitize{guard_name}");

    // Collect field names from form:error directives on the same element
    // (typically these are on child elements, so we also emit a dynamic
    // discovery fallback in the script)
    let error_fields: Vec<&str> = directives
        .iter()
        .filter_map(|d| {
            if let Directive::FormError { field, .. } = d {
                Some(field.as_str())
            } else {
                None
            }
        })
        .collect();
    let _ = error_fields; // Fields are discovered dynamically by the runtime

    // Emit the wiring script
    e.writeln("// Client-side guard validation wiring");
    e.writeln("html.push_str(\"<script type=\\\"module\\\">\");");
    e.writeln(&format!(
        "html.push_str(\"import {{ {validate_fn}, {sanitize_fn} }} from '/js/guards/{module_name}.js';\");",
    ));
    e.writeln("html.push_str(\"import { wireForm } from '/js/gale-forms.js';\");");
    e.writeln(&format!(
        "html.push_str(\"var __f=document.querySelector('[data-gale-guard=\\\\\\\"{guard_name}\\\\\\\"]');\");",
    ));
    e.writeln(&format!(
        "html.push_str(\"if(__f)wireForm(__f,{{validate:{validate_fn},sanitize:typeof {sanitize_fn}==='function'?{sanitize_fn}:null,fields:Array.from(__f.querySelectorAll('[name]')).map(function(e){{return e.name}}).filter(function(n){{return n[0]!=='_'}})}});\");",
    ));
    e.writeln("html.push_str(\"</script>\");");
}

/// Escape static text for safe embedding in HTML string literals.
fn html_escape_static(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
