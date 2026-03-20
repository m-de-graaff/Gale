//! Rich hover information provider.
//!
//! Shows type signatures, documentation, and descriptions for identifiers,
//! declarations, HTML tags, directives, and validators.

use lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind};

use super::document::DocumentManager;
use super::position::{self, node_info_span, span_to_lsp_range, DeclKind, NodeInfo};
use crate::ast::*;
use crate::types::env::BindingKind;

/// Provide hover information at the given byte offset.
pub fn provide_hover(
    docs: &DocumentManager,
    file_id: u32,
    offset: u32,
    source: &str,
) -> Option<Hover> {
    let program = docs.merged_program()?;
    let node = position::node_at_offset(program, file_id, offset)?;
    let checker = docs.cached_checker.as_ref();

    let content = match node {
        NodeInfo::Ident { ref name, .. } => hover_for_ident(name, docs, checker),
        NodeInfo::Decl {
            ref name, ref kind, ..
        } => hover_for_decl(name, kind, docs, checker),
        NodeInfo::TypeRef { ref name, .. } => hover_for_type_ref(name, checker),
        NodeInfo::HtmlTag { ref tag, .. } => hover_for_html_tag(tag),
        NodeInfo::DirectiveRef { ref kind, .. } => hover_for_directive(kind),
        NodeInfo::ExprNode { .. } => hover_for_expression(checker),
    };

    let content = content?;

    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: content,
        }),
        range: Some(span_to_lsp_range(&node_info_span(&node), source)),
    })
}

fn hover_for_ident(
    name: &str,
    docs: &DocumentManager,
    checker: Option<&crate::checker::TypeChecker>,
) -> Option<String> {
    if let Some(checker) = checker {
        if let Some(binding) = checker.env.lookup(name) {
            let type_str = checker.interner.display(binding.ty);
            let kind_str = format!("{:?}", binding.kind).to_lowercase();

            let mut result = format!("```gale\n{kind_str} {name}: {type_str}\n```");

            // Add contextual information based on binding kind
            match binding.kind {
                BindingKind::Signal => {
                    result.push_str("\n\n*Reactive signal* — changes trigger UI updates.");
                }
                BindingKind::Derived => {
                    result.push_str(
                        "\n\n*Derived value* — automatically recomputes when dependencies change.",
                    );
                }
                BindingKind::Frozen => {
                    result.push_str("\n\n*Frozen binding* — immutable after initialization.");
                }
                BindingKind::Action => {
                    result.push_str(
                        "\n\n*Server action* — runs on the server, callable from client forms.",
                    );
                }
                BindingKind::Query => {
                    result.push_str("\n\n*Client query* — fetches data from an API endpoint.");
                }
                BindingKind::Guard => {
                    // Show guard fields if available
                    if let Some(program) = docs.merged_program() {
                        if let Some(guard_info) = find_guard_info(&program.items, name) {
                            result.push_str(&guard_info);
                        }
                    }
                }
                BindingKind::Store => {
                    if let Some(program) = docs.merged_program() {
                        if let Some(store_info) = find_store_info(&program.items, name) {
                            result.push_str(&store_info);
                        }
                    }
                }
                BindingKind::Component => {
                    result.push_str("\n\n*UI component* — renders reactive HTML.");
                }
                BindingKind::Channel => {
                    result.push_str(
                        "\n\n*WebSocket channel* — real-time bidirectional communication.",
                    );
                }
                _ => {}
            }

            return Some(result);
        }
    }

    // Check if it's a validator method name
    if let Some(desc) = validator_description(name) {
        return Some(format!("**Validator:** `{name}()`\n\n{desc}"));
    }

    Some(format!("`{name}` — unresolved"))
}

fn hover_for_decl(
    name: &str,
    kind: &DeclKind,
    docs: &DocumentManager,
    checker: Option<&crate::checker::TypeChecker>,
) -> Option<String> {
    let kind_str = match kind {
        DeclKind::Function => "fn",
        DeclKind::Guard => "guard",
        DeclKind::Store => "store",
        DeclKind::Action => "action",
        DeclKind::Query => "query",
        DeclKind::Channel => "channel",
        DeclKind::Component => "out ui",
        DeclKind::Layout => "out layout",
        DeclKind::Api => "out api",
        DeclKind::Middleware => "middleware",
        DeclKind::TypeAlias => "type",
        DeclKind::Enum => "enum",
    };

    let mut result = if let Some(checker) = checker {
        if let Some(binding) = checker.env.lookup(name) {
            let type_str = checker.interner.display(binding.ty);
            format!("```gale\n{kind_str} {name}: {type_str}\n```")
        } else {
            format!("```gale\n{kind_str} {name}\n```")
        }
    } else {
        format!("```gale\n{kind_str} {name}\n```")
    };

    // Add detailed info based on declaration type
    if let Some(program) = docs.merged_program() {
        match kind {
            DeclKind::Guard => {
                if let Some(info) = find_guard_info(&program.items, name) {
                    result.push_str(&info);
                }
            }
            DeclKind::Store => {
                if let Some(info) = find_store_info(&program.items, name) {
                    result.push_str(&info);
                }
            }
            DeclKind::Action => {
                if let Some(info) = find_action_info(&program.items, name) {
                    result.push_str(&info);
                }
            }
            DeclKind::Enum => {
                if let Some(info) = find_enum_info(&program.items, name) {
                    result.push_str(&info);
                }
            }
            DeclKind::Channel => {
                if let Some(info) = find_channel_info(&program.items, name) {
                    result.push_str(&info);
                }
            }
            _ => {}
        }
    }

    Some(result)
}

fn hover_for_type_ref(name: &str, checker: Option<&crate::checker::TypeChecker>) -> Option<String> {
    // Built-in type descriptions
    let builtin = match name {
        "string" => Some("UTF-8 string type. Supports `.trim()`, `.minLen()`, `.maxLen()`, `.email()`, `.url()` validators."),
        "int" => Some("64-bit signed integer. Supports `.min()`, `.max()`, `.positive()`, `.integer()` validators."),
        "float" => Some("64-bit floating point number. Supports `.min()`, `.max()`, `.precision()` validators."),
        "bool" => Some("Boolean value (`true` or `false`)."),
        "void" => Some("No return value."),
        "null" => Some("Null value type."),
        "never" => Some("Type that never produces a value (always throws or loops infinitely)."),
        "HTMLElement" => Some("DOM element reference type. Obtained via `ref:name` directive."),
        "Event" => Some("DOM event object. Passed to `on:event` handlers."),
        _ => None,
    };

    if let Some(desc) = builtin {
        return Some(format!("```gale\ntype {name}\n```\n\n{desc}"));
    }

    if let Some(checker) = checker {
        if let Some(ty_id) = checker.env.resolve_type(name) {
            let type_str = checker.interner.display(ty_id);
            return Some(format!("```gale\ntype {name} = {type_str}\n```"));
        }
    }

    Some(format!("`{name}` — type"))
}

fn hover_for_html_tag(tag: &str) -> Option<String> {
    let desc = match tag {
        "div" => "Generic container element. Block-level.",
        "span" => "Inline container element.",
        "p" => "Paragraph element.",
        "a" => "Hyperlink element. Use `href` attribute for the URL.",
        "button" => "Interactive button element. Use `on:click` for handlers.",
        "input" => "Form input element. Use `bind:value` for two-way binding.",
        "form" => "Form container. Use `form:action`, `form:guard` for GaleX integration.",
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => "Heading element.",
        "ul" => "Unordered list container.",
        "ol" => "Ordered list container.",
        "li" => "List item.",
        "img" => "Image element. Requires `alt` attribute for accessibility.",
        "video" => "Video player element.",
        "audio" => "Audio player element.",
        "table" => "Table element.",
        "nav" => "Navigation section.",
        "header" => "Page or section header.",
        "footer" => "Page or section footer.",
        "main" => "Main content area. Should be unique per page.",
        "section" => "Thematic grouping of content.",
        "article" => "Self-contained composition.",
        "label" => "Form control label. Use `for` attribute to associate.",
        "select" => "Dropdown select element.",
        "textarea" => "Multi-line text input.",
        "slot" => "**GaleX slot** — content injection point for layouts. Pages fill this slot.",
        "pre" => "Preformatted text block.",
        "code" => "Inline code element.",
        _ => return Some(format!("`<{tag}>` — HTML element")),
    };

    Some(format!("`<{tag}>` — {desc}"))
}

fn hover_for_directive(kind: &str) -> Option<String> {
    let (title, desc, example) = match kind {
        "bind" => (
            "bind:field",
            "Two-way data binding. Syncs an input's value with a signal or guard field.",
            "```gale\n<input bind:value={name} />\n```",
        ),
        "on" => (
            "on:event",
            "Event handler directive. Attaches a callback to a DOM event.",
            "```gale\n<button on:click={handleClick}>Click</button>\n```\n\nSupports modifiers: `.prevent`, `.stop`, `.once`",
        ),
        "class" => (
            "class:name",
            "Conditional CSS class. Adds/removes a class based on a boolean expression.",
            "```gale\n<div class:active={isActive}>...</div>\n```",
        ),
        "ref" => (
            "ref:name",
            "DOM element reference. Binds a variable to the actual DOM node.",
            "```gale\nref el: HTMLElement\n<canvas ref:el />\n```",
        ),
        "transition" => (
            "transition:type",
            "CSS transition animation. Applied when elements enter/leave the DOM.",
            "```gale\n<div transition:fade>...</div>\n```",
        ),
        "key" => (
            "key",
            "Keyed list item. Ensures efficient DOM diffing in `each` loops.",
            "```gale\neach item in items {\n  <li key={item.id}>{item.name}</li>\n}\n```",
        ),
        "form:action" => (
            "form:action",
            "Binds a server action to the form's submit event. The action receives validated form data.",
            "```gale\n<form form:action={submitForm} form:guard={FormGuard}>\n  ...\n</form>\n```",
        ),
        "form:guard" => (
            "form:guard",
            "Attaches a guard for client + server validation. Fields are validated before the action runs.",
            "```gale\n<form form:guard={LoginGuard}>\n  <input bind:value={email} />\n  <form:error field=\"email\" />\n</form>\n```",
        ),
        "form:error" => (
            "form:error",
            "Displays validation errors for a specific guard field.",
            "```gale\n<form:error field=\"email\" class=\"error-text\" />\n```",
        ),
        _ => return Some(format!("**{kind}** — directive")),
    };

    Some(format!("**{title}**\n\n{desc}\n\n{example}"))
}

fn hover_for_expression(checker: Option<&crate::checker::TypeChecker>) -> Option<String> {
    // Could re-infer the expression type here in the future
    let _ = checker;
    None // Don't show hover for generic expressions
}

// ── AST lookup helpers ─────────────────────────────────────────────────

fn find_guard_info(items: &[Item], name: &str) -> Option<String> {
    for item in items {
        match item {
            Item::GuardDecl(g) if g.name == name => {
                let mut info = String::from("\n\n**Fields:**\n");
                for field in &g.fields {
                    info.push_str(&format!(
                        "- `{}`: `{}`",
                        field.name,
                        format_type_ann(&field.ty)
                    ));
                    if !field.validators.is_empty() {
                        let chain: Vec<String> = field
                            .validators
                            .iter()
                            .map(|v| format!(".{}()", v.name))
                            .collect();
                        info.push_str(&chain.join(""));
                    }
                    info.push('\n');
                }
                return Some(info);
            }
            Item::Out(out) => {
                if let Some(info) = find_guard_info(&[*out.inner.clone()], name) {
                    return Some(info);
                }
            }
            Item::ServerBlock(b) | Item::ClientBlock(b) | Item::SharedBlock(b) => {
                if let Some(info) = find_guard_info(&b.items, name) {
                    return Some(info);
                }
            }
            _ => {}
        }
    }
    None
}

fn find_store_info(items: &[Item], name: &str) -> Option<String> {
    for item in items {
        match item {
            Item::StoreDecl(s) if s.name == name => {
                let mut signals = Vec::new();
                let mut derives = Vec::new();
                let mut methods = Vec::new();
                for member in &s.members {
                    match member {
                        StoreMember::Signal(Stmt::Signal { name, .. }) => {
                            signals.push(name.to_string());
                        }
                        StoreMember::Derive(Stmt::Derive { name, .. }) => {
                            derives.push(name.to_string());
                        }
                        StoreMember::Method(f) => {
                            methods.push(f.name.to_string());
                        }
                        _ => {}
                    }
                }
                let mut info = String::from("\n\n**Members:**\n");
                for s in &signals {
                    info.push_str(&format!("- `signal {s}`\n"));
                }
                for d in &derives {
                    info.push_str(&format!("- `derive {d}`\n"));
                }
                for m in &methods {
                    info.push_str(&format!("- `fn {m}()`\n"));
                }
                return Some(info);
            }
            Item::Out(out) => {
                if let Some(info) = find_store_info(&[*out.inner.clone()], name) {
                    return Some(info);
                }
            }
            Item::ServerBlock(b) | Item::ClientBlock(b) | Item::SharedBlock(b) => {
                if let Some(info) = find_store_info(&b.items, name) {
                    return Some(info);
                }
            }
            _ => {}
        }
    }
    None
}

fn find_action_info(items: &[Item], name: &str) -> Option<String> {
    for item in items {
        match item {
            Item::ActionDecl(a) if a.name == name => {
                let params: Vec<String> = a
                    .params
                    .iter()
                    .map(|p| {
                        let ty = p
                            .ty_ann
                            .as_ref()
                            .map(|t| format_type_ann(t))
                            .unwrap_or_else(|| "unknown".into());
                        format!("{}: {ty}", p.name)
                    })
                    .collect();
                let ret = a
                    .ret_ty
                    .as_ref()
                    .map(|t| format!(" -> {}", format_type_ann(t)))
                    .unwrap_or_default();
                return Some(format!(
                    "\n\n```gale\naction {}({}){ret}\n```\n\n*Runs on the server.* Call via `form:action` or directly.",
                    a.name,
                    params.join(", ")
                ));
            }
            Item::Out(out) => {
                if let Some(info) = find_action_info(&[*out.inner.clone()], name) {
                    return Some(info);
                }
            }
            Item::ServerBlock(b) | Item::ClientBlock(b) | Item::SharedBlock(b) => {
                if let Some(info) = find_action_info(&b.items, name) {
                    return Some(info);
                }
            }
            _ => {}
        }
    }
    None
}

fn find_enum_info(items: &[Item], name: &str) -> Option<String> {
    for item in items {
        match item {
            Item::EnumDecl(e) if e.name == name => {
                let variants: Vec<String> = e.variants.iter().map(|v| format!("`{v}`")).collect();
                return Some(format!("\n\n**Variants:** {}", variants.join(", ")));
            }
            Item::Out(out) => {
                if let Some(info) = find_enum_info(&[*out.inner.clone()], name) {
                    return Some(info);
                }
            }
            Item::ServerBlock(b) | Item::ClientBlock(b) | Item::SharedBlock(b) => {
                if let Some(info) = find_enum_info(&b.items, name) {
                    return Some(info);
                }
            }
            _ => {}
        }
    }
    None
}

fn find_channel_info(items: &[Item], name: &str) -> Option<String> {
    for item in items {
        match item {
            Item::ChannelDecl(ch) if ch.name == name => {
                let dir = match ch.direction {
                    ChannelDirection::ServerToClient => "->",
                    ChannelDirection::ClientToServer => "<-",
                    ChannelDirection::Bidirectional => "<->",
                };
                let handlers: Vec<&str> = ch.handlers.iter().map(|h| h.event.as_str()).collect();
                return Some(format!(
                    "\n\n```gale\nchannel {}() {dir} {}\n```\n\nHandlers: {}",
                    ch.name,
                    format_type_ann(&ch.msg_ty),
                    handlers.join(", ")
                ));
            }
            Item::Out(out) => {
                if let Some(info) = find_channel_info(&[*out.inner.clone()], name) {
                    return Some(info);
                }
            }
            Item::ServerBlock(b) | Item::ClientBlock(b) | Item::SharedBlock(b) => {
                if let Some(info) = find_channel_info(&b.items, name) {
                    return Some(info);
                }
            }
            _ => {}
        }
    }
    None
}

// ── Formatting helpers ─────────────────────────────────────────────────

fn format_type_ann(ty: &TypeAnnotation) -> String {
    match ty {
        TypeAnnotation::Named { name, .. } => name.to_string(),
        TypeAnnotation::Array { element, .. } => format!("{}[]", format_type_ann(element)),
        TypeAnnotation::Union { types, .. } => types
            .iter()
            .map(|t| format_type_ann(t))
            .collect::<Vec<_>>()
            .join(" | "),
        TypeAnnotation::Optional { inner, .. } => format!("{}?", format_type_ann(inner)),
        TypeAnnotation::StringLiteral { value, .. } => format!("\"{value}\""),
        TypeAnnotation::Function { params, ret, .. } => {
            let params_str: Vec<String> = params.iter().map(|p| format_type_ann(p)).collect();
            format!("fn({}) -> {}", params_str.join(", "), format_type_ann(ret))
        }
        TypeAnnotation::Tuple { elements, .. } => {
            let els: Vec<String> = elements.iter().map(|e| format_type_ann(e)).collect();
            format!("({})", els.join(", "))
        }
        TypeAnnotation::Object { fields, .. } => {
            let fs: Vec<String> = fields
                .iter()
                .map(|f| format!("{}: {}", f.name, format_type_ann(&f.ty)))
                .collect();
            format!("{{ {} }}", fs.join(", "))
        }
    }
}

fn validator_description(name: &str) -> Option<&'static str> {
    match name {
        "trim" => Some("Removes leading and trailing whitespace from the string value."),
        "minLen" => Some("Enforces a minimum string length. `minLen(n)` — value must have at least `n` characters."),
        "maxLen" => Some("Enforces a maximum string length. `maxLen(n)` — value must have at most `n` characters."),
        "nonEmpty" => Some("Value must not be empty. Equivalent to `minLen(1)`."),
        "email" => Some("Value must be a valid email address (RFC 5322 format)."),
        "url" => Some("Value must be a valid URL."),
        "uuid" => Some("Value must be a valid UUID (v4 format)."),
        "regex" => Some("Value must match the given regular expression pattern."),
        "optional" => Some("Field is optional — empty values are allowed and skip other validators."),
        "nullable" => Some("Field can be `null`."),
        "min" => Some("Minimum numeric value. `min(n)` — value must be >= `n`."),
        "max" => Some("Maximum numeric value. `max(n)` — value must be <= `n`."),
        "positive" => Some("Value must be positive (> 0)."),
        "nonNegative" => Some("Value must not be negative (>= 0)."),
        "integer" => Some("Value must be a whole number (no decimal part)."),
        "precision" => Some("Controls decimal precision. `precision(n)` — at most `n` decimal places."),
        "oneOf" => Some("Value must be one of the specified values."),
        _ => None,
    }
}
