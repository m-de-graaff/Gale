//! Env system validation (GX1100–GX1108).
//!
//! Validates environment variable declarations:
//! - GX1100: Required env variable is not set
//! - GX1101: Env variable failed validation
//! - GX1102: Client env must start with GALE_PUBLIC_
//! - GX1103: Server env has GALE_PUBLIC_ prefix
//! - GX1104: Duplicate env variable
//! - GX1105: Env variable not declared in env.gx
//! - GX1106: Env with no validation chain (warning)
//! - GX1107: Env accessed outside server or client context
//! - GX1108: No env.gx file found

use crate::ast::*;
use crate::errors::{codes, Diagnostic};
use crate::span::Span;
use std::collections::{HashMap, HashSet};

/// Context of an env var declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EnvSection {
    /// Declared at top level (no specific section)
    TopLevel,
    /// Inside a `server { }` block
    Server,
    /// Inside a `client { }` block
    Client,
}

/// Validate all env declarations in a program.
pub fn validate_env_decls(program: &Program) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    // Track all declared env var names across all sections
    let mut all_env_vars: HashMap<String, Vec<(Span, EnvSection)>> = HashMap::new();

    validate_env_items(
        &program.items,
        EnvSection::TopLevel,
        &mut all_env_vars,
        &mut diagnostics,
    );

    // GX1104: Check for duplicate env variables across sections
    for (key, locations) in &all_env_vars {
        if locations.len() > 1 {
            // Check if the same var appears in both server and client sections
            let has_server = locations.iter().any(|(_, s)| *s == EnvSection::Server);
            let has_client = locations.iter().any(|(_, s)| *s == EnvSection::Client);
            if has_server && has_client {
                // Report on the second occurrence
                if let Some(&(span, _)) = locations.get(1) {
                    diagnostics.push(
                        Diagnostic::with_message(
                            &codes::GX1104,
                            format!(
                                "Duplicate env variable `{}` — appears in both server and client sections",
                                key
                            ),
                            span,
                        )
                        .with_hint("an env variable should be in either server or client section, not both"),
                    );
                }
            } else if locations.len() > 1 {
                // Same section, still a duplicate
                if let Some(&(span, _)) = locations.get(1) {
                    diagnostics.push(
                        Diagnostic::with_message(
                            &codes::GX1104,
                            format!("Duplicate env variable `{}`", key),
                            span,
                        )
                        .with_hint("each env variable should be declared only once"),
                    );
                }
            }
        }
    }

    diagnostics
}

/// Recursively walk items to find and validate env declarations.
fn validate_env_items(
    items: &[Item],
    section: EnvSection,
    all_env_vars: &mut HashMap<String, Vec<(Span, EnvSection)>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for item in items {
        match item {
            Item::EnvDecl(decl) => {
                validate_env_decl(decl, section, all_env_vars, diagnostics);
            }
            Item::ServerBlock(block) => {
                validate_env_items(&block.items, EnvSection::Server, all_env_vars, diagnostics);
            }
            Item::ClientBlock(block) => {
                validate_env_items(&block.items, EnvSection::Client, all_env_vars, diagnostics);
            }
            Item::SharedBlock(block) => {
                // Env in shared block is unusual — validated by boundary.rs
                validate_env_items(
                    &block.items,
                    EnvSection::TopLevel,
                    all_env_vars,
                    diagnostics,
                );
            }
            Item::Out(out) => {
                let inner_items = vec![(*out.inner).clone()];
                validate_env_items(&inner_items, section, all_env_vars, diagnostics);
            }
            _ => {}
        }
    }
}

/// Validate a single env declaration block.
fn validate_env_decl(
    decl: &EnvDecl,
    section: EnvSection,
    all_env_vars: &mut HashMap<String, Vec<(Span, EnvSection)>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for var in &decl.vars {
        let key = var.key.to_string();

        // GX1102: Client env must start with GALE_PUBLIC_
        if section == EnvSection::Client && !key.starts_with("GALE_PUBLIC_") {
            diagnostics.push(
                Diagnostic::with_message(
                    &codes::GX1102,
                    format!(
                        "Client env variable `{}` must start with `GALE_PUBLIC_`",
                        key
                    ),
                    var.span,
                )
                .with_hint("rename to `GALE_PUBLIC_{}` or move to a server { } section"),
            );
        }

        // GX1103: Server env has GALE_PUBLIC_ prefix
        if section == EnvSection::Server && key.starts_with("GALE_PUBLIC_") {
            diagnostics.push(
                Diagnostic::with_message(
                    &codes::GX1103,
                    format!("Server env variable `{}` has `GALE_PUBLIC_` prefix", key),
                    var.span,
                )
                .with_hint("move to a client { } section or remove the `GALE_PUBLIC_` prefix"),
            );
        }

        // GX1106: Env with no validation chain (warning)
        if var.validators.is_empty() {
            diagnostics.push(
                Diagnostic::with_message(
                    &codes::GX1106,
                    format!("Env variable `{}` has no validation chain", key),
                    var.span,
                )
                .with_hint("consider adding constraints like `.nonEmpty()` or `.min(1)`"),
            );
        }

        // Track for duplicate detection
        all_env_vars
            .entry(key)
            .or_default()
            .push((var.span, section));
    }
}

/// Check that an env variable is declared (GX1105).
pub fn check_env_declared(
    key: &str,
    declared_vars: &HashSet<String>,
    span: Span,
) -> Option<Diagnostic> {
    if !declared_vars.contains(key) {
        Some(
            Diagnostic::with_message(
                &codes::GX1105,
                format!("Env variable `{}` is not declared in `env.gx`", key),
                span,
            )
            .with_hint("add this variable to your env declaration block"),
        )
    } else {
        None
    }
}

/// Check env access scope (GX1107).
pub fn check_env_access_scope(
    key: &str,
    is_in_shared_scope: bool,
    span: Span,
) -> Option<Diagnostic> {
    if is_in_shared_scope {
        Some(
            Diagnostic::with_message(
                &codes::GX1107,
                format!(
                    "Env variable `{}` accessed outside server or client context",
                    key
                ),
                span,
            )
            .with_hint("env variables have scope — access them from server or client blocks"),
        )
    } else {
        None
    }
}

/// Check that an env.gx file exists when env variables are accessed (GX1108).
pub fn check_env_file_exists(has_env_file: bool, span: Span) -> Option<Diagnostic> {
    if !has_env_file {
        Some(
            Diagnostic::with_message(
                &codes::GX1108,
                "No `env.gx` file found but env variables are accessed",
                span,
            )
            .with_hint("create an `env.gx` file or `out env { }` block to declare env variables"),
        )
    } else {
        None
    }
}

/// Check that a required env variable has a value at build time (GX1100).
pub fn check_required_env_set(
    key: &str,
    has_value: bool,
    has_default: bool,
    span: Span,
) -> Option<Diagnostic> {
    if !has_value && !has_default {
        Some(
            Diagnostic::with_message(
                &codes::GX1100,
                format!("Required env variable `{}` is not set", key),
                span,
            )
            .with_hint("set this variable in your environment or provide a default value"),
        )
    } else {
        None
    }
}

/// Check that an env variable passes its validation (GX1101).
pub fn check_env_validation(key: &str, validation_error: &str, span: Span) -> Diagnostic {
    Diagnostic::with_message(
        &codes::GX1101,
        format!(
            "Env variable `{}` failed validation: {}",
            key, validation_error
        ),
        span,
    )
    .with_hint("fix the variable's value or adjust the validation constraints")
}
