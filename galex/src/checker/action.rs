//! Action, query, and channel validation (GX0900–GX0913).
//!
//! Validates action/query/channel declarations at the program level:
//! - GX0900: Action is not defined
//! - GX0901: Action parameter guard validation failed
//! - GX0902: Action outside server block
//! - GX0903: Action return type not serializable
//! - GX0904: Duplicate action name
//! - GX0905: Query URL interpolation error (handled in decl.rs)
//! - GX0906: Query return type not deserializable
//! - GX0907: Channel not defined
//! - GX0908: Channel unidirectional — cannot send
//! - GX0909: Channel message type mismatch
//! - GX0910: Channel requires parameters
//! - GX0911: Action called outside client/server context
//! - GX0912: Action has no guard on parameters (warning)
//! - GX0913: Query declared in server block

use crate::ast::*;
use crate::errors::{codes, Diagnostic};
use crate::span::Span;
use std::collections::{HashMap, HashSet};

/// Context for where a declaration appears.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeclContext {
    TopLevel,
    ServerBlock,
    ClientBlock,
    SharedBlock,
}

/// Validate all action/query/channel declarations in a program.
pub fn validate_actions(program: &Program) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut action_names: HashMap<String, Vec<Span>> = HashMap::new();
    let mut query_names: HashMap<String, Vec<Span>> = HashMap::new();

    validate_items(
        &program.items,
        DeclContext::TopLevel,
        &mut action_names,
        &mut query_names,
        &mut diagnostics,
    );

    diagnostics
}

/// Recursively validate items, tracking the declaration context.
fn validate_items(
    items: &[Item],
    context: DeclContext,
    action_names: &mut HashMap<String, Vec<Span>>,
    query_names: &mut HashMap<String, Vec<Span>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for item in items {
        match item {
            Item::ActionDecl(decl) => {
                validate_action_decl(decl, context, action_names, diagnostics);
            }
            Item::QueryDecl(decl) => {
                validate_query_decl(decl, context, query_names, diagnostics);
            }
            Item::ChannelDecl(decl) => {
                validate_channel_decl(decl, context, diagnostics);
            }
            Item::ServerBlock(block) => {
                validate_items(
                    &block.items,
                    DeclContext::ServerBlock,
                    action_names,
                    query_names,
                    diagnostics,
                );
            }
            Item::ClientBlock(block) => {
                validate_items(
                    &block.items,
                    DeclContext::ClientBlock,
                    action_names,
                    query_names,
                    diagnostics,
                );
            }
            Item::SharedBlock(block) => {
                validate_items(
                    &block.items,
                    DeclContext::SharedBlock,
                    action_names,
                    query_names,
                    diagnostics,
                );
            }
            Item::Out(out) => {
                // Check the inner item with the same context
                let inner_items = vec![(*out.inner).clone()];
                validate_items(
                    &inner_items,
                    context,
                    action_names,
                    query_names,
                    diagnostics,
                );
            }
            _ => {}
        }
    }
}

/// Validate a single action declaration.
fn validate_action_decl(
    decl: &ActionDecl,
    context: DeclContext,
    action_names: &mut HashMap<String, Vec<Span>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let name = decl.name.to_string();

    // GX0902: Action must be in a server block (or top-level, which gets server scope)
    if context == DeclContext::ClientBlock || context == DeclContext::SharedBlock {
        diagnostics.push(
            Diagnostic::with_message(
                &codes::GX0902,
                format!(
                    "Action `{}` must be declared inside a `server {{ }}` block",
                    name
                ),
                decl.span,
            )
            .with_hint("actions are server-side RPC endpoints — move to a server { } block"),
        );
    }

    // GX0904: Duplicate action name
    let entry = action_names.entry(name.clone()).or_default();
    if !entry.is_empty() {
        diagnostics.push(
            Diagnostic::with_message(
                &codes::GX0904,
                format!("Duplicate action name `{}`", name),
                decl.span,
            )
            .with_hint("each action must have a unique name within the file"),
        );
    }
    entry.push(decl.span);

    // GX0912: Action has no guard on parameters (warning)
    if !decl.params.is_empty() {
        let has_guard_type = decl.params.iter().any(|p| {
            if let Some(TypeAnnotation::Named { name, .. }) = &p.ty_ann {
                // Guard names are typically PascalCase
                name.chars().next().map_or(false, |c| c.is_uppercase())
            } else {
                false
            }
        });
        if !has_guard_type {
            diagnostics.push(
                Diagnostic::with_message(
                    &codes::GX0912,
                    format!(
                        "Action `{}` accepts parameters but uses no guard for validation",
                        name
                    ),
                    decl.span,
                )
                .with_hint("consider using a guard type for automatic input validation"),
            );
        }
    }

    // GX0903: Action return type serializable check
    // If a return type annotation exists, check it's not obviously non-serializable
    if let Some(ret_ty) = &decl.ret_ty {
        if is_obviously_non_serializable(ret_ty) {
            diagnostics.push(
                Diagnostic::with_message(
                    &codes::GX0903,
                    format!("Action `{}` return type is not serializable", name),
                    decl.span,
                )
                .with_hint("action return values must be JSON-serializable for the client"),
            );
        }
    }
}

/// Validate a single query declaration.
fn validate_query_decl(
    decl: &QueryDecl,
    context: DeclContext,
    query_names: &mut HashMap<String, Vec<Span>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let name = decl.name.to_string();

    // GX0913: Query declared in server block
    if context == DeclContext::ServerBlock {
        diagnostics.push(
            Diagnostic::with_message(
                &codes::GX0913,
                format!("Query `{}` is declared in a server block", name),
                decl.span,
            )
            .with_hint("queries are client-side data fetching — use direct DB calls on the server"),
        );
    }

    // GX0906: Query return type not deserializable
    if let Some(ret_ty) = &decl.ret_ty {
        if is_obviously_non_serializable(ret_ty) {
            diagnostics.push(
                Diagnostic::with_message(
                    &codes::GX0906,
                    format!(
                        "Query `{}` return type is not deserializable from JSON",
                        name
                    ),
                    decl.span,
                )
                .with_hint("query return types must be JSON-deserializable"),
            );
        }
    }

    // Duplicate query name detection (reusing the pattern)
    let entry = query_names.entry(name.clone()).or_default();
    if !entry.is_empty() {
        // No specific code for duplicate query — report as general issue
        // (the type checker handles this via GX0327)
    }
    entry.push(decl.span);
}

/// Validate a channel declaration.
fn validate_channel_decl(
    decl: &ChannelDecl,
    context: DeclContext,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let name = decl.name.to_string();

    // Channels in client blocks were already caught by boundary.rs (GX0513).
    // Channels in shared blocks are also invalid.
    if context == DeclContext::SharedBlock {
        diagnostics.push(
            Diagnostic::with_message(
                &codes::GX0902,
                format!(
                    "Channel `{}` must be declared inside a `server {{ }}` block",
                    name
                ),
                decl.span,
            )
            .with_hint("channels are server-defined — move to a server { } block"),
        );
    }
}

/// Check if an action was called from outside client or server context (GX0911).
pub fn check_action_call_context(
    action_name: &str,
    context: DeclContext,
    span: Span,
) -> Option<Diagnostic> {
    if context == DeclContext::SharedBlock {
        Some(
            Diagnostic::with_message(
                &codes::GX0911,
                format!(
                    "Action `{}` called outside client or server context",
                    action_name
                ),
                span,
            )
            .with_hint("actions can only be called from server or client blocks"),
        )
    } else {
        None
    }
}

/// Check that a referenced action exists (GX0900).
pub fn check_action_exists(
    action_name: &str,
    known_actions: &HashSet<String>,
    span: Span,
) -> Option<Diagnostic> {
    if !known_actions.contains(action_name) {
        Some(
            Diagnostic::with_message(
                &codes::GX0900,
                format!("Action `{}` is not defined", action_name),
                span,
            )
            .with_hint("declare this action in a server { } block"),
        )
    } else {
        None
    }
}

/// Check that a referenced channel exists (GX0907).
pub fn check_channel_exists(
    channel_name: &str,
    known_channels: &HashSet<String>,
    span: Span,
) -> Option<Diagnostic> {
    if !known_channels.contains(channel_name) {
        Some(
            Diagnostic::with_message(
                &codes::GX0907,
                format!("Channel `{}` is not defined", channel_name),
                span,
            )
            .with_hint("declare this channel in a server { } block"),
        )
    } else {
        None
    }
}

/// Check that a unidirectional channel does not attempt `.send()` (GX0908).
pub fn check_channel_send_direction(
    channel_name: &str,
    direction: &ChannelDirection,
    span: Span,
) -> Option<Diagnostic> {
    if *direction == ChannelDirection::ServerToClient {
        Some(
            Diagnostic::with_message(
                &codes::GX0908,
                format!(
                    "Channel `{}` is unidirectional (server -> client) — cannot `.send()` from client",
                    channel_name
                ),
                span,
            )
            .with_hint("use `<->` for bidirectional channels"),
        )
    } else {
        None
    }
}

/// Check that a channel requiring parameters is subscribed with arguments (GX0910).
pub fn check_channel_params(
    channel_name: &str,
    has_params: bool,
    args_provided: bool,
    span: Span,
) -> Option<Diagnostic> {
    if has_params && !args_provided {
        Some(
            Diagnostic::with_message(
                &codes::GX0910,
                format!(
                    "Channel `{}` requires parameters but none were provided",
                    channel_name
                ),
                span,
            )
            .with_hint("provide arguments when subscribing to this channel"),
        )
    } else {
        None
    }
}

/// Check for obviously non-serializable type annotations.
///
/// This is a syntactic check — the full serialization check happens in
/// the type checker. Here we catch function types and known
/// non-serializable patterns.
fn is_obviously_non_serializable(ann: &TypeAnnotation) -> bool {
    match ann {
        TypeAnnotation::Function { .. } => true,
        TypeAnnotation::Array { element, .. } => is_obviously_non_serializable(element),
        TypeAnnotation::Union { types, .. } => types.iter().any(is_obviously_non_serializable),
        TypeAnnotation::Optional { inner, .. } => is_obviously_non_serializable(inner),
        TypeAnnotation::Named { name, .. } => {
            // Known non-serializable built-in types
            matches!(
                name.as_str(),
                "HTMLElement"
                    | "HTMLCanvasElement"
                    | "HTMLInputElement"
                    | "Event"
                    | "MouseEvent"
                    | "KeyboardEvent"
            )
        }
        _ => false,
    }
}
