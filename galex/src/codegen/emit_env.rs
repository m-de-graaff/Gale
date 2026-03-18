#![allow(clippy::collapsible_match)]
//! Environment variable module generator.
//!
//! For each GaleX [`EnvDecl`], generates a `src/env_config.rs` Rust file
//! containing a typed `Env` struct, startup loading via `dotenvy`, guard-style
//! validation with fail-fast semantics, and a `public_vars_json()` method
//! for embedding `PUBLIC_*` vars in SSR output.

use std::collections::HashSet;

use crate::ast::*;
use crate::codegen::emit_expr::emit_expr;
use crate::codegen::rust_emitter::RustEmitter;

/// Convert an env var key to a Rust field name (lowercase SCREAMING_CASE).
fn env_key_to_field_name(key: &str) -> String {
    key.to_lowercase()
}

/// Emit the complete `src/env_config.rs` file from an EnvDecl.
pub fn emit_env_file(e: &mut RustEmitter, decl: &EnvDecl) {
    e.emit_file_header("Environment configuration.");
    e.newline();

    e.emit_use("std::sync::LazyLock");
    e.newline();

    // Struct
    emit_env_struct(e, decl);
    e.newline();

    // impl Env
    e.block("impl Env", |e| {
        // load()
        emit_load_fn(e, decl);
        e.newline();

        // validate()
        emit_validate_fn(e, decl);
        e.newline();

        // public_vars_json()
        emit_public_vars_json_fn(e, decl);
    });
    e.newline();

    // Static singleton
    emit_static_singleton(e);
}

/// Return the Rust expression to emit for an `env.KEY` access.
///
/// If `declared_keys` contains the key, returns a typed accessor through
/// the static `ENV` singleton. Otherwise falls back to `std::env::var()`.
pub fn env_access_expr(key: &str, declared_keys: &HashSet<String>) -> String {
    let field = env_key_to_field_name(key);
    if declared_keys.contains(key) {
        format!("crate::env_config::ENV.{field}")
    } else {
        format!("std::env::var({:?}).unwrap_or_default()", key)
    }
}

// ── Struct emission ────────────────────────────────────────────────────

fn emit_env_struct(e: &mut RustEmitter, decl: &EnvDecl) {
    e.emit_doc_comment("Typed, validated environment variables.");
    e.emit_attribute("derive(Debug)");
    e.block("pub struct Env", |e| {
        for var in &decl.vars {
            let field = env_key_to_field_name(&var.key);
            let rust_ty = annotation_to_env_type(&var.ty);
            e.writeln(&format!("pub {field}: {rust_ty},"));
        }
    });
}

// ── load() emission ────────────────────────────────────────────────────

fn emit_load_fn(e: &mut RustEmitter, decl: &EnvDecl) {
    e.emit_doc_comment("Load env vars from the system environment.");
    e.block("pub fn load() -> Self", |e| {
        e.writeln("Self {");
        e.indent();
        for var in &decl.vars {
            let field = env_key_to_field_name(&var.key);
            let key = &var.key;
            let rust_ty = annotation_to_env_type(&var.ty);

            match rust_ty {
                "String" => {
                    if let Some(ref default_expr) = var.default {
                        let mut de = RustEmitter::new();
                        emit_expr(&mut de, default_expr);
                        let default_str = de.finish();
                        e.writeln(&format!(
                            "{field}: std::env::var({key:?}).unwrap_or_else(|_| {default_str}),",
                        ));
                    } else {
                        e.writeln(&format!(
                            "{field}: std::env::var({key:?}).unwrap_or_default(),",
                        ));
                    }
                }
                "i64" => {
                    let default_val = if let Some(ref default_expr) = var.default {
                        if let Expr::IntLit { value, .. } = default_expr {
                            format!("{value}")
                        } else {
                            "0".into()
                        }
                    } else {
                        "0".into()
                    };
                    e.writeln(&format!(
                        "{field}: std::env::var({key:?}).unwrap_or_default().parse::<i64>().unwrap_or({default_val}),",
                    ));
                }
                "f64" => {
                    let default_val = if let Some(ref default_expr) = var.default {
                        if let Expr::FloatLit { value, .. } = default_expr {
                            format!("{value}")
                        } else {
                            "0.0".into()
                        }
                    } else {
                        "0.0".into()
                    };
                    e.writeln(&format!(
                        "{field}: std::env::var({key:?}).unwrap_or_default().parse::<f64>().unwrap_or({default_val}),",
                    ));
                }
                "bool" => {
                    let default_val = if let Some(ref default_expr) = var.default {
                        if let Expr::BoolLit { value, .. } = default_expr {
                            format!("{value}")
                        } else {
                            "false".into()
                        }
                    } else {
                        "false".into()
                    };
                    e.writeln(&format!(
                        "{field}: std::env::var({key:?}).unwrap_or_default().parse::<bool>().unwrap_or({default_val}),",
                    ));
                }
                _ => {
                    e.writeln(&format!(
                        "{field}: std::env::var({key:?}).unwrap_or_default(),",
                    ));
                }
            }
        }
        e.dedent();
        e.writeln("}");
    });
}

// ── validate() emission ────────────────────────────────────────────────

fn emit_validate_fn(e: &mut RustEmitter, decl: &EnvDecl) {
    e.emit_doc_comment("Validate all env vars against their declared constraints.");
    e.block("pub fn validate(&self) -> Result<(), Vec<String>>", |e| {
        e.writeln("let mut errors = Vec::new();");

        for var in &decl.vars {
            let field = env_key_to_field_name(&var.key);
            let key = &var.key;
            let rust_ty = annotation_to_env_type(&var.ty);

            for v in &var.validators {
                emit_validator_check(e, &field, key, rust_ty, v);
            }
        }

        e.block("if errors.is_empty()", |e| {
            e.writeln("Ok(())");
        });
        e.block("else", |e| {
            e.writeln("Err(errors)");
        });
    });
}

fn emit_validator_check(
    e: &mut RustEmitter,
    field: &str,
    key: &str,
    rust_ty: &str,
    validator: &ValidatorCall,
) {
    match validator.name.as_str() {
        "nonEmpty" => {
            if rust_ty == "String" {
                e.block(&format!("if self.{field}.is_empty()"), |e| {
                    e.writeln(&format!(
                        "errors.push(\"{key}: must not be empty\".into());"
                    ));
                });
            }
        }
        "min" => {
            if let Some(Expr::IntLit { value, .. }) = validator.args.first() {
                e.block(&format!("if self.{field} < {value}"), |e| {
                    e.writeln(&format!(
                        "errors.push(\"{key}: must be at least {value}\".into());"
                    ));
                });
            }
        }
        "max" => {
            if let Some(Expr::IntLit { value, .. }) = validator.args.first() {
                e.block(&format!("if self.{field} > {value}"), |e| {
                    e.writeln(&format!(
                        "errors.push(\"{key}: must be at most {value}\".into());"
                    ));
                });
            }
        }
        "minLen" => {
            if let Some(Expr::IntLit { value, .. }) = validator.args.first() {
                e.block(&format!("if self.{field}.len() < {value}"), |e| {
                    e.writeln(&format!(
                        "errors.push(\"{key}: must be at least {value} characters\".into());"
                    ));
                });
            }
        }
        "maxLen" => {
            if let Some(Expr::IntLit { value, .. }) = validator.args.first() {
                e.block(&format!("if self.{field}.len() > {value}"), |e| {
                    e.writeln(&format!(
                        "errors.push(\"{key}: must be at most {value} characters\".into());"
                    ));
                });
            }
        }
        "url" => {
            e.block(&format!("if !self.{field}.starts_with(\"http\")"), |e| {
                e.writeln(&format!(
                    "errors.push(\"{key}: must be a valid URL\".into());"
                ));
            });
        }
        "email" => {
            e.block(&format!("if !self.{field}.contains('@')"), |e| {
                e.writeln(&format!(
                    "errors.push(\"{key}: must be a valid email address\".into());"
                ));
            });
        }
        "oneOf" => {
            let variants: Vec<String> = validator
                .args
                .iter()
                .filter_map(|a| {
                    if let Expr::StringLit { value, .. } = a {
                        Some(format!("{:?}", value.as_str()))
                    } else {
                        None
                    }
                })
                .collect();
            if !variants.is_empty() {
                let list = variants.join(", ");
                e.block(
                    &format!("if ![{list}].contains(&self.{field}.as_str())"),
                    |e| {
                        e.writeln(&format!(
                            "errors.push(\"{key}: must be one of [{list}]\".into());"
                        ));
                    },
                );
            }
        }
        _ => {
            // Unknown validator — emit a comment
            e.emit_comment(&format!("TODO: validator '{}' for {key}", validator.name));
        }
    }
}

// ── public_vars_json() emission ────────────────────────────────────────

fn emit_public_vars_json_fn(e: &mut RustEmitter, decl: &EnvDecl) {
    let public_vars: Vec<&EnvVarDef> = decl
        .vars
        .iter()
        .filter(|v| v.key.starts_with("PUBLIC_"))
        .collect();

    e.emit_doc_comment("Serialize PUBLIC_ vars as JSON for client-side embedding.");
    e.block("pub fn public_vars_json(&self) -> String", |e| {
        if public_vars.is_empty() {
            e.writeln("String::from(\"{}\")");
        } else {
            e.writeln("serde_json::json!({");
            e.indent();
            for var in &public_vars {
                let field = env_key_to_field_name(&var.key);
                e.writeln(&format!("{:?}: self.{field},", var.key.as_str()));
            }
            e.dedent();
            e.writeln("}).to_string()");
        }
    });
}

// ── Static singleton emission ──────────────────────────────────────────

fn emit_static_singleton(e: &mut RustEmitter) {
    e.emit_doc_comment("Singleton — loaded once at startup, validated, exits on failure.");
    e.block("pub static ENV: LazyLock<Env> = LazyLock::new(||", |e| {
        e.writeln("dotenvy::dotenv().ok();");
        e.writeln("let env = Env::load();");
        e.block("env.validate().unwrap_or_else(|errors|", |e| {
            e.writeln("eprintln!(\"Environment validation failed:\");");
            e.block("for e in &errors", |e| {
                e.writeln("eprintln!(\"  - {e}\");");
            });
            e.writeln("std::process::exit(1);");
        });
        e.writeln("env");
    });
    // Close the LazyLock::new() call
    e.writeln(");");
}

// ── Helpers ────────────────────────────────────────────────────────────

/// Map a GaleX type annotation to the Rust type for env var fields.
fn annotation_to_env_type(ann: &TypeAnnotation) -> &'static str {
    if let TypeAnnotation::Named { name, .. } = ann {
        match name.as_str() {
            "string" => "String",
            "int" => "i64",
            "float" => "f64",
            "bool" => "bool",
            _ => "String", // fallback
        }
    } else {
        "String"
    }
}
