//! Context-aware autocomplete provider.
//!
//! Determines the cursor context (top-level, guard field, template tag,
//! expression, type position, etc.) and returns only relevant completions.

use lsp_types::{
    CompletionItem, CompletionItemKind, Documentation, InsertTextFormat, MarkupContent, MarkupKind,
    Url,
};

use super::document::DocumentManager;
use crate::ast::*;
use crate::types::env::BindingKind;

// ── Context detection ──────────────────────────────────────────────────

/// What kind of completion to provide based on cursor position.
#[derive(Debug)]
#[allow(dead_code)]
enum CompletionContext {
    /// At the top level of a file — offer declaration keywords.
    TopLevel,
    /// Inside a guard body — offer type names after `:`.
    GuardFieldType,
    /// After a `.` in a guard field — offer validator methods.
    ValidatorChain,
    /// Inside `<` in a template — offer HTML tags and component names.
    TemplateTag,
    /// Inside an HTML open tag (after the tag name) — offer attributes and directives.
    TemplateAttribute,
    /// After `on:` — offer DOM event names.
    DirectiveEvent,
    /// Inside `{...}` interpolation or expression position — offer bindings.
    Expression,
    /// In a type annotation position (after `:` in params, return types, etc.).
    TypePosition,
    /// Inside a boundary block (`server { }`, `client { }`, `shared { }`).
    BoundaryBody,
    /// Inside a component body (signals, head, template).
    ComponentBody,
    /// Inside an action/fn body — offer statement keywords + bindings.
    CodeBody,
    /// Inside an import path — offer nothing special for now.
    ImportPath,
    /// Could not determine context — offer a broad set.
    Unknown,
}

/// Provide completion items based on cursor context.
pub fn provide_completions(
    docs: &DocumentManager,
    uri: &Url,
    source: &str,
    offset: u32,
    trigger: Option<&str>,
) -> Vec<CompletionItem> {
    let ctx = determine_context(docs, uri, source, offset, trigger);
    let mut items = Vec::new();

    match ctx {
        CompletionContext::TopLevel => {
            add_toplevel_keywords(&mut items);
        }
        CompletionContext::GuardFieldType | CompletionContext::TypePosition => {
            add_type_completions(docs, &mut items);
        }
        CompletionContext::ValidatorChain => {
            add_validator_completions(&mut items);
        }
        CompletionContext::TemplateTag => {
            add_html_tag_completions(&mut items);
            add_component_completions(docs, &mut items);
            // Also offer template control flow
            add_template_keywords(&mut items);
        }
        CompletionContext::TemplateAttribute => {
            add_directive_completions(&mut items);
            add_html_attribute_completions(&mut items);
        }
        CompletionContext::DirectiveEvent => {
            add_event_completions(&mut items);
        }
        CompletionContext::Expression => {
            add_binding_completions(docs, &mut items);
        }
        CompletionContext::BoundaryBody => {
            add_boundary_keywords(&mut items);
            add_binding_completions(docs, &mut items);
        }
        CompletionContext::ComponentBody => {
            add_component_body_keywords(&mut items);
            add_binding_completions(docs, &mut items);
        }
        CompletionContext::CodeBody => {
            add_code_keywords(&mut items);
            add_binding_completions(docs, &mut items);
        }
        CompletionContext::ImportPath => {
            // No special completions for import paths yet
        }
        CompletionContext::Unknown => {
            // Fallback: offer keywords + bindings (but not everything)
            add_toplevel_keywords(&mut items);
            add_code_keywords(&mut items);
            add_binding_completions(docs, &mut items);
        }
    }

    items
}

/// Determine the completion context by inspecting text before the cursor
/// and the AST context.
fn determine_context(
    docs: &DocumentManager,
    uri: &Url,
    source: &str,
    offset: u32,
    trigger: Option<&str>,
) -> CompletionContext {
    let off = offset as usize;
    let before = &source[..off.min(source.len())];
    let trimmed = before.trim_end();

    // Trigger character shortcuts
    if trigger == Some(".") {
        // Check if we're in a guard field context (after a type name)
        if is_in_guard_context(before) {
            return CompletionContext::ValidatorChain;
        }
        // Otherwise it's a member access expression
        return CompletionContext::Expression;
    }
    if trigger == Some("<") {
        return CompletionContext::TemplateTag;
    }
    if trigger == Some(":") {
        // Check if this is `on:`, `bind:`, `class:`, etc.
        if trimmed.ends_with("on:") {
            return CompletionContext::DirectiveEvent;
        }
        // After `:` in a type annotation position
        if is_in_type_position(before) {
            return CompletionContext::GuardFieldType;
        }
    }
    if trigger == Some("\"") || trigger == Some("/") {
        // After `from "` in an import
        if before.contains("from ") {
            return CompletionContext::ImportPath;
        }
    }

    // Use AST context if available
    if let Some(ast) = docs.get_ast(uri) {
        if let Some(ctx) = context_from_ast(ast, source, off) {
            return ctx;
        }
    }

    // Text-based heuristics as fallback
    context_from_text(before)
}

/// Try to determine context from the AST by finding which construct
/// the cursor is inside.
fn context_from_ast(program: &Program, source: &str, offset: usize) -> Option<CompletionContext> {
    let offset = offset as u32;
    for item in &program.items {
        if let Some(ctx) = context_in_item(item, offset, source) {
            return Some(ctx);
        }
    }
    None
}

fn context_in_item(item: &Item, offset: u32, source: &str) -> Option<CompletionContext> {
    match item {
        Item::GuardDecl(g) if g.span.contains_offset(offset) => {
            let before = &source[..offset as usize];
            // After a `:` following a field name → type position
            let last_colon = before.rfind(':');
            let last_dot = before.rfind('.');
            if let Some(dot_pos) = last_dot {
                // Check if the dot is after the colon (validator chain)
                if last_colon.map_or(false, |cp| dot_pos > cp) {
                    return Some(CompletionContext::ValidatorChain);
                }
            }
            if let Some(colon_pos) = last_colon {
                // Check that the colon is after a field name
                let after_colon = before[colon_pos + 1..].trim();
                if after_colon.is_empty()
                    || after_colon.chars().all(|c| c.is_alphanumeric() || c == '_')
                {
                    return Some(CompletionContext::GuardFieldType);
                }
            }
            Some(CompletionContext::GuardFieldType)
        }
        Item::ComponentDecl(c) if c.span.contains_offset(offset) => {
            context_in_component_body(&c.body, offset, source)
        }
        Item::LayoutDecl(l) if l.span.contains_offset(offset) => {
            context_in_component_body(&l.body, offset, source)
        }
        Item::ActionDecl(a) if a.span.contains_offset(offset) => Some(CompletionContext::CodeBody),
        Item::FnDecl(f) if f.span.contains_offset(offset) => Some(CompletionContext::CodeBody),
        Item::StoreDecl(s) if s.span.contains_offset(offset) => Some(CompletionContext::CodeBody),
        Item::ServerBlock(b) | Item::ClientBlock(b) | Item::SharedBlock(b)
            if b.span.contains_offset(offset) =>
        {
            // Check if cursor is inside a nested item
            for inner in &b.items {
                if let Some(ctx) = context_in_item(inner, offset, source) {
                    return Some(ctx);
                }
            }
            Some(CompletionContext::BoundaryBody)
        }
        Item::Out(out) => context_in_item(&out.inner, offset, source),
        Item::EnvDecl(e) if e.span.contains_offset(offset) => {
            Some(CompletionContext::GuardFieldType)
        }
        _ => None,
    }
}

fn context_in_component_body(
    body: &ComponentBody,
    offset: u32,
    source: &str,
) -> Option<CompletionContext> {
    let before = &source[..offset as usize];

    // Check if cursor is inside a template node
    for node in &body.template {
        if let Some(ctx) = context_in_template(node, offset, before) {
            return Some(ctx);
        }
    }

    // Check if cursor is inside head block
    if let Some(ref head) = body.head {
        if head.span.contains_offset(offset) {
            return Some(CompletionContext::Expression);
        }
    }

    // Check if cursor is inside a statement
    for stmt in &body.stmts {
        if stmt_contains(stmt, offset) {
            return Some(CompletionContext::CodeBody);
        }
    }

    Some(CompletionContext::ComponentBody)
}

fn context_in_template(
    node: &TemplateNode,
    offset: u32,
    before: &str,
) -> Option<CompletionContext> {
    match node {
        TemplateNode::Element { children, span, .. } if span.contains_offset(offset) => {
            // Check children first
            for child in children {
                if let Some(ctx) = context_in_template(child, offset, before) {
                    return Some(ctx);
                }
            }
            // If we're in the element but not in a child, could be attribute position
            // Check if cursor is before the closing `>` of the open tag
            let trimmed = before.trim_end();
            if trimmed.ends_with("on:") {
                return Some(CompletionContext::DirectiveEvent);
            }
            // Inside template — could be a new tag or expression
            Some(CompletionContext::ComponentBody)
        }
        TemplateNode::SelfClosing { span, .. } if span.contains_offset(offset) => {
            let trimmed = before.trim_end();
            if trimmed.ends_with("on:") {
                return Some(CompletionContext::DirectiveEvent);
            }
            Some(CompletionContext::TemplateAttribute)
        }
        TemplateNode::ExprInterp { span, .. } if span.contains_offset(offset) => {
            Some(CompletionContext::Expression)
        }
        TemplateNode::When {
            body,
            condition,
            span,
            ..
        } if span.contains_offset(offset) => {
            if condition.span().contains_offset(offset) {
                return Some(CompletionContext::Expression);
            }
            for child in body {
                if let Some(ctx) = context_in_template(child, offset, before) {
                    return Some(ctx);
                }
            }
            Some(CompletionContext::ComponentBody)
        }
        TemplateNode::Each {
            body,
            iterable,
            span,
            ..
        } if span.contains_offset(offset) => {
            if iterable.span().contains_offset(offset) {
                return Some(CompletionContext::Expression);
            }
            for child in body {
                if let Some(ctx) = context_in_template(child, offset, before) {
                    return Some(ctx);
                }
            }
            Some(CompletionContext::ComponentBody)
        }
        _ => None,
    }
}

fn stmt_contains(stmt: &Stmt, offset: u32) -> bool {
    match stmt {
        Stmt::Let { span, .. }
        | Stmt::Mut { span, .. }
        | Stmt::Signal { span, .. }
        | Stmt::Derive { span, .. }
        | Stmt::Frozen { span, .. }
        | Stmt::RefDecl { span, .. }
        | Stmt::If { span, .. }
        | Stmt::For { span, .. }
        | Stmt::Return { span, .. }
        | Stmt::Effect { span, .. }
        | Stmt::Watch { span, .. }
        | Stmt::ExprStmt { span, .. } => span.contains_offset(offset),
        Stmt::FnDecl(f) => f.span.contains_offset(offset),
        Stmt::Block(b) => b.span.contains_offset(offset),
    }
}

/// Text-based heuristic to determine context when AST lookup fails.
fn context_from_text(before: &str) -> CompletionContext {
    let trimmed = before.trim_end();

    // Check for patterns in the text before cursor
    if trimmed.ends_with('<') || trimmed.ends_with("</") {
        return CompletionContext::TemplateTag;
    }
    if trimmed.ends_with("on:") {
        return CompletionContext::DirectiveEvent;
    }

    // Inside a guard block: look for `guard Name {` pattern
    if is_in_guard_context(before) {
        if trimmed.ends_with('.') {
            return CompletionContext::ValidatorChain;
        }
        if trimmed.ends_with(':') {
            return CompletionContext::GuardFieldType;
        }
    }

    // After `->` in a return type annotation
    if trimmed.ends_with("->") {
        return CompletionContext::TypePosition;
    }

    // Inside a brace-delimited block — count nesting
    let open_braces = before.matches('{').count();
    let close_braces = before.matches('}').count();
    if open_braces > close_braces {
        // We're inside some block
        // Check if last unclosed block is a known type
        if let Some(last_keyword) = find_last_block_keyword(before) {
            return match last_keyword {
                "guard" | "env" => CompletionContext::GuardFieldType,
                "server" | "client" | "shared" => CompletionContext::BoundaryBody,
                "action" | "fn" | "test" | "middleware" => CompletionContext::CodeBody,
                _ => CompletionContext::ComponentBody,
            };
        }
    }

    CompletionContext::TopLevel
}

/// Check if we're inside a guard block based on text.
fn is_in_guard_context(before: &str) -> bool {
    // Simple heuristic: find the last `guard` keyword and check if we're
    // inside its braces
    if let Some(guard_pos) = before.rfind("guard ") {
        let after_guard = &before[guard_pos..];
        let opens = after_guard.matches('{').count();
        let closes = after_guard.matches('}').count();
        return opens > closes;
    }
    false
}

/// Check if we're in a type position (after `:` in params, field declarations, etc.).
fn is_in_type_position(before: &str) -> bool {
    let trimmed = before.trim_end();
    // After `name:` or `param:` pattern
    if let Some(colon_pos) = trimmed.rfind(':') {
        let before_colon = trimmed[..colon_pos].trim_end();
        // Check that what's before the colon looks like an identifier
        before_colon
            .chars()
            .last()
            .map_or(false, |c| c.is_alphanumeric() || c == '_')
    } else {
        false
    }
}

/// Find the keyword that opened the most recent unclosed block.
fn find_last_block_keyword(before: &str) -> Option<&str> {
    let keywords = [
        "guard",
        "server",
        "client",
        "shared",
        "action",
        "fn",
        "store",
        "channel",
        "test",
        "middleware",
        "env",
    ];

    let mut best: Option<(&str, usize)> = None;
    for kw in &keywords {
        if let Some(pos) = before.rfind(kw) {
            // Verify this keyword starts a block (followed by something + `{`)
            let after = &before[pos + kw.len()..];
            if after.contains('{') {
                // Check nesting: the block must still be open
                let opens = after.matches('{').count();
                let closes = after.matches('}').count();
                if opens > closes {
                    if best.map_or(true, |(_, bp)| pos > bp) {
                        best = Some((kw, pos));
                    }
                }
            }
        }
    }
    best.map(|(kw, _)| kw)
}

// ── Completion item builders ───────────────────────────────────────────

fn add_toplevel_keywords(items: &mut Vec<CompletionItem>) {
    let keywords = [
        ("guard", "Define a validation guard", "guard ${1:Name} {\n\t${2:field}: ${3:string}\n}"),
        ("server", "Server-only boundary block", "server {\n\t$0\n}"),
        ("client", "Client-only boundary block", "client {\n\t$0\n}"),
        ("shared", "Shared between server and client", "shared {\n\t$0\n}"),
        ("out ui", "Define a UI component", "out ui ${1:Name} {\n\thead {\n\t\ttitle: \"${2:Title}\"\n\t}\n\n\t<main>\n\t\t$0\n\t</main>\n}"),
        ("out layout", "Define a layout", "out layout ${1:Name} {\n\t$0\n\t<slot/>\n}"),
        ("out api", "Define an API resource", "out api ${1:Resource} {\n\tget() {\n\t\t$0\n\t}\n}"),
        ("store", "Define a reactive store", "store ${1:Name} {\n\tsignal ${2:count} = ${3:0}\n}"),
        ("channel", "Define a WebSocket channel", "channel ${1:name}() <-> ${2:string} {\n\ton connect(emit) {\n\t\t$0\n\t}\n}"),
        ("fn", "Define a function", "fn ${1:name}(${2:params}) {\n\t$0\n}"),
        ("action", "Define a server action", "action ${1:name}(${2:data}: ${3:Type}) -> ${4:string} {\n\t$0\n}"),
        ("query", "Define a client query", "query ${1:name} = \"${2:/api/endpoint}\" -> ${3:Type}"),
        ("type", "Define a type alias", "type ${1:Name} = ${2:string}"),
        ("enum", "Define an enum", "enum ${1:Name} {\n\t${2:Variant}\n}"),
        ("test", "Define a test", "test \"${1:description}\" {\n\t$0\n}"),
        ("middleware", "Define middleware", "middleware ${1:name}(req, next) {\n\t$0\n}"),
        ("env", "Define environment variables", "env {\n\t${1:KEY}: ${2:string}\n}"),
        ("use", "Import from a module", "use { ${1:Name} } from \"${2:path}\""),
    ];

    for (label, doc, snippet) in &keywords {
        items.push(CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some(doc.to_string()),
            insert_text: Some(snippet.to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            documentation: Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format!("```gale\n{snippet}\n```"),
            })),
            ..Default::default()
        });
    }
}

fn add_type_completions(docs: &DocumentManager, items: &mut Vec<CompletionItem>) {
    // Primitive types
    for (ty, desc) in &[
        ("string", "UTF-8 string"),
        ("int", "64-bit integer"),
        ("float", "64-bit floating point"),
        ("bool", "Boolean (true/false)"),
        ("void", "No return value"),
        ("null", "Null value"),
        ("never", "Never returns"),
    ] {
        items.push(CompletionItem {
            label: ty.to_string(),
            kind: Some(CompletionItemKind::TYPE_PARAMETER),
            detail: Some(desc.to_string()),
            ..Default::default()
        });
    }

    // User-defined types from the type checker
    if let Some(ref checker) = docs.cached_checker {
        for name in checker.env.all_type_names() {
            items.push(CompletionItem {
                label: name.to_string(),
                kind: Some(CompletionItemKind::TYPE_PARAMETER),
                detail: Some("type".into()),
                ..Default::default()
            });
        }
        // Guards are also valid types
        for (name, binding) in checker.env.all_visible_bindings() {
            if binding.kind == BindingKind::Guard {
                items.push(CompletionItem {
                    label: name.to_string(),
                    kind: Some(CompletionItemKind::STRUCT),
                    detail: Some("guard".into()),
                    ..Default::default()
                });
            }
        }
    }
}

fn add_validator_completions(items: &mut Vec<CompletionItem>) {
    let validators = [
        ("trim()", "Remove leading/trailing whitespace", "trim()"),
        ("minLen(n)", "Minimum string length", "minLen(${1:1})"),
        ("maxLen(n)", "Maximum string length", "maxLen(${1:255})"),
        ("nonEmpty()", "Must not be empty", "nonEmpty()"),
        ("email()", "Must be a valid email address", "email()"),
        ("url()", "Must be a valid URL", "url()"),
        ("uuid()", "Must be a valid UUID", "uuid()"),
        (
            "regex(pattern)",
            "Must match regex pattern",
            "regex(${1:pattern})",
        ),
        (
            "optional()",
            "Field is optional (allows empty)",
            "optional()",
        ),
        ("nullable()", "Field can be null", "nullable()"),
        ("min(n)", "Minimum numeric value", "min(${1:0})"),
        ("max(n)", "Maximum numeric value", "max(${1:100})"),
        ("positive()", "Must be positive (> 0)", "positive()"),
        (
            "nonNegative()",
            "Must not be negative (>= 0)",
            "nonNegative()",
        ),
        ("integer()", "Must be a whole number", "integer()"),
        ("precision(n)", "Decimal precision", "precision(${1:2})"),
        (
            "oneOf(...)",
            "Must be one of the values",
            "oneOf(${1:values})",
        ),
    ];

    for (label, detail, snippet) in &validators {
        items.push(CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some(detail.to_string()),
            insert_text: Some(snippet.to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
    }
}

fn add_html_tag_completions(items: &mut Vec<CompletionItem>) {
    let tags = [
        ("div", "Generic container element"),
        ("span", "Inline container element"),
        ("p", "Paragraph"),
        ("a", "Hyperlink"),
        ("button", "Clickable button"),
        ("input", "Form input field"),
        ("form", "Form container"),
        ("h1", "Heading level 1"),
        ("h2", "Heading level 2"),
        ("h3", "Heading level 3"),
        ("h4", "Heading level 4"),
        ("h5", "Heading level 5"),
        ("h6", "Heading level 6"),
        ("ul", "Unordered list"),
        ("ol", "Ordered list"),
        ("li", "List item"),
        ("img", "Image (self-closing)"),
        ("video", "Video player"),
        ("audio", "Audio player"),
        ("table", "Table"),
        ("thead", "Table head"),
        ("tbody", "Table body"),
        ("tr", "Table row"),
        ("th", "Table header cell"),
        ("td", "Table data cell"),
        ("nav", "Navigation section"),
        ("header", "Header section"),
        ("footer", "Footer section"),
        ("main", "Main content"),
        ("section", "Generic section"),
        ("article", "Article content"),
        ("label", "Form label"),
        ("select", "Dropdown select"),
        ("option", "Select option"),
        ("textarea", "Multi-line text input"),
        ("slot", "Layout slot (GaleX)"),
        ("pre", "Preformatted text"),
        ("code", "Inline code"),
        ("strong", "Bold/strong text"),
        ("em", "Italic/emphasis"),
    ];

    // Self-closing tags
    let self_closing = ["input", "img", "slot"];

    for (tag, desc) in &tags {
        let snippet = if self_closing.contains(tag) {
            format!("<{tag} $1/>")
        } else {
            format!("<{tag}>$0</{tag}>")
        };
        items.push(CompletionItem {
            label: tag.to_string(),
            kind: Some(CompletionItemKind::PROPERTY),
            detail: Some(desc.to_string()),
            insert_text: Some(snippet),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
    }
}

fn add_component_completions(docs: &DocumentManager, items: &mut Vec<CompletionItem>) {
    if let Some(ref checker) = docs.cached_checker {
        for (name, binding) in checker.env.all_visible_bindings() {
            if binding.kind == BindingKind::Component {
                items.push(CompletionItem {
                    label: name.to_string(),
                    kind: Some(CompletionItemKind::CLASS),
                    detail: Some("component".into()),
                    ..Default::default()
                });
            }
        }
    }
}

fn add_template_keywords(items: &mut Vec<CompletionItem>) {
    let keywords = [
        (
            "when",
            "Conditional rendering",
            "when ${1:condition} {\n\t$0\n}",
        ),
        (
            "each",
            "List rendering",
            "each ${1:item}, ${2:index} in ${3:items} {\n\t$0\n}",
        ),
        ("suspend", "Async loading boundary", "suspend {\n\t$0\n}"),
    ];

    for (label, doc, snippet) in &keywords {
        items.push(CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some(doc.to_string()),
            insert_text: Some(snippet.to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
    }
}

fn add_directive_completions(items: &mut Vec<CompletionItem>) {
    let directives = [
        (
            "bind:value",
            "Two-way binding to a signal or guard field",
            "bind:${1:value}",
        ),
        ("on:click", "Click event handler", "on:click={${1:handler}}"),
        (
            "on:submit",
            "Form submit event handler",
            "on:submit={${1:handler}}",
        ),
        ("on:input", "Input event handler", "on:input={${1:handler}}"),
        (
            "class:active",
            "Conditional CSS class",
            "class:${1:name}={${2:condition}}",
        ),
        ("ref:el", "DOM element reference", "ref:${1:name}"),
        ("transition:fade", "CSS transition", "transition:${1:fade}"),
        ("key", "Keyed list item", "key={${1:id}}"),
        (
            "form:action",
            "Form server action",
            "form:action={${1:actionName}}",
        ),
        (
            "form:guard",
            "Form validation guard",
            "form:guard={${1:GuardName}}",
        ),
        (
            "form:error",
            "Form error display",
            "form:error field=\"${1:fieldName}\"",
        ),
        (
            "prefetch",
            "Link prefetch mode",
            "prefetch=\"${1|hover,intent,viewport|}\"",
        ),
    ];

    for (label, doc, snippet) in &directives {
        items.push(CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::PROPERTY),
            detail: Some(doc.to_string()),
            insert_text: Some(snippet.to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
    }
}

fn add_html_attribute_completions(items: &mut Vec<CompletionItem>) {
    let attrs = [
        "class",
        "id",
        "href",
        "src",
        "alt",
        "type",
        "name",
        "value",
        "placeholder",
        "disabled",
        "style",
        "title",
        "target",
        "rel",
        "width",
        "height",
        "action",
        "method",
        "for",
        "required",
        "readonly",
        "checked",
        "selected",
        "hidden",
        "aria-label",
        "role",
        "data-testid",
    ];

    for attr in &attrs {
        items.push(CompletionItem {
            label: attr.to_string(),
            kind: Some(CompletionItemKind::PROPERTY),
            detail: Some("HTML attribute".into()),
            insert_text: Some(format!("{attr}=\"$1\"")),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
    }
}

fn add_event_completions(items: &mut Vec<CompletionItem>) {
    let events = [
        ("click", "Mouse click"),
        ("dblclick", "Mouse double click"),
        ("mousedown", "Mouse button pressed"),
        ("mouseup", "Mouse button released"),
        ("mouseover", "Mouse entered element"),
        ("mouseout", "Mouse left element"),
        ("mousemove", "Mouse moved over element"),
        ("input", "Input value changed"),
        ("change", "Value committed"),
        ("focus", "Element received focus"),
        ("blur", "Element lost focus"),
        ("submit", "Form submitted"),
        ("keydown", "Key pressed"),
        ("keyup", "Key released"),
        ("keypress", "Key pressed (char)"),
        ("scroll", "Element scrolled"),
        ("resize", "Window resized"),
        ("load", "Resource loaded"),
        ("error", "Resource error"),
        ("touchstart", "Touch started"),
        ("touchend", "Touch ended"),
        ("touchmove", "Touch moved"),
        ("contextmenu", "Right-click menu"),
        ("dragstart", "Drag started"),
        ("dragend", "Drag ended"),
        ("drop", "Element dropped"),
    ];

    for (event, desc) in &events {
        items.push(CompletionItem {
            label: event.to_string(),
            kind: Some(CompletionItemKind::EVENT),
            detail: Some(desc.to_string()),
            insert_text: Some(format!("{event}={{$1}}")),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
    }
}

fn add_binding_completions(docs: &DocumentManager, items: &mut Vec<CompletionItem>) {
    if let Some(ref checker) = docs.cached_checker {
        for (name, binding) in checker.env.all_visible_bindings() {
            let kind = match binding.kind {
                BindingKind::Function | BindingKind::Action => CompletionItemKind::FUNCTION,
                BindingKind::Signal | BindingKind::Derived => CompletionItemKind::VARIABLE,
                BindingKind::Guard => CompletionItemKind::STRUCT,
                BindingKind::Store => CompletionItemKind::MODULE,
                BindingKind::Component => CompletionItemKind::CLASS,
                BindingKind::Channel | BindingKind::Query => CompletionItemKind::INTERFACE,
                BindingKind::TypeAlias | BindingKind::EnumDef => CompletionItemKind::TYPE_PARAMETER,
                _ => CompletionItemKind::VARIABLE,
            };
            let type_str = checker.interner.display(binding.ty);
            let kind_label = format!("{:?}", binding.kind).to_lowercase();
            items.push(CompletionItem {
                label: name.to_string(),
                kind: Some(kind),
                detail: Some(type_str.clone()),
                documentation: Some(Documentation::MarkupContent(MarkupContent {
                    kind: MarkupKind::Markdown,
                    value: format!("```gale\n{kind_label} {name}: {type_str}\n```"),
                })),
                ..Default::default()
            });
        }
    }
}

fn add_boundary_keywords(items: &mut Vec<CompletionItem>) {
    let keywords = [
        (
            "action",
            "Server action",
            "action ${1:name}(${2:data}: ${3:Type}) -> ${4:string} {\n\t$0\n}",
        ),
        (
            "query",
            "Client query",
            "query ${1:name} = \"${2:/api/endpoint}\" -> ${3:Type}",
        ),
        ("fn", "Function", "fn ${1:name}(${2:params}) {\n\t$0\n}"),
        (
            "guard",
            "Validation guard",
            "guard ${1:Name} {\n\t${2:field}: ${3:string}\n}",
        ),
        ("type", "Type alias", "type ${1:Name} = ${2:string}"),
        (
            "enum",
            "Enum definition",
            "enum ${1:Name} {\n\t${2:Variant}\n}",
        ),
    ];

    for (label, doc, snippet) in &keywords {
        items.push(CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some(doc.to_string()),
            insert_text: Some(snippet.to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
    }
}

fn add_component_body_keywords(items: &mut Vec<CompletionItem>) {
    let keywords = [
        ("signal", "Reactive signal", "signal ${1:name} = ${2:value}"),
        ("derive", "Derived value", "derive ${1:name} = ${2:expr}"),
        (
            "frozen",
            "Frozen (constant) binding",
            "frozen ${1:name} = ${2:expr}",
        ),
        ("let", "Immutable binding", "let ${1:name} = ${2:value}"),
        ("mut", "Mutable binding", "mut ${1:name} = ${2:value}"),
        ("ref", "DOM element ref", "ref ${1:name}: HTMLElement"),
        (
            "head",
            "Page metadata block",
            "head {\n\ttitle: \"${1:Title}\"\n}",
        ),
        ("effect", "Reactive side effect", "effect {\n\t$0\n}"),
        (
            "watch",
            "Watch a value for changes",
            "watch ${1:signal} as (next, prev) {\n\t$0\n}",
        ),
        (
            "fn",
            "Helper function",
            "fn ${1:name}(${2:params}) {\n\t$0\n}",
        ),
        (
            "when",
            "Conditional template",
            "when ${1:condition} {\n\t$0\n}",
        ),
        (
            "each",
            "List template",
            "each ${1:item}, ${2:index} in ${3:items} {\n\t$0\n}",
        ),
    ];

    for (label, doc, snippet) in &keywords {
        items.push(CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some(doc.to_string()),
            insert_text: Some(snippet.to_string()),
            insert_text_format: Some(InsertTextFormat::SNIPPET),
            ..Default::default()
        });
    }
}

fn add_code_keywords(items: &mut Vec<CompletionItem>) {
    let keywords = [
        ("let", "Immutable binding"),
        ("mut", "Mutable binding"),
        ("if", "Conditional"),
        ("else", "Else branch"),
        ("for", "Loop"),
        ("return", "Return value"),
        ("await", "Await async"),
        ("fn", "Nested function"),
    ];

    for (label, doc) in &keywords {
        items.push(CompletionItem {
            label: label.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            detail: Some(doc.to_string()),
            ..Default::default()
        });
    }
}
