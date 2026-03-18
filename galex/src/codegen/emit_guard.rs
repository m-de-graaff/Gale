//! Guard struct and validator generation.
//!
//! For each GaleX [`GuardDecl`], generates:
//! - A Rust struct with `#[derive(Serialize, Deserialize)]`
//! - `impl Validate for Guard` — runtime constraint checks
//! - `impl Sanitize for Guard` — input transforms (trim, precision, default)
//! - `LazyLock<Regex>` statics for regex-based validators

use std::collections::{BTreeSet, HashSet};

use crate::ast::*;
use crate::codegen::emit_stmt::annotation_to_rust;
use crate::codegen::rust_emitter::RustEmitter;
use crate::codegen::types::{collect_shared_type_refs, to_module_name, to_snake_case};

// ── Public entry point ─────────────────────────────────────────────────

/// Emit a complete guard Rust file.
pub fn emit_guard_file(
    e: &mut RustEmitter,
    decl: &GuardDecl,
    known_shared_types: &HashSet<String>,
) {
    e.emit_file_header(&format!("Guard validator: `{}`.", decl.name));
    e.newline();

    let has_transforms = decl.fields.iter().any(|f| {
        f.validators
            .iter()
            .any(|v| matches!(v.name.as_str(), "trim" | "precision" | "default"))
    });

    // Collect all regex patterns needed so we can emit LazyLock statics
    let regex_patterns = collect_regex_patterns(decl);
    let needs_regex = !regex_patterns.is_empty();

    // ── Imports ────────────────────────────────────────────────
    e.emit_use("serde::{Deserialize, Serialize}");
    e.emit_use("crate::shared::validation::{ValidationError, Validate}");
    if has_transforms {
        e.emit_use("crate::shared::validation::Sanitize");
    }
    if needs_regex {
        e.emit_use("std::sync::LazyLock");
    }

    // Shared type imports (enums, type aliases referenced in field annotations)
    if !known_shared_types.is_empty() {
        let annotations: Vec<&TypeAnnotation> = decl.fields.iter().map(|f| &f.ty).collect();
        for name in collect_shared_type_refs(&annotations, known_shared_types) {
            let mod_name = to_module_name(&name);
            e.emit_use(&format!("crate::shared::{mod_name}::{name}"));
        }
    }
    e.newline();

    // ── Regex statics ──────────────────────────────────────────
    if needs_regex {
        for (static_name, pattern) in &regex_patterns {
            e.writeln(&format!(
                "static {static_name}: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r\"{pattern}\").unwrap());",
            ));
        }
        e.newline();
    }

    // ── Struct ─────────────────────────────────────────────────
    emit_guard_struct(e, decl);
    e.newline();

    // ── impl Validate ──────────────────────────────────────────
    emit_validate_impl(e, decl, &regex_patterns);

    // ── impl Sanitize (only if transforms exist) ───────────────
    if has_transforms {
        emit_sanitize_impl(e, decl);
    }
}

// ── Struct generation ──────────────────────────────────────────────────

/// Emit the `#[derive(...)] pub struct Guard { ... }` declaration.
fn emit_guard_struct(e: &mut RustEmitter, decl: &GuardDecl) {
    e.emit_attribute("derive(Debug, Clone, Serialize, Deserialize)");
    let struct_name = &decl.name;
    e.block(&format!("pub struct {struct_name}"), |e| {
        for field in &decl.fields {
            let field_name = to_snake_case(&field.name);
            let rust_type = guard_field_rust_type(field);

            // Add serde attributes for default/nullable fields
            if has_validator(field, "default") {
                e.emit_attribute(&format!(
                    "serde(default = \"{}::__default_{}\")",
                    struct_name, field_name
                ));
            } else if has_validator(field, "nullable") {
                e.emit_attribute("serde(default)");
            }

            e.writeln(&format!("pub {field_name}: {rust_type},"));
        }
    });
}

// ── Validate trait impl ────────────────────────────────────────────────

/// Emit `impl Validate for Guard { fn validate(&self) -> ... }`.
fn emit_validate_impl(e: &mut RustEmitter, decl: &GuardDecl, regex_patterns: &[(String, String)]) {
    e.block(&format!("impl Validate for {}", decl.name), |e| {
        e.block(
            "fn validate(&self) -> Result<(), Vec<ValidationError>>",
            |e| {
                e.writeln("let mut errors = Vec::new();");

                for field in &decl.fields {
                    let checks: Vec<&ValidatorCall> = field
                        .validators
                        .iter()
                        .filter(|v| !is_meta_validator(&v.name))
                        .collect();
                    if checks.is_empty() {
                        continue;
                    }
                    e.newline();
                    let field_name = to_snake_case(&field.name);
                    let base_type = annotation_to_rust(&field.ty);
                    let is_optional =
                        has_validator(field, "optional") || has_validator(field, "nullable");

                    e.emit_comment(&format!("--- field: {} ---", field.name));

                    for v in checks {
                        emit_validator_check(
                            e,
                            &field_name,
                            &base_type,
                            is_optional,
                            v,
                            regex_patterns,
                        );
                    }
                }

                e.newline();
                e.block("if errors.is_empty()", |e| {
                    e.writeln("Ok(())");
                });
                e.block("else", |e| {
                    e.writeln("Err(errors)");
                });
            },
        );
    });
    e.newline();
}

// ── Sanitize trait impl ────────────────────────────────────────────────

/// Emit `impl Sanitize for Guard { fn sanitize(&mut self) { ... } }`.
fn emit_sanitize_impl(e: &mut RustEmitter, decl: &GuardDecl) {
    e.block(&format!("impl Sanitize for {}", decl.name), |e| {
        e.block("fn sanitize(&mut self)", |e| {
            for field in &decl.fields {
                let field_name = to_snake_case(&field.name);
                let is_optional = has_validator(field, "optional")
                    || has_validator(field, "nullable");

                for v in &field.validators {
                    match v.name.as_str() {
                        "trim" => {
                            if is_optional {
                                e.block(
                                    &format!("if let Some(ref mut __val) = self.{field_name}"),
                                    |e| {
                                        e.writeln("*__val = __val.trim().to_string();");
                                    },
                                );
                            } else {
                                e.writeln(&format!(
                                    "self.{field_name} = self.{field_name}.trim().to_string();"
                                ));
                            }
                        }
                        "precision" => {
                            let n = extract_int_arg(&v.args, 0).unwrap_or(2);
                            let factor = 10_f64.powi(n as i32);
                            if is_optional {
                                e.block(
                                    &format!("if let Some(ref mut __val) = self.{field_name}"),
                                    |e| {
                                        e.writeln(&format!(
                                            "*__val = (*__val * {factor:.1}).round() / {factor:.1};"
                                        ));
                                    },
                                );
                            } else {
                                e.writeln(&format!(
                                    "self.{field_name} = (self.{field_name} * {factor:.1}).round() / {factor:.1};"
                                ));
                            }
                        }
                        _ => {} // non-transform validators handled in validate()
                    }
                }
            }
        });
    });
    e.newline();
}

// ── Default value helpers ──────────────────────────────────────────────

/// Emit default value functions referenced by `#[serde(default = "...")]`.
///
/// Called from `emit_guard_file` after the struct to produce companion
/// functions in a separate impl block.
fn _emit_default_fns(e: &mut RustEmitter, decl: &GuardDecl) {
    let has_defaults = decl.fields.iter().any(|f| has_validator(f, "default"));
    if !has_defaults {
        return;
    }

    e.emit_impl(decl.name.as_ref(), |e| {
        for field in &decl.fields {
            for v in &field.validators {
                if v.name == "default" {
                    let field_name = to_snake_case(&field.name);
                    let rust_type = guard_field_rust_type(field);
                    let default_val = extract_string_arg(&v.args, 0)
                        .unwrap_or_else(|| "Default::default()".to_string());
                    e.emit_fn(
                        "",
                        false,
                        &format!("__default_{field_name}"),
                        &[],
                        Some(&rust_type),
                        |e| {
                            e.writeln(&default_val);
                        },
                    );
                }
            }
        }
    });
}

// ── Validator check emission ───────────────────────────────────────────

/// Emit a single validator check for a field.
fn emit_validator_check(
    e: &mut RustEmitter,
    field_name: &str,
    base_type: &str,
    is_optional: bool,
    validator: &ValidatorCall,
    regex_patterns: &[(String, String)],
) {
    let accessor = if is_optional {
        "__val".to_string()
    } else {
        format!("self.{field_name}")
    };

    let check = build_validator_condition(&accessor, base_type, validator, regex_patterns);
    let message = build_validator_message(validator);

    if check.is_none() {
        e.writeln(&format!(
            "// TODO: custom validator '{}' for field '{}'",
            validator.name, field_name
        ));
        return;
    }
    let check = check.unwrap();

    if is_optional {
        e.block(
            &format!("if let Some(ref __val) = self.{field_name}"),
            |e| {
                e.block(&format!("if {check}"), |e| {
                    e.writeln(&format!(
                        "errors.push(ValidationError::new(\"{field_name}\", \"{message}\"));",
                    ));
                });
            },
        );
    } else {
        e.block(&format!("if {check}"), |e| {
            e.writeln(&format!(
                "errors.push(ValidationError::new(\"{field_name}\", \"{message}\"));",
            ));
        });
    }
}

/// Build the boolean condition expression for a validator.
fn build_validator_condition(
    accessor: &str,
    base_type: &str,
    validator: &ValidatorCall,
    regex_patterns: &[(String, String)],
) -> Option<String> {
    let is_numeric = base_type == "i64" || base_type == "f64";

    match validator.name.as_str() {
        // ── Numeric checks ─────────────────────────────────────
        "min" => {
            let val = extract_int_arg(&validator.args, 0)?;
            Some(format!("{accessor} < {val}"))
        }
        "max" => {
            let val = extract_int_arg(&validator.args, 0)?;
            Some(format!("{accessor} > {val}"))
        }
        "range" => {
            let min = extract_int_arg(&validator.args, 0)?;
            let max = extract_int_arg(&validator.args, 1)?;
            Some(format!("{accessor} < {min} || {accessor} > {max}"))
        }
        "positive" if is_numeric => Some(format!("{accessor} <= 0")),
        "nonNegative" if is_numeric => Some(format!("{accessor} < 0")),
        "integer" if base_type == "f64" => Some(format!("{accessor}.fract() != 0.0")),

        // ── Length checks ──────────────────────────────────────
        "minLen" => {
            let val = extract_int_arg(&validator.args, 0)?;
            Some(format!("{accessor}.len() < {val}"))
        }
        "maxLen" => {
            let val = extract_int_arg(&validator.args, 0)?;
            Some(format!("{accessor}.len() > {val}"))
        }
        "nonEmpty" => Some(format!("{accessor}.is_empty()")),

        // ── Format checks (regex-based) ────────────────────────
        "email" => {
            let static_name = find_regex_static("email", regex_patterns)?;
            Some(format!("!{static_name}.is_match(&{accessor})"))
        }
        "url" => {
            let static_name = find_regex_static("url", regex_patterns)?;
            Some(format!("!{static_name}.is_match(&{accessor})"))
        }
        "uuid" => {
            let static_name = find_regex_static("uuid", regex_patterns)?;
            Some(format!("!{static_name}.is_match(&{accessor})"))
        }
        "regex" => {
            if let Some(Expr::StringLit { value, .. }) = validator.args.first() {
                let static_name = find_regex_static_by_pattern(value.as_str(), regex_patterns)?;
                Some(format!("!{static_name}.is_match(&{accessor})"))
            } else {
                None
            }
        }

        // ── OneOf ──────────────────────────────────────────────
        "oneOf" => {
            let values: Vec<String> = validator
                .args
                .iter()
                .filter_map(|a| {
                    if let Expr::StringLit { value, .. } = a {
                        Some(format!("\"{}\"", value))
                    } else {
                        None
                    }
                })
                .collect();
            if values.is_empty() {
                return None;
            }
            Some(format!(
                "![{}].contains(&{accessor}.as_str())",
                values.join(", ")
            ))
        }

        // Transforms and meta-validators produce no check
        "trim" | "precision" | "default" | "optional" | "nullable" => None,

        _ => None, // Unknown — emit TODO comment
    }
}

/// Build a human-readable error message for a validator.
fn build_validator_message(validator: &ValidatorCall) -> String {
    match validator.name.as_str() {
        "min" => {
            let val = extract_int_arg(&validator.args, 0).unwrap_or(0);
            format!("must be at least {val}")
        }
        "max" => {
            let val = extract_int_arg(&validator.args, 0).unwrap_or(0);
            format!("must be at most {val}")
        }
        "range" => {
            let min = extract_int_arg(&validator.args, 0).unwrap_or(0);
            let max = extract_int_arg(&validator.args, 1).unwrap_or(0);
            format!("must be between {min} and {max}")
        }
        "minLen" => {
            let val = extract_int_arg(&validator.args, 0).unwrap_or(0);
            format!("must be at least {val} character(s)")
        }
        "maxLen" => {
            let val = extract_int_arg(&validator.args, 0).unwrap_or(0);
            format!("must be at most {val} character(s)")
        }
        "email" => "must be a valid email address".into(),
        "url" => "must be a valid URL".into(),
        "uuid" => "must be a valid UUID".into(),
        "regex" => "does not match required pattern".into(),
        "oneOf" => "must be one of the allowed values".into(),
        "nonEmpty" => "must not be empty".into(),
        "positive" => "must be positive".into(),
        "nonNegative" => "must not be negative".into(),
        "integer" => "must be a whole number".into(),
        other => format!("failed validation: {other}"),
    }
}

// ── Regex pattern collection ───────────────────────────────────────────

/// Well-known regex patterns for built-in validators.
const EMAIL_PATTERN: &str = r"^[^@\s]+@[^@\s]+\.[^@\s]+$";
const URL_PATTERN: &str = r"^https?://[^\s]+$";
const UUID_PATTERN: &str =
    r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-4[0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}$";

/// Collect all regex patterns needed by a guard's validators.
///
/// Returns `(STATIC_NAME, pattern)` pairs, deduplicated.
fn collect_regex_patterns(decl: &GuardDecl) -> Vec<(String, String)> {
    let mut seen = BTreeSet::new();
    let mut patterns = Vec::new();

    for field in &decl.fields {
        for v in &field.validators {
            match v.name.as_str() {
                "email" => {
                    if seen.insert("EMAIL_RE") {
                        patterns.push(("EMAIL_RE".into(), EMAIL_PATTERN.into()));
                    }
                }
                "url" => {
                    if seen.insert("URL_RE") {
                        patterns.push(("URL_RE".into(), URL_PATTERN.into()));
                    }
                }
                "uuid" => {
                    if seen.insert("UUID_RE") {
                        patterns.push(("UUID_RE".into(), UUID_PATTERN.into()));
                    }
                }
                "regex" => {
                    if let Some(Expr::StringLit { value, .. }) = v.args.first() {
                        let static_name = regex_static_name(value);
                        if seen.insert(Box::leak(static_name.clone().into_boxed_str())) {
                            patterns.push((static_name, value.to_string()));
                        }
                    }
                }
                _ => {}
            }
        }
    }
    patterns
}

/// Generate a static name for a custom regex pattern.
fn regex_static_name(pattern: &str) -> String {
    // Create a deterministic name from the pattern hash
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    pattern.hash(&mut hasher);
    let hash = hasher.finish();
    format!("CUSTOM_RE_{:X}", hash & 0xFFFF_FFFF)
}

/// Find the static name for a built-in regex validator type.
fn find_regex_static(kind: &str, patterns: &[(String, String)]) -> Option<String> {
    let expected_name = match kind {
        "email" => "EMAIL_RE",
        "url" => "URL_RE",
        "uuid" => "UUID_RE",
        _ => return None,
    };
    patterns
        .iter()
        .find(|(name, _)| name == expected_name)
        .map(|(name, _)| name.clone())
}

/// Find the static name for a custom regex pattern.
fn find_regex_static_by_pattern(pattern: &str, patterns: &[(String, String)]) -> Option<String> {
    patterns
        .iter()
        .find(|(_, p)| p == pattern)
        .map(|(name, _)| name.clone())
}

// ── Helpers ────────────────────────────────────────────────────────────

/// Whether a validator name is a meta-validator that doesn't produce a runtime check.
fn is_meta_validator(name: &str) -> bool {
    matches!(
        name,
        "optional" | "nullable" | "trim" | "precision" | "default"
    )
}

/// Check if a field has a specific validator by name.
fn has_validator(field: &GuardFieldDecl, name: &str) -> bool {
    field.validators.iter().any(|v| v.name == name)
}

/// Extract an integer literal argument from a validator's args list.
fn extract_int_arg(args: &[Expr], index: usize) -> Option<i64> {
    args.get(index).and_then(|expr| {
        if let Expr::IntLit { value, .. } = expr {
            Some(*value)
        } else {
            None
        }
    })
}

/// Extract a string literal argument from a validator's args list.
#[allow(dead_code)]
fn extract_string_arg(args: &[Expr], index: usize) -> Option<String> {
    args.get(index).and_then(|expr| match expr {
        Expr::StringLit { value, .. } => Some(format!("String::from(\"{}\")", value)),
        Expr::IntLit { value, .. } => Some(format!("{value}_i64")),
        Expr::FloatLit { value, .. } => Some(format!("{value}_f64")),
        Expr::BoolLit { value, .. } => Some(format!("{value}")),
        _ => None,
    })
}

/// Determine the Rust type for a guard field, considering optional/nullable validators.
fn guard_field_rust_type(field: &GuardFieldDecl) -> String {
    let base = annotation_to_rust(&field.ty);
    let is_optional = has_validator(field, "optional") || has_validator(field, "nullable");
    if is_optional {
        format!("Option<{base}>")
    } else {
        base
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::rust_emitter::RustEmitter;
    use crate::span::Span;

    fn s() -> Span {
        Span::dummy()
    }

    fn emit(decl: &GuardDecl) -> String {
        let mut e = RustEmitter::new();
        let no_shared = HashSet::new();
        emit_guard_file(&mut e, decl, &no_shared);
        e.finish()
    }

    #[test]
    fn simple_guard_struct() {
        let out = emit(&GuardDecl {
            name: "LoginForm".into(),
            fields: vec![
                GuardFieldDecl {
                    name: "email".into(),
                    ty: TypeAnnotation::Named {
                        name: "string".into(),
                        span: s(),
                    },
                    validators: vec![],
                    span: s(),
                },
                GuardFieldDecl {
                    name: "password".into(),
                    ty: TypeAnnotation::Named {
                        name: "string".into(),
                        span: s(),
                    },
                    validators: vec![],
                    span: s(),
                },
            ],
            span: s(),
        });
        assert!(out.contains("#[derive(Debug, Clone, Serialize, Deserialize)]"));
        assert!(out.contains("pub struct LoginForm {"));
        assert!(out.contains("pub email: String,"));
        assert!(out.contains("pub password: String,"));
    }

    #[test]
    fn guard_with_min_validator() {
        let out = emit(&GuardDecl {
            name: "AgeForm".into(),
            fields: vec![GuardFieldDecl {
                name: "age".into(),
                ty: TypeAnnotation::Named {
                    name: "int".into(),
                    span: s(),
                },
                validators: vec![ValidatorCall {
                    name: "min".into(),
                    args: vec![Expr::IntLit {
                        value: 0,
                        span: s(),
                    }],
                    span: s(),
                }],
                span: s(),
            }],
            span: s(),
        });
        assert!(out.contains("if self.age < 0"));
        assert!(out.contains("must be at least 0"));
    }

    #[test]
    fn guard_email_uses_regex() {
        let out = emit(&GuardDecl {
            name: "EmailForm".into(),
            fields: vec![GuardFieldDecl {
                name: "email".into(),
                ty: TypeAnnotation::Named {
                    name: "string".into(),
                    span: s(),
                },
                validators: vec![ValidatorCall {
                    name: "email".into(),
                    args: vec![],
                    span: s(),
                }],
                span: s(),
            }],
            span: s(),
        });
        assert!(out.contains("EMAIL_RE"), "should use EMAIL_RE static");
        assert!(
            out.contains("LazyLock<regex::Regex>"),
            "should have LazyLock"
        );
        assert!(out.contains("EMAIL_RE.is_match"), "should call is_match");
    }

    #[test]
    fn guard_url_uses_regex() {
        let out = emit(&GuardDecl {
            name: "UrlForm".into(),
            fields: vec![GuardFieldDecl {
                name: "website".into(),
                ty: TypeAnnotation::Named {
                    name: "string".into(),
                    span: s(),
                },
                validators: vec![ValidatorCall {
                    name: "url".into(),
                    args: vec![],
                    span: s(),
                }],
                span: s(),
            }],
            span: s(),
        });
        assert!(out.contains("URL_RE"));
        assert!(out.contains("URL_RE.is_match"));
    }

    #[test]
    fn guard_uuid_validator() {
        let out = emit(&GuardDecl {
            name: "TokenForm".into(),
            fields: vec![GuardFieldDecl {
                name: "token".into(),
                ty: TypeAnnotation::Named {
                    name: "string".into(),
                    span: s(),
                },
                validators: vec![ValidatorCall {
                    name: "uuid".into(),
                    args: vec![],
                    span: s(),
                }],
                span: s(),
            }],
            span: s(),
        });
        assert!(out.contains("UUID_RE"));
        assert!(out.contains("UUID_RE.is_match"));
    }

    #[test]
    fn guard_custom_regex() {
        let out = emit(&GuardDecl {
            name: "CodeForm".into(),
            fields: vec![GuardFieldDecl {
                name: "code".into(),
                ty: TypeAnnotation::Named {
                    name: "string".into(),
                    span: s(),
                },
                validators: vec![ValidatorCall {
                    name: "regex".into(),
                    args: vec![Expr::StringLit {
                        value: "^[A-Z]{3}$".into(),
                        span: s(),
                    }],
                    span: s(),
                }],
                span: s(),
            }],
            span: s(),
        });
        assert!(out.contains("CUSTOM_RE_"));
        assert!(out.contains("^[A-Z]{3}$"));
        assert!(out.contains(".is_match(&self.code)"));
    }

    #[test]
    fn guard_oneof_validator() {
        let out = emit(&GuardDecl {
            name: "RoleForm".into(),
            fields: vec![GuardFieldDecl {
                name: "role".into(),
                ty: TypeAnnotation::Named {
                    name: "string".into(),
                    span: s(),
                },
                validators: vec![ValidatorCall {
                    name: "oneOf".into(),
                    args: vec![
                        Expr::StringLit {
                            value: "admin".into(),
                            span: s(),
                        },
                        Expr::StringLit {
                            value: "user".into(),
                            span: s(),
                        },
                    ],
                    span: s(),
                }],
                span: s(),
            }],
            span: s(),
        });
        assert!(out.contains("\"admin\", \"user\""));
        assert!(out.contains(".contains(&self.role.as_str())"));
    }

    #[test]
    fn guard_range_validator() {
        let out = emit(&GuardDecl {
            name: "AgeForm".into(),
            fields: vec![GuardFieldDecl {
                name: "age".into(),
                ty: TypeAnnotation::Named {
                    name: "int".into(),
                    span: s(),
                },
                validators: vec![ValidatorCall {
                    name: "range".into(),
                    args: vec![
                        Expr::IntLit {
                            value: 0,
                            span: s(),
                        },
                        Expr::IntLit {
                            value: 150,
                            span: s(),
                        },
                    ],
                    span: s(),
                }],
                span: s(),
            }],
            span: s(),
        });
        assert!(out.contains("self.age < 0 || self.age > 150"));
        assert!(out.contains("must be between 0 and 150"));
    }

    #[test]
    fn guard_trim_generates_sanitize() {
        let out = emit(&GuardDecl {
            name: "TrimForm".into(),
            fields: vec![GuardFieldDecl {
                name: "name".into(),
                ty: TypeAnnotation::Named {
                    name: "string".into(),
                    span: s(),
                },
                validators: vec![
                    ValidatorCall {
                        name: "trim".into(),
                        args: vec![],
                        span: s(),
                    },
                    ValidatorCall {
                        name: "minLen".into(),
                        args: vec![Expr::IntLit {
                            value: 1,
                            span: s(),
                        }],
                        span: s(),
                    },
                ],
                span: s(),
            }],
            span: s(),
        });
        assert!(
            out.contains("impl Sanitize for TrimForm"),
            "should impl Sanitize"
        );
        assert!(
            out.contains("fn sanitize(&mut self)"),
            "should have sanitize method"
        );
        assert!(out.contains(".trim().to_string()"), "should trim");
        // Should also have the minLen check in validate()
        assert!(out.contains("self.name.len() < 1"));
    }

    #[test]
    fn guard_precision_generates_sanitize() {
        let out = emit(&GuardDecl {
            name: "PriceForm".into(),
            fields: vec![GuardFieldDecl {
                name: "price".into(),
                ty: TypeAnnotation::Named {
                    name: "float".into(),
                    span: s(),
                },
                validators: vec![ValidatorCall {
                    name: "precision".into(),
                    args: vec![Expr::IntLit {
                        value: 2,
                        span: s(),
                    }],
                    span: s(),
                }],
                span: s(),
            }],
            span: s(),
        });
        assert!(out.contains("impl Sanitize for PriceForm"));
        assert!(out.contains("100.0"), "should use 10^2 factor");
        assert!(out.contains(".round()"), "should round");
    }

    #[test]
    fn guard_optional_field() {
        let out = emit(&GuardDecl {
            name: "OptForm".into(),
            fields: vec![GuardFieldDecl {
                name: "bio".into(),
                ty: TypeAnnotation::Named {
                    name: "string".into(),
                    span: s(),
                },
                validators: vec![
                    ValidatorCall {
                        name: "optional".into(),
                        args: vec![],
                        span: s(),
                    },
                    ValidatorCall {
                        name: "maxLen".into(),
                        args: vec![Expr::IntLit {
                            value: 500,
                            span: s(),
                        }],
                        span: s(),
                    },
                ],
                span: s(),
            }],
            span: s(),
        });
        assert!(out.contains("pub bio: Option<String>,"));
        assert!(out.contains("if let Some(ref __val) = self.bio"));
        assert!(out.contains("__val.len() > 500"));
    }

    #[test]
    fn guard_nullable_field() {
        let out = emit(&GuardDecl {
            name: "NullForm".into(),
            fields: vec![GuardFieldDecl {
                name: "middle".into(),
                ty: TypeAnnotation::Named {
                    name: "string".into(),
                    span: s(),
                },
                validators: vec![ValidatorCall {
                    name: "nullable".into(),
                    args: vec![],
                    span: s(),
                }],
                span: s(),
            }],
            span: s(),
        });
        assert!(out.contains("pub middle: Option<String>,"));
        assert!(out.contains("#[serde(default)]"));
    }

    #[test]
    fn guard_validate_uses_trait() {
        let out = emit(&GuardDecl {
            name: "X".into(),
            fields: vec![],
            span: s(),
        });
        assert!(out.contains("impl Validate for X"));
        assert!(out.contains("use crate::shared::validation::{ValidationError, Validate}"));
    }

    #[test]
    fn guard_no_sanitize_without_transforms() {
        let out = emit(&GuardDecl {
            name: "NoTransform".into(),
            fields: vec![GuardFieldDecl {
                name: "name".into(),
                ty: TypeAnnotation::Named {
                    name: "string".into(),
                    span: s(),
                },
                validators: vec![ValidatorCall {
                    name: "minLen".into(),
                    args: vec![Expr::IntLit {
                        value: 1,
                        span: s(),
                    }],
                    span: s(),
                }],
                span: s(),
            }],
            span: s(),
        });
        assert!(
            !out.contains("impl Sanitize"),
            "should not impl Sanitize without transforms"
        );
        assert!(
            !out.contains("fn sanitize"),
            "should not have sanitize method"
        );
    }

    #[test]
    fn guard_deduplicates_regex_statics() {
        // Two email fields should share one EMAIL_RE
        let out = emit(&GuardDecl {
            name: "TwoEmails".into(),
            fields: vec![
                GuardFieldDecl {
                    name: "email1".into(),
                    ty: TypeAnnotation::Named {
                        name: "string".into(),
                        span: s(),
                    },
                    validators: vec![ValidatorCall {
                        name: "email".into(),
                        args: vec![],
                        span: s(),
                    }],
                    span: s(),
                },
                GuardFieldDecl {
                    name: "email2".into(),
                    ty: TypeAnnotation::Named {
                        name: "string".into(),
                        span: s(),
                    },
                    validators: vec![ValidatorCall {
                        name: "email".into(),
                        args: vec![],
                        span: s(),
                    }],
                    span: s(),
                },
            ],
            span: s(),
        });
        // Should only have one EMAIL_RE declaration
        let count = out.matches("static EMAIL_RE").count();
        assert_eq!(count, 1, "should deduplicate EMAIL_RE: found {count}");
    }

    #[test]
    fn empty_guard() {
        let out = emit(&GuardDecl {
            name: "Empty".into(),
            fields: vec![],
            span: s(),
        });
        assert!(out.contains("pub struct Empty {"));
        assert!(out.contains("fn validate"));
    }
}
