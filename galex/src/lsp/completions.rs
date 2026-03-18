//! Autocomplete provider — context-aware completions.

use lsp_types::{CompletionItem, CompletionItemKind};

use super::document::DocumentManager;
use crate::types::env::BindingKind;

/// Provide completion items at the given position.
pub fn provide_completions(docs: &DocumentManager) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    // Add visible bindings from the type environment
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
            items.push(CompletionItem {
                label: name.to_string(),
                kind: Some(kind),
                detail: Some(type_str),
                ..Default::default()
            });
        }

        // Add type names
        for name in checker.env.all_type_names() {
            items.push(CompletionItem {
                label: name.to_string(),
                kind: Some(CompletionItemKind::TYPE_PARAMETER),
                detail: Some("type".into()),
                ..Default::default()
            });
        }
    }

    // Add keywords
    for kw in COMMON_KEYWORDS {
        items.push(CompletionItem {
            label: kw.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            ..Default::default()
        });
    }

    // Add HTML tags (for template context)
    for tag in COMMON_HTML_TAGS {
        items.push(CompletionItem {
            label: tag.to_string(),
            kind: Some(CompletionItemKind::PROPERTY),
            detail: Some("HTML element".into()),
            ..Default::default()
        });
    }

    // Add validator methods (for guard chain context)
    for (name, desc) in VALIDATOR_METHODS {
        items.push(CompletionItem {
            label: name.to_string(),
            kind: Some(CompletionItemKind::METHOD),
            detail: Some(desc.to_string()),
            ..Default::default()
        });
    }

    // Add event names (for on: directive context)
    for event in COMMON_EVENTS {
        items.push(CompletionItem {
            label: event.to_string(),
            kind: Some(CompletionItemKind::EVENT),
            detail: Some("DOM event".into()),
            ..Default::default()
        });
    }

    items
}

const COMMON_KEYWORDS: &[&str] = &[
    "let",
    "mut",
    "signal",
    "derive",
    "frozen",
    "ref",
    "fn",
    "return",
    "if",
    "else",
    "for",
    "await",
    "guard",
    "action",
    "query",
    "store",
    "channel",
    "effect",
    "watch",
    "when",
    "each",
    "suspend",
    "server",
    "client",
    "shared",
    "use",
    "out",
    "type",
    "enum",
    "test",
    "env",
    "middleware",
];

const COMMON_HTML_TAGS: &[&str] = &[
    "div", "span", "p", "a", "button", "input", "form", "h1", "h2", "h3", "h4", "h5", "h6", "ul",
    "ol", "li", "img", "video", "audio", "table", "thead", "tbody", "tr", "th", "td", "nav",
    "header", "footer", "main", "section", "article", "label", "select", "option", "textarea",
    "slot",
];

const VALIDATOR_METHODS: &[(&str, &str)] = &[
    ("email()", "must be a valid email address"),
    ("url()", "must be a valid URL"),
    ("uuid()", "must be a valid UUID"),
    ("min(n)", "minimum numeric value"),
    ("max(n)", "maximum numeric value"),
    ("minLen(n)", "minimum string length"),
    ("maxLen(n)", "maximum string length"),
    ("nonEmpty()", "must not be empty"),
    ("trim()", "trim whitespace"),
    ("optional()", "field is optional"),
    ("nullable()", "field can be null"),
    ("oneOf(...)", "must be one of the values"),
    ("regex(pattern)", "must match regex pattern"),
    ("positive()", "must be positive"),
    ("nonNegative()", "must not be negative"),
    ("integer()", "must be a whole number"),
    ("precision(n)", "decimal precision"),
];

const COMMON_EVENTS: &[&str] = &[
    "click",
    "dblclick",
    "mousedown",
    "mouseup",
    "mouseover",
    "mouseout",
    "input",
    "change",
    "focus",
    "blur",
    "submit",
    "keydown",
    "keyup",
    "keypress",
    "scroll",
    "resize",
    "load",
    "error",
    "touchstart",
    "touchend",
    "touchmove",
];
