//! Guard → JavaScript validation function code generation.
//!
//! For each GaleX [`GuardDecl`], generates an ES module containing:
//! - `export function validate{Guard}(data)` — runtime constraint checks
//! - `export function sanitize{Guard}(data)` — input transforms (trim, precision)
//! - Regex `const` declarations for format validators (email, url, uuid, custom)
//!
//! Mirrors the Rust-side codegen in [`emit_guard`](super::emit_guard) with
//! identical validator logic and error messages, ensuring client-side and
//! server-side validation always agree.

use std::collections::BTreeSet;

use crate::ast::*;
use crate::codegen::js_emitter::JsEmitter;
use crate::codegen::types::to_module_name;

// ── Well-known regex patterns (shared with emit_guard.rs) ──────────────

/// These must be identical to the Rust-side patterns in `emit_guard.rs`
/// to ensure client and server validation match.
const EMAIL_PATTERN: &str = r"^[^@\s]+@[^@\s]+\.[^@\s]+$";
const URL_PATTERN: &str = r"^https?://[^\s]+$";
const UUID_PATTERN: &str =
    r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-4[0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}$";

// ── Metadata returned for form wiring ──────────────────────────────────

/// Metadata about a generated JS guard module, used by form wiring and
/// SSR script injection to produce correct import/wire calls.
#[derive(Debug, Clone)]
pub struct GuardJsMeta {
    /// PascalCase guard name (e.g. `LoginForm`).
    pub guard_name: String,
    /// Snake_case module file name (e.g. `login_form`).
    pub module_name: String,
    /// JS function name for validation (e.g. `validateLoginForm`).
    pub validate_fn: String,
    /// JS function name for sanitization (e.g. `sanitizeLoginForm`),
    /// or `None` if the guard has no transforms.
    pub sanitize_fn: Option<String>,
    /// Field names declared in the guard (original camelCase).
    pub fields: Vec<String>,
}

// ── Public entry point ─────────────────────────────────────────────────

/// Emit a complete guard JS validation module.
///
/// Returns [`GuardJsMeta`] for use by form wiring / SSR injection.
pub fn emit_guard_js_file(e: &mut JsEmitter, decl: &GuardDecl) -> GuardJsMeta {
    e.emit_file_header(&format!("Guard validator: `{}`.", decl.name));

    let has_transforms = decl.fields.iter().any(|f| {
        f.validators
            .iter()
            .any(|v| matches!(v.name.as_str(), "trim" | "precision"))
    });

    // ── Regex constants ────────────────────────────────────────
    let regex_patterns = collect_js_regex_patterns(decl);
    for (const_name, pattern) in &regex_patterns {
        e.emit_const(const_name, &format!("/{pattern}/"));
    }
    if !regex_patterns.is_empty() {
        e.newline();
    }

    // ── Sanitize function (only if transforms exist) ───────────
    if has_transforms {
        emit_sanitize_fn(e, decl);
        e.newline();
    }

    // ── Validate function ──────────────────────────────────────
    emit_validate_fn(e, decl, &regex_patterns);

    // ── Build metadata ─────────────────────────────────────────
    let guard_name = decl.name.to_string();
    let module_name = to_module_name(&guard_name);
    let validate_fn = format!("validate{guard_name}");
    let sanitize_fn = if has_transforms {
        Some(format!("sanitize{guard_name}"))
    } else {
        None
    };
    let fields = decl.fields.iter().map(|f| f.name.to_string()).collect();

    GuardJsMeta {
        guard_name,
        module_name,
        validate_fn,
        sanitize_fn,
        fields,
    }
}

// ── Validate function emission ─────────────────────────────────────────

/// Emit `export function validate{Guard}(data) { ... }`.
fn emit_validate_fn(e: &mut JsEmitter, decl: &GuardDecl, regex_patterns: &[(String, String)]) {
    let fn_name = format!("validate{}", decl.name);
    e.emit_export_fn(&fn_name, &["data"], |e| {
        e.writeln("const errors = [];");

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
            let field_name = &field.name;
            let is_optional = has_validator(field, "optional") || has_validator(field, "nullable");
            let base_type = annotation_to_js_type(&field.ty);

            e.emit_comment(&format!("--- field: {field_name} ---"));

            if is_optional {
                e.emit_if(&format!("data.{field_name} != null"), |e| {
                    for v in &checks {
                        emit_js_validator_check(e, field_name, &base_type, v, regex_patterns);
                    }
                });
            } else {
                for v in &checks {
                    emit_js_validator_check(e, field_name, &base_type, v, regex_patterns);
                }
            }
        }

        e.newline();
        e.writeln("return errors.length === 0");
        e.writeln("  ? { ok: true, data }");
        e.writeln("  : { ok: false, errors };");
    });
}

/// Emit a single validator check for a field.
fn emit_js_validator_check(
    e: &mut JsEmitter,
    field_name: &str,
    base_type: &str,
    validator: &ValidatorCall,
    regex_patterns: &[(String, String)],
) {
    let accessor = format!("data.{field_name}");

    let check = build_js_validator_condition(&accessor, base_type, validator, regex_patterns);
    let message = build_validator_message(validator);

    if let Some(check) = check {
        e.emit_if(&check, |e| {
            e.writeln(&format!(
                "errors.push({{ field: \"{field_name}\", message: \"{message}\" }});"
            ));
        });
    } else {
        e.emit_comment(&format!(
            "TODO: custom validator '{}' for field '{}'",
            validator.name, field_name
        ));
    }
}

/// Build the boolean condition expression for a JS validator.
///
/// Returns `Some(condition)` for known validators, `None` for unknown.
fn build_js_validator_condition(
    accessor: &str,
    base_type: &str,
    validator: &ValidatorCall,
    regex_patterns: &[(String, String)],
) -> Option<String> {
    let is_numeric = base_type == "number";

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
        "integer" if is_numeric => Some(format!("!Number.isInteger({accessor})")),

        // ── Length checks ──────────────────────────────────────
        "minLen" => {
            let val = extract_int_arg(&validator.args, 0)?;
            Some(format!("{accessor}.length < {val}"))
        }
        "maxLen" => {
            let val = extract_int_arg(&validator.args, 0)?;
            Some(format!("{accessor}.length > {val}"))
        }
        "nonEmpty" => Some(format!("{accessor}.length === 0")),

        // ── Format checks (regex-based) ────────────────────────
        "email" => {
            let const_name = find_js_regex_const("email", regex_patterns)?;
            Some(format!("!{const_name}.test({accessor})"))
        }
        "url" => {
            let const_name = find_js_regex_const("url", regex_patterns)?;
            Some(format!("!{const_name}.test({accessor})"))
        }
        "uuid" => {
            let const_name = find_js_regex_const("uuid", regex_patterns)?;
            Some(format!("!{const_name}.test({accessor})"))
        }
        "regex" => {
            if let Some(Expr::StringLit { value, .. }) = validator.args.first() {
                let const_name = find_js_regex_const_by_pattern(value.as_str(), regex_patterns)?;
                Some(format!("!{const_name}.test({accessor})"))
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
            Some(format!("![{}].includes({accessor})", values.join(", ")))
        }

        // ── Custom validator (callback hook) ───────────────────
        "custom" => {
            if let Some(Expr::StringLit { value, .. }) = validator.args.first() {
                // Calls window.__galeValidators.fnName(value) if registered
                Some(format!(
                    "window.__galeValidators?.{value}?.({accessor}) === false"
                ))
            } else if let Some(Expr::Ident { name, .. }) = validator.args.first() {
                Some(format!(
                    "window.__galeValidators?.{name}?.({accessor}) === false"
                ))
            } else {
                None
            }
        }

        // Transforms and meta-validators produce no check
        "trim" | "precision" | "default" | "optional" | "nullable" => None,

        _ => None, // Unknown — emit TODO comment
    }
}

/// Build a human-readable error message for a validator.
///
/// These messages are identical to the Rust side (`emit_guard.rs:build_validator_message`)
/// to ensure client and server errors match.
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
        "custom" => {
            if let Some(Expr::StringLit { value, .. }) = validator.args.first() {
                format!("failed validation: {value}")
            } else if let Some(Expr::Ident { name, .. }) = validator.args.first() {
                format!("failed validation: {name}")
            } else {
                "failed custom validation".into()
            }
        }
        other => format!("failed validation: {other}"),
    }
}

// ── Sanitize function emission ─────────────────────────────────────────

/// Emit `export function sanitize{Guard}(data) { ... }`.
fn emit_sanitize_fn(e: &mut JsEmitter, decl: &GuardDecl) {
    let fn_name = format!("sanitize{}", decl.name);
    e.emit_export_fn(&fn_name, &["data"], |e| {
        e.writeln("const out = { ...data };");

        for field in &decl.fields {
            let field_name = &field.name;
            let is_optional = has_validator(field, "optional") || has_validator(field, "nullable");

            for v in &field.validators {
                match v.name.as_str() {
                    "trim" => {
                        if is_optional {
                            e.emit_if(
                                &format!("typeof out.{field_name} === 'string'"),
                                |e| {
                                    e.writeln(&format!(
                                        "out.{field_name} = out.{field_name}.trim();"
                                    ));
                                },
                            );
                        } else {
                            e.writeln(&format!(
                                "if (typeof out.{field_name} === 'string') out.{field_name} = out.{field_name}.trim();"
                            ));
                        }
                    }
                    "precision" => {
                        let n = extract_int_arg(&v.args, 0).unwrap_or(2);
                        let factor = 10_f64.powi(n as i32);
                        if is_optional {
                            e.emit_if(
                                &format!("typeof out.{field_name} === 'number'"),
                                |e| {
                                    e.writeln(&format!(
                                        "out.{field_name} = Math.round(out.{field_name} * {factor:.0}) / {factor:.0};"
                                    ));
                                },
                            );
                        } else {
                            e.writeln(&format!(
                                "if (typeof out.{field_name} === 'number') out.{field_name} = Math.round(out.{field_name} * {factor:.0}) / {factor:.0};"
                            ));
                        }
                    }
                    _ => {} // non-transform validators handled in validate()
                }
            }
        }

        e.writeln("return out;");
    });
}

// ── Regex pattern collection (JS-specific) ─────────────────────────────

/// Collect all regex patterns needed by a guard's validators.
///
/// Returns `(CONST_NAME, pattern)` pairs, deduplicated.
/// Uses JS regex literal format (no delimiters in pattern — caller wraps in `/.../`).
fn collect_js_regex_patterns(decl: &GuardDecl) -> Vec<(String, String)> {
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
                        let const_name = js_regex_const_name(value);
                        if seen.insert(Box::leak(const_name.clone().into_boxed_str())) {
                            patterns.push((const_name, value.to_string()));
                        }
                    }
                }
                _ => {}
            }
        }
    }
    patterns
}

/// Generate a constant name for a custom regex pattern.
fn js_regex_const_name(pattern: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    pattern.hash(&mut hasher);
    let hash = hasher.finish();
    format!("CUSTOM_RE_{:X}", hash & 0xFFFF_FFFF)
}

/// Find the constant name for a built-in regex validator type.
fn find_js_regex_const(kind: &str, patterns: &[(String, String)]) -> Option<String> {
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

/// Find the constant name for a custom regex pattern.
fn find_js_regex_const_by_pattern(pattern: &str, patterns: &[(String, String)]) -> Option<String> {
    patterns
        .iter()
        .find(|(_, p)| p == pattern)
        .map(|(name, _)| name.clone())
}

// ── Type mapping ───────────────────────────────────────────────────────

/// Map a GaleX type annotation to its JS type string for typeof checks.
///
/// Used internally to determine which validator conditions are applicable.
fn annotation_to_js_type(ann: &TypeAnnotation) -> String {
    match ann {
        TypeAnnotation::Named { name, .. } => match name.as_str() {
            "string" => "string".into(),
            "int" | "float" => "number".into(),
            "bool" => "boolean".into(),
            _ => "object".into(),
        },
        TypeAnnotation::Array { .. } => "object".into(),
        TypeAnnotation::Optional { inner, .. } => annotation_to_js_type(inner),
        _ => "object".into(),
    }
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

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::js_emitter::JsEmitter;
    use crate::span::Span;

    fn s() -> Span {
        Span::dummy()
    }

    fn make_guard(name: &str, fields: Vec<GuardFieldDecl>) -> GuardDecl {
        GuardDecl {
            name: name.into(),
            fields,
            span: s(),
        }
    }

    fn make_field(name: &str, ty: &str, validators: Vec<(&str, Vec<Expr>)>) -> GuardFieldDecl {
        GuardFieldDecl {
            name: name.into(),
            ty: TypeAnnotation::Named {
                name: ty.into(),
                span: s(),
            },
            validators: validators
                .into_iter()
                .map(|(vname, args)| ValidatorCall {
                    name: vname.into(),
                    args,
                    span: s(),
                })
                .collect(),
            span: s(),
        }
    }

    fn int_lit(val: i64) -> Expr {
        Expr::IntLit {
            value: val,
            span: s(),
        }
    }

    fn str_lit(val: &str) -> Expr {
        Expr::StringLit {
            value: val.into(),
            span: s(),
        }
    }

    // ── Basic structure tests ──────────────────────────────────

    #[test]
    fn simple_guard_js_validate() {
        let decl = make_guard(
            "User",
            vec![make_field(
                "name",
                "string",
                vec![("minLen", vec![int_lit(2)])],
            )],
        );
        let mut e = JsEmitter::new();
        let meta = emit_guard_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(out.contains("export function validateUser(data)"));
        assert!(out.contains("data.name.length < 2"));
        assert!(out.contains("field: \"name\""));
        assert_eq!(meta.validate_fn, "validateUser");
        assert_eq!(meta.fields, vec!["name"]);
    }

    #[test]
    fn guard_js_no_sanitize_without_transforms() {
        let decl = make_guard(
            "Simple",
            vec![make_field("age", "int", vec![("min", vec![int_lit(0)])])],
        );
        let mut e = JsEmitter::new();
        let meta = emit_guard_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(!out.contains("sanitize"));
        assert!(meta.sanitize_fn.is_none());
    }

    // ── Numeric validator tests ────────────────────────────────

    #[test]
    fn guard_js_min_validator() {
        let decl = make_guard(
            "G",
            vec![make_field("age", "int", vec![("min", vec![int_lit(18)])])],
        );
        let mut e = JsEmitter::new();
        emit_guard_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(out.contains("data.age < 18"));
        assert!(out.contains("must be at least 18"));
    }

    #[test]
    fn guard_js_max_validator() {
        let decl = make_guard(
            "G",
            vec![make_field(
                "score",
                "int",
                vec![("max", vec![int_lit(100)])],
            )],
        );
        let mut e = JsEmitter::new();
        emit_guard_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(out.contains("data.score > 100"));
        assert!(out.contains("must be at most 100"));
    }

    #[test]
    fn guard_js_range_validator() {
        let decl = make_guard(
            "G",
            vec![make_field(
                "age",
                "int",
                vec![("range", vec![int_lit(0), int_lit(150)])],
            )],
        );
        let mut e = JsEmitter::new();
        emit_guard_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(out.contains("data.age < 0 || data.age > 150"));
        assert!(out.contains("must be between 0 and 150"));
    }

    #[test]
    fn guard_js_positive_validator() {
        let decl = make_guard(
            "G",
            vec![make_field("count", "int", vec![("positive", vec![])])],
        );
        let mut e = JsEmitter::new();
        emit_guard_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(out.contains("data.count <= 0"));
        assert!(out.contains("must be positive"));
    }

    #[test]
    fn guard_js_non_negative_validator() {
        let decl = make_guard(
            "G",
            vec![make_field(
                "balance",
                "float",
                vec![("nonNegative", vec![])],
            )],
        );
        let mut e = JsEmitter::new();
        emit_guard_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(out.contains("data.balance < 0"));
        assert!(out.contains("must not be negative"));
    }

    #[test]
    fn guard_js_integer_validator() {
        let decl = make_guard(
            "G",
            vec![make_field("qty", "float", vec![("integer", vec![])])],
        );
        let mut e = JsEmitter::new();
        emit_guard_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(out.contains("!Number.isInteger(data.qty)"));
        assert!(out.contains("must be a whole number"));
    }

    // ── Length validator tests ──────────────────────────────────

    #[test]
    fn guard_js_minlen_validator() {
        let decl = make_guard(
            "G",
            vec![make_field(
                "name",
                "string",
                vec![("minLen", vec![int_lit(2)])],
            )],
        );
        let mut e = JsEmitter::new();
        emit_guard_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(out.contains("data.name.length < 2"));
        assert!(out.contains("must be at least 2 character(s)"));
    }

    #[test]
    fn guard_js_maxlen_validator() {
        let decl = make_guard(
            "G",
            vec![make_field(
                "bio",
                "string",
                vec![("maxLen", vec![int_lit(500)])],
            )],
        );
        let mut e = JsEmitter::new();
        emit_guard_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(out.contains("data.bio.length > 500"));
        assert!(out.contains("must be at most 500 character(s)"));
    }

    #[test]
    fn guard_js_nonempty_validator() {
        let decl = make_guard(
            "G",
            vec![make_field("title", "string", vec![("nonEmpty", vec![])])],
        );
        let mut e = JsEmitter::new();
        emit_guard_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(out.contains("data.title.length === 0"));
        assert!(out.contains("must not be empty"));
    }

    // ── Format validator tests (regex) ─────────────────────────

    #[test]
    fn guard_js_email_regex() {
        let decl = make_guard(
            "G",
            vec![make_field("email", "string", vec![("email", vec![])])],
        );
        let mut e = JsEmitter::new();
        emit_guard_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(out.contains("const EMAIL_RE = /^"));
        assert!(out.contains("!EMAIL_RE.test(data.email)"));
        assert!(out.contains("must be a valid email address"));
    }

    #[test]
    fn guard_js_url_regex() {
        let decl = make_guard(
            "G",
            vec![make_field("website", "string", vec![("url", vec![])])],
        );
        let mut e = JsEmitter::new();
        emit_guard_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(out.contains("const URL_RE = /^"));
        assert!(out.contains("!URL_RE.test(data.website)"));
        assert!(out.contains("must be a valid URL"));
    }

    #[test]
    fn guard_js_uuid_regex() {
        let decl = make_guard(
            "G",
            vec![make_field("token", "string", vec![("uuid", vec![])])],
        );
        let mut e = JsEmitter::new();
        emit_guard_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(out.contains("const UUID_RE = /^"));
        assert!(out.contains("!UUID_RE.test(data.token)"));
        assert!(out.contains("must be a valid UUID"));
    }

    #[test]
    fn guard_js_custom_regex() {
        let decl = make_guard(
            "G",
            vec![make_field(
                "code",
                "string",
                vec![("regex", vec![str_lit(r"^[A-Z]{3}$")])],
            )],
        );
        let mut e = JsEmitter::new();
        emit_guard_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(out.contains("const CUSTOM_RE_"));
        assert!(out.contains("/^[A-Z]{3}$/"));
        assert!(out.contains(".test(data.code)"));
    }

    #[test]
    fn guard_js_regex_dedup() {
        let decl = make_guard(
            "G",
            vec![
                make_field("email1", "string", vec![("email", vec![])]),
                make_field("email2", "string", vec![("email", vec![])]),
            ],
        );
        let mut e = JsEmitter::new();
        emit_guard_js_file(&mut e, &decl);
        let out = e.finish();

        // EMAIL_RE should appear exactly once as a const declaration
        let count = out.matches("const EMAIL_RE").count();
        assert_eq!(count, 1, "EMAIL_RE should be deduplicated");
    }

    // ── OneOf validator test ───────────────────────────────────

    #[test]
    fn guard_js_oneof_validator() {
        let decl = make_guard(
            "G",
            vec![make_field(
                "role",
                "string",
                vec![(
                    "oneOf",
                    vec![str_lit("admin"), str_lit("user"), str_lit("guest")],
                )],
            )],
        );
        let mut e = JsEmitter::new();
        emit_guard_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(out.contains(r#"!["admin", "user", "guest"].includes(data.role)"#));
        assert!(out.contains("must be one of the allowed values"));
    }

    // ── Optional field test ────────────────────────────────────

    #[test]
    fn guard_js_optional_field() {
        let decl = make_guard(
            "G",
            vec![make_field(
                "bio",
                "string",
                vec![("optional", vec![]), ("maxLen", vec![int_lit(500)])],
            )],
        );
        let mut e = JsEmitter::new();
        emit_guard_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(out.contains("if (data.bio != null)"));
        assert!(out.contains("data.bio.length > 500"));
    }

    #[test]
    fn guard_js_nullable_field() {
        let decl = make_guard(
            "G",
            vec![make_field(
                "note",
                "string",
                vec![("nullable", vec![]), ("minLen", vec![int_lit(1)])],
            )],
        );
        let mut e = JsEmitter::new();
        emit_guard_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(out.contains("if (data.note != null)"));
    }

    // ── Transform / sanitize tests ─────────────────────────────

    #[test]
    fn guard_js_trim_generates_sanitize() {
        let decl = make_guard(
            "G",
            vec![make_field(
                "name",
                "string",
                vec![("trim", vec![]), ("minLen", vec![int_lit(1)])],
            )],
        );
        let mut e = JsEmitter::new();
        let meta = emit_guard_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(out.contains("export function sanitizeG(data)"));
        assert!(out.contains("out.name = out.name.trim()"));
        assert!(meta.sanitize_fn.as_deref() == Some("sanitizeG"));
    }

    #[test]
    fn guard_js_precision_generates_sanitize() {
        let decl = make_guard(
            "G",
            vec![make_field(
                "price",
                "float",
                vec![("precision", vec![int_lit(2)])],
            )],
        );
        let mut e = JsEmitter::new();
        emit_guard_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(out.contains("export function sanitizeG(data)"));
        assert!(out.contains("Math.round(out.price * 100)"));
        assert!(out.contains("/ 100"));
    }

    #[test]
    fn guard_js_optional_trim() {
        let decl = make_guard(
            "G",
            vec![make_field(
                "bio",
                "string",
                vec![("optional", vec![]), ("trim", vec![])],
            )],
        );
        let mut e = JsEmitter::new();
        emit_guard_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(out.contains("typeof out.bio === 'string'"));
        assert!(out.contains("out.bio = out.bio.trim()"));
    }

    // ── Custom validator test ──────────────────────────────────

    #[test]
    fn guard_js_custom_validator_callback() {
        let decl = make_guard(
            "G",
            vec![make_field(
                "code",
                "string",
                vec![("custom", vec![str_lit("checkCode")])],
            )],
        );
        let mut e = JsEmitter::new();
        emit_guard_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(out.contains("window.__galeValidators?.checkCode?.(data.code) === false"));
    }

    // ── Metadata tests ─────────────────────────────────────────

    #[test]
    fn guard_js_meta_fields() {
        let decl = make_guard(
            "LoginForm",
            vec![
                make_field("email", "string", vec![("email", vec![])]),
                make_field("password", "string", vec![("minLen", vec![int_lit(8)])]),
            ],
        );
        let mut e = JsEmitter::new();
        let meta = emit_guard_js_file(&mut e, &decl);

        assert_eq!(meta.guard_name, "LoginForm");
        assert_eq!(meta.module_name, "login_form");
        assert_eq!(meta.validate_fn, "validateLoginForm");
        assert!(meta.sanitize_fn.is_none());
        assert_eq!(meta.fields, vec!["email", "password"]);
    }

    #[test]
    fn guard_js_meta_with_sanitize() {
        let decl = make_guard(
            "SignUp",
            vec![make_field("name", "string", vec![("trim", vec![])])],
        );
        let mut e = JsEmitter::new();
        let meta = emit_guard_js_file(&mut e, &decl);

        assert_eq!(meta.sanitize_fn.as_deref(), Some("sanitizeSignUp"));
    }

    // ── Error message parity tests ─────────────────────────────

    #[test]
    fn guard_js_error_messages_match_rust() {
        // Verify key error messages are identical to emit_guard.rs
        let decl = make_guard(
            "G",
            vec![
                make_field("a", "int", vec![("min", vec![int_lit(5)])]),
                make_field("b", "string", vec![("email", vec![])]),
                make_field("c", "string", vec![("nonEmpty", vec![])]),
            ],
        );
        let mut e = JsEmitter::new();
        emit_guard_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(out.contains("must be at least 5"));
        assert!(out.contains("must be a valid email address"));
        assert!(out.contains("must not be empty"));
    }

    // ── Empty guard test ───────────────────────────────────────

    #[test]
    fn empty_guard_js() {
        let decl = make_guard("Empty", vec![]);
        let mut e = JsEmitter::new();
        let meta = emit_guard_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(out.contains("export function validateEmpty(data)"));
        assert!(out.contains("return errors.length === 0"));
        assert_eq!(meta.fields.len(), 0);
    }

    // ── Multiple validators on one field ───────────────────────

    #[test]
    fn guard_js_chained_validators() {
        let decl = make_guard(
            "G",
            vec![make_field(
                "password",
                "string",
                vec![("minLen", vec![int_lit(8)]), ("maxLen", vec![int_lit(100)])],
            )],
        );
        let mut e = JsEmitter::new();
        emit_guard_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(out.contains("data.password.length < 8"));
        assert!(out.contains("data.password.length > 100"));
    }
}
