//! Store system validation (GX1000–GX1006).
//!
//! Validates store declarations:
//! - GX1000: Store is not defined
//! - GX1001: Store field does not exist
//! - GX1002: Cannot mutate store signal outside store methods
//! - GX1003: Duplicate store name
//! - GX1004: Store contains `action`
//! - GX1005: Store contains `query`
//! - GX1006: Store has no signals (warning)

use crate::ast::*;
use crate::errors::{codes, Diagnostic};
use crate::span::Span;
use std::collections::{HashMap, HashSet};

/// Validate all store declarations in a program.
pub fn validate_stores(program: &Program) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut store_names: HashMap<String, Vec<Span>> = HashMap::new();

    validate_items_for_stores(&program.items, &mut store_names, &mut diagnostics);

    diagnostics
}

/// Recursively walk items to find and validate store declarations.
fn validate_items_for_stores(
    items: &[Item],
    store_names: &mut HashMap<String, Vec<Span>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for item in items {
        match item {
            Item::StoreDecl(decl) => {
                validate_store_decl(decl, store_names, diagnostics);
            }
            Item::ServerBlock(block) | Item::ClientBlock(block) | Item::SharedBlock(block) => {
                validate_items_for_stores(&block.items, store_names, diagnostics);
            }
            Item::Out(out) => {
                let inner_items = vec![(*out.inner).clone()];
                validate_items_for_stores(&inner_items, store_names, diagnostics);
            }
            _ => {}
        }
    }
}

/// Validate a single store declaration.
fn validate_store_decl(
    decl: &StoreDecl,
    store_names: &mut HashMap<String, Vec<Span>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let name = decl.name.to_string();

    // GX1003: Duplicate store name
    let entry = store_names.entry(name.clone()).or_default();
    if !entry.is_empty() {
        diagnostics.push(
            Diagnostic::with_message(
                &codes::GX1003,
                format!("Duplicate store name `{}`", name),
                decl.span,
            )
            .with_hint("each store must have a unique name"),
        );
    }
    entry.push(decl.span);

    let mut has_signal = false;
    let mut has_action_inside = false;
    let mut has_query_inside = false;

    for member in &decl.members {
        match member {
            StoreMember::Signal(_) => {
                has_signal = true;
            }
            StoreMember::Derive(_) => {
                // derives are fine in stores
            }
            StoreMember::Method(fn_decl) => {
                // GX1004: Check if method body contains action-like patterns
                if is_action_keyword_in_block(&fn_decl.body) {
                    has_action_inside = true;
                }
                // GX1005: Check if method body contains query-like patterns
                if is_query_keyword_in_block(&fn_decl.body) {
                    has_query_inside = true;
                }
            }
        }
    }

    // GX1004: Store contains `action`
    if has_action_inside {
        diagnostics.push(
            Diagnostic::with_message(
                &codes::GX1004,
                format!("Store `{}` contains action declarations", name),
                decl.span,
            )
            .with_hint("actions are server concepts — store methods should use `fn` instead"),
        );
    }

    // GX1005: Store contains `query`
    if has_query_inside {
        diagnostics.push(
            Diagnostic::with_message(
                &codes::GX1005,
                format!("Store `{}` contains query declarations", name),
                decl.span,
            )
            .with_hint(
                "queries belong in components, not stores — stores hold derived/local state",
            ),
        );
    }

    // GX1006: Store has no signals (warning)
    if !has_signal {
        diagnostics.push(
            Diagnostic::with_message(
                &codes::GX1006,
                format!("Store `{}` has no signal declarations", name),
                decl.span,
            )
            .with_hint(
                "a store without signals isn't reactive — consider using plain functions instead",
            ),
        );
    }
}

/// Check that a referenced store exists (GX1000).
pub fn check_store_exists(
    store_name: &str,
    known_stores: &HashSet<String>,
    span: Span,
) -> Option<Diagnostic> {
    if !known_stores.contains(store_name) {
        Some(
            Diagnostic::with_message(
                &codes::GX1000,
                format!("Store `{}` is not defined", store_name),
                span,
            )
            .with_hint("declare this store with `store Name { ... }`"),
        )
    } else {
        None
    }
}

/// Check that a store field/signal/method exists (GX1001).
pub fn check_store_field(
    store_name: &str,
    field_name: &str,
    known_fields: &HashSet<String>,
    span: Span,
) -> Option<Diagnostic> {
    if !known_fields.contains(field_name) {
        Some(
            Diagnostic::with_message(
                &codes::GX1001,
                format!("Store `{}` has no field `{}`", store_name, field_name),
                span,
            )
            .with_hint("check the store's signals, derives, and methods"),
        )
    } else {
        None
    }
}

/// Check that a store signal is not mutated from outside (GX1002).
pub fn check_store_signal_mutation(
    store_name: &str,
    signal_name: &str,
    is_inside_store_method: bool,
    span: Span,
) -> Option<Diagnostic> {
    if !is_inside_store_method {
        Some(
            Diagnostic::with_message(
                &codes::GX1002,
                format!(
                    "Cannot mutate store signal `{}.{}` outside store methods",
                    store_name, signal_name
                ),
                span,
            )
            .with_hint("use a store method to modify signals: `storeName.methodName()`"),
        )
    } else {
        None
    }
}

/// Scan a block for action-like declarations (heuristic).
///
/// Since store members are declared as `StoreMember::Method`, we check
/// if the method name or body contains patterns that suggest an action
/// was accidentally placed in a store.
fn is_action_keyword_in_block(block: &Block) -> bool {
    for stmt in &block.stmts {
        if let Stmt::ExprStmt { expr, .. } = stmt {
            // Check for calls to functions named "action" or similar patterns
            if let Expr::FnCall { callee, .. } = expr {
                if let Expr::Ident { name, .. } = callee.as_ref() {
                    if name.as_str() == "action" {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Scan a block for query-like declarations (heuristic).
fn is_query_keyword_in_block(block: &Block) -> bool {
    for stmt in &block.stmts {
        if let Stmt::ExprStmt { expr, .. } = stmt {
            if let Expr::FnCall { callee, .. } = expr {
                if let Expr::Ident { name, .. } = callee.as_ref() {
                    if name.as_str() == "query" {
                        return true;
                    }
                }
            }
        }
    }
    false
}
