//! Guard-specific validation (GX0600–GX0632).
//!
//! These functions validate guard field validation chains, checking that:
//! - Chain methods are valid for the field's base type
//! - Numeric arguments are correct
//! - Impossible ranges are detected
//! - Enum constraints are well-formed
//! - Guard references exist and are acyclic
//!
//! Functions here produce [`Diagnostic`] values directly (rather than
//! [`TypeError`]), keeping guard analysis independent of the type-checker's
//! error pipeline. The caller collects diagnostics and merges them.

use std::collections::{HashMap, HashSet};

use smol_str::SmolStr;

use crate::ast::*;
use crate::errors::{codes, Diagnostic};
use crate::span::Span;

// ── Known chain methods ────────────────────────────────────────────────

/// All recognised guard chain method names.
const KNOWN_METHODS: &[&str] = &[
    "min",
    "max",
    "minLen",
    "maxLen",
    "range",
    "email",
    "url",
    "uuid",
    "regex",
    "oneOf",
    "integer",
    "positive",
    "nonNegative",
    "nonEmpty",
    "optional",
    "nullable",
    "trim",
    "lower",
    "upper",
    "precision",
    "default",
    "partial",
    "pick",
    "omit",
    "of",
    "unique",
    "validate",
    "transform",
];

/// Methods that are only valid on `string` types.
const STRING_ONLY: &[&str] = &["email", "url", "uuid", "regex", "trim", "lower", "upper"];

/// Methods that are only valid on `float` types.
const FLOAT_ONLY: &[&str] = &["precision"];

/// Methods that are only valid on numeric (`int` or `float`) types.
const NUMERIC_ONLY: &[&str] = &["positive", "nonNegative"];

/// Methods that are only valid on `array` types.
const ARRAY_ONLY: &[&str] = &["of", "unique"];

// ── Resolved type name ─────────────────────────────────────────────────

/// Rough classification of a guard field's base type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BaseType {
    String,
    Int,
    Float,
    Bool,
    Array,
    /// A named guard / object type.
    Object,
    /// Unknown / unresolvable.
    Unknown,
}

/// Classify a type annotation into a [`BaseType`].
fn classify_type(ann: &TypeAnnotation) -> BaseType {
    match ann {
        TypeAnnotation::Named { name, .. } => match name.as_str() {
            "string" => BaseType::String,
            "int" => BaseType::Int,
            "float" => BaseType::Float,
            "bool" => BaseType::Bool,
            _ => BaseType::Object, // assume named types are guards/objects
        },
        TypeAnnotation::Array { .. } => BaseType::Array,
        _ => BaseType::Unknown,
    }
}

// ── Public validation entry point ──────────────────────────────────────

/// Validate all guard declarations in a program.
///
/// This checks:
/// - Unknown chain methods (GX0600)
/// - Method-type compatibility (GX0605–GX0618)
/// - Numeric argument requirements (GX0601–GX0602)
/// - Impossible min/max ranges (GX0603)
/// - Enum constraints (GX0619–GX0620)
/// - Guard references (GX0626–GX0627)
pub fn validate_guards(program: &Program, diagnostics: &mut Vec<Diagnostic>) {
    let mut guard_names: HashSet<SmolStr> = HashSet::new();

    // First pass: collect all guard names for reference checking.
    collect_guard_names(&program.items, &mut guard_names);

    // Second pass: validate each guard declaration.
    validate_items(&program.items, &guard_names, diagnostics);
}

/// Recursively collect guard names from items (including boundary blocks).
fn collect_guard_names(items: &[Item], names: &mut HashSet<SmolStr>) {
    for item in items {
        match item {
            Item::GuardDecl(g) => {
                names.insert(g.name.clone());
            }
            Item::Out(out) => {
                if let Item::GuardDecl(g) = out.inner.as_ref() {
                    names.insert(g.name.clone());
                }
            }
            Item::ServerBlock(b) | Item::ClientBlock(b) | Item::SharedBlock(b) => {
                collect_guard_names(&b.items, names);
            }
            _ => {}
        }
    }
}

/// Validate guard declarations within a list of items.
fn validate_items(
    items: &[Item],
    guard_names: &HashSet<SmolStr>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for item in items {
        match item {
            Item::GuardDecl(g) => {
                validate_guard_decl(g, guard_names, diagnostics);
            }
            Item::Out(out) => {
                if let Item::GuardDecl(g) = out.inner.as_ref() {
                    validate_guard_decl(g, guard_names, diagnostics);
                }
            }
            Item::ServerBlock(b) | Item::ClientBlock(b) | Item::SharedBlock(b) => {
                validate_items(&b.items, guard_names, diagnostics);
            }
            _ => {}
        }
    }
}

/// Validate a single guard declaration.
fn validate_guard_decl(
    guard: &GuardDecl,
    guard_names: &HashSet<SmolStr>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for field in &guard.fields {
        let base = classify_type(&field.ty);

        // Check for guard field type references (GX0626)
        check_guard_type_reference(&field.ty, guard_names, diagnostics);

        // Track min/max values for impossible range detection (GX0603)
        let mut min_value: Option<i64> = None;
        let mut max_value: Option<i64> = None;

        // Track enum variants for duplicate detection (GX0620)
        let mut enum_variants: HashSet<String> = HashSet::new();

        for validator in &field.validators {
            let method = validator.name.as_str();

            // GX0600: Unknown chain method
            if !KNOWN_METHODS.contains(&method) {
                diagnostics.push(
                    Diagnostic::with_message(
                        &codes::GX0600,
                        format!(
                            "unknown guard chain method `.{}()` on field `{}`",
                            method, field.name
                        ),
                        validator.span,
                    )
                    .with_hint(format!(
                        "`.{}()` is not a recognised validation method",
                        method
                    )),
                );
                continue;
            }

            // GX0601: `.min()` requires numeric argument
            if method == "min" {
                match validator.args.first() {
                    Some(Expr::IntLit { value, .. }) => {
                        min_value = Some(*value);
                    }
                    Some(Expr::FloatLit { .. }) => {
                        // floats are fine for min
                    }
                    _ => {
                        diagnostics.push(Diagnostic::with_message(
                            &codes::GX0601,
                            format!(
                                "`.min()` on field `{}` requires a numeric argument",
                                field.name
                            ),
                            validator.span,
                        ));
                    }
                }
            }

            // GX0602: `.max()` requires numeric argument
            if method == "max" {
                match validator.args.first() {
                    Some(Expr::IntLit { value, .. }) => {
                        max_value = Some(*value);
                    }
                    Some(Expr::FloatLit { .. }) => {
                        // floats are fine for max
                    }
                    _ => {
                        diagnostics.push(Diagnostic::with_message(
                            &codes::GX0602,
                            format!(
                                "`.max()` on field `{}` requires a numeric argument",
                                field.name
                            ),
                            validator.span,
                        ));
                    }
                }
            }

            // GX0605–GX0608: String-only methods on non-string
            if STRING_ONLY.contains(&method) && base != BaseType::String {
                let code = match method {
                    "email" => &codes::GX0605,
                    "url" => &codes::GX0606,
                    "uuid" => &codes::GX0607,
                    "regex" => &codes::GX0608,
                    "trim" => &codes::GX0616,
                    "lower" => &codes::GX0617,
                    "upper" => &codes::GX0618,
                    _ => &codes::GX0600,
                };
                diagnostics.push(
                    Diagnostic::with_message(
                        code,
                        format!(
                            "`.{}()` is only valid on `string` type, but field `{}` has type `{}`",
                            method,
                            field.name,
                            type_annotation_name(&field.ty)
                        ),
                        validator.span,
                    )
                    .with_help("Change the field type to `string`, or remove this validator."),
                );
            }

            // GX0610: `.precision()` only valid on float
            if FLOAT_ONLY.contains(&method) && base != BaseType::Float {
                diagnostics.push(
                    Diagnostic::with_message(
                        &codes::GX0610,
                        format!(
                            "`.precision()` is only valid on `float`, but field `{}` has type `{}`",
                            field.name,
                            type_annotation_name(&field.ty)
                        ),
                        validator.span,
                    )
                    .with_help("Change the field type to `float`, or remove `.precision()`."),
                );
            }

            // GX0611: `.positive()` / `.nonNegative()` only valid on int or float
            if NUMERIC_ONLY.contains(&method) && base != BaseType::Int && base != BaseType::Float {
                diagnostics.push(
                    Diagnostic::with_message(
                        &codes::GX0611,
                        format!(
                            "`.{}()` is only valid on `int` or `float`, but field `{}` has type `{}`",
                            method,
                            field.name,
                            type_annotation_name(&field.ty)
                        ),
                        validator.span,
                    )
                    .with_help("Change the field type to `int` or `float`, or remove this validator."),
                );
            }

            // GX0614–GX0615: Array-only methods on non-array
            if ARRAY_ONLY.contains(&method) && base != BaseType::Array {
                let code = match method {
                    "of" => &codes::GX0614,
                    "unique" => &codes::GX0615,
                    _ => &codes::GX0600,
                };
                diagnostics.push(
                    Diagnostic::with_message(
                        code,
                        format!(
                            "`.{}()` is only valid on `array`, but field `{}` has type `{}`",
                            method,
                            field.name,
                            type_annotation_name(&field.ty)
                        ),
                        validator.span,
                    )
                    .with_help("Change the field type to an array type, or remove this validator."),
                );
            }

            // GX0619: `oneOf()` requires at least one value
            if method == "oneOf" && validator.args.is_empty() {
                diagnostics.push(Diagnostic::with_message(
                    &codes::GX0619,
                    format!(
                        "`.oneOf()` on field `{}` requires at least one value",
                        field.name
                    ),
                    validator.span,
                ));
            }

            // GX0620: Duplicate enum variant
            if method == "oneOf" {
                for arg in &validator.args {
                    let variant_str = match arg {
                        Expr::StringLit { value, .. } => value.to_string(),
                        Expr::IntLit { value, .. } => value.to_string(),
                        Expr::BoolLit { value, .. } => value.to_string(),
                        _ => continue,
                    };
                    if !enum_variants.insert(variant_str.clone()) {
                        diagnostics.push(Diagnostic::with_message(
                            &codes::GX0620,
                            format!(
                                "duplicate enum variant `{}` in `.oneOf()` on field `{}`",
                                variant_str, field.name
                            ),
                            validator.span,
                        ));
                    }
                }
            }
        }

        // GX0603: `.min()` exceeds `.max()` — impossible range
        if let (Some(min), Some(max)) = (min_value, max_value) {
            if min > max {
                diagnostics.push(
                    Diagnostic::with_message(
                        &codes::GX0603,
                        format!(
                            "impossible range on field `{}`: min({}) > max({})",
                            field.name, min, max
                        ),
                        field.span,
                    )
                    .with_help("Swap the min and max values, or correct the range."),
                );
            }
        }
    }

    // GX0627: Circular guard reference (self-reference check)
    check_circular_guard_reference(guard, diagnostics);
}

/// Check if a type annotation references a guard that doesn't exist (GX0626).
fn check_guard_type_reference(
    ann: &TypeAnnotation,
    guard_names: &HashSet<SmolStr>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match ann {
        TypeAnnotation::Named { name, span } => {
            // Skip built-in primitive types
            let builtins = [
                "string", "int", "float", "bool", "void", "null", "never", "datetime",
            ];
            if !builtins.contains(&name.as_str()) && !guard_names.contains(name) {
                // This might be a type alias or external type — only flag if it
                // looks like a guard name (PascalCase). We use a heuristic:
                // starts with uppercase.
                if name.chars().next().is_some_and(|c| c.is_uppercase()) {
                    diagnostics.push(
                        Diagnostic::with_message(
                            &codes::GX0626,
                            format!("guard `{}` is not defined", name),
                            *span,
                        )
                        .with_hint(
                            "Ensure the guard is declared before use, or check the spelling.",
                        ),
                    );
                }
            }
        }
        TypeAnnotation::Array { element, .. } => {
            check_guard_type_reference(element, guard_names, diagnostics);
        }
        TypeAnnotation::Union { types, .. } => {
            for t in types {
                check_guard_type_reference(t, guard_names, diagnostics);
            }
        }
        TypeAnnotation::Optional { inner, .. } => {
            check_guard_type_reference(inner, guard_names, diagnostics);
        }
        _ => {}
    }
}

/// Check for circular self-references in a guard (GX0627).
///
/// A guard field whose type annotation references the guard itself creates
/// an infinite structure. (Cross-guard cycles require a global graph which
/// is handled at a higher level.)
fn check_circular_guard_reference(guard: &GuardDecl, diagnostics: &mut Vec<Diagnostic>) {
    for field in &guard.fields {
        if type_references_name(&field.ty, &guard.name) {
            diagnostics.push(
                Diagnostic::with_message(
                    &codes::GX0627,
                    format!(
                        "field `{}` in guard `{}` creates a circular reference",
                        field.name, guard.name
                    ),
                    field.span,
                )
                .with_help(
                    "Use an array or optional type to break the cycle, e.g. `children: MyGuard[]`",
                ),
            );
        }
    }
}

/// Check if a type annotation references a specific name (non-recursively through arrays).
fn type_references_name(ann: &TypeAnnotation, name: &str) -> bool {
    match ann {
        TypeAnnotation::Named { name: n, .. } => n.as_str() == name,
        // Arrays break the cycle (they're indirect references), so we skip them
        TypeAnnotation::Array { .. } => false,
        TypeAnnotation::Union { types, .. } => types.iter().any(|t| type_references_name(t, name)),
        TypeAnnotation::Optional { inner, .. } => {
            // Optional doesn't break the cycle
            type_references_name(inner, name)
        }
        _ => false,
    }
}

/// Extract a human-readable name from a type annotation.
fn type_annotation_name(ann: &TypeAnnotation) -> String {
    match ann {
        TypeAnnotation::Named { name, .. } => name.to_string(),
        TypeAnnotation::Array { element, .. } => format!("{}[]", type_annotation_name(element)),
        TypeAnnotation::Union { types, .. } => types
            .iter()
            .map(type_annotation_name)
            .collect::<Vec<_>>()
            .join(" | "),
        TypeAnnotation::Optional { inner, .. } => format!("{}?", type_annotation_name(inner)),
        TypeAnnotation::StringLiteral { value, .. } => format!("\"{}\"", value),
        TypeAnnotation::Function { .. } => "fn(...)".into(),
        TypeAnnotation::Tuple { elements, .. } => {
            let parts: Vec<_> = elements.iter().map(type_annotation_name).collect();
            format!("({})", parts.join(", "))
        }
        TypeAnnotation::Object { .. } => "{ ... }".into(),
    }
}

// ── Cross-guard cycle detection ────────────────────────────────────────

/// Detect cycles across multiple guards using a dependency graph.
///
/// Call this after all guards have been parsed to find cases like:
/// ```text
/// guard A { b: B }
/// guard B { a: A }
/// ```
pub fn detect_guard_cycles(program: &Program, diagnostics: &mut Vec<Diagnostic>) {
    // Build a dependency map: guard name → set of guard names it references.
    let mut deps: HashMap<SmolStr, Vec<(SmolStr, Span)>> = HashMap::new();
    let mut guard_spans: HashMap<SmolStr, Span> = HashMap::new();

    collect_guard_deps(&program.items, &mut deps, &mut guard_spans);

    // DFS-based cycle detection.
    let mut visited: HashSet<SmolStr> = HashSet::new();
    let mut stack: HashSet<SmolStr> = HashSet::new();

    for name in deps.keys() {
        if !visited.contains(name) {
            detect_cycle_dfs(
                name,
                &deps,
                &guard_spans,
                &mut visited,
                &mut stack,
                diagnostics,
            );
        }
    }
}

fn collect_guard_deps(
    items: &[Item],
    deps: &mut HashMap<SmolStr, Vec<(SmolStr, Span)>>,
    spans: &mut HashMap<SmolStr, Span>,
) {
    for item in items {
        match item {
            Item::GuardDecl(g) => {
                spans.insert(g.name.clone(), g.span);
                let field_refs = g
                    .fields
                    .iter()
                    .filter_map(|f| extract_guard_ref(&f.ty).map(|(n, s)| (n, s)))
                    .collect();
                deps.insert(g.name.clone(), field_refs);
            }
            Item::Out(out) => {
                if let Item::GuardDecl(g) = out.inner.as_ref() {
                    spans.insert(g.name.clone(), g.span);
                    let field_refs = g
                        .fields
                        .iter()
                        .filter_map(|f| extract_guard_ref(&f.ty).map(|(n, s)| (n, s)))
                        .collect();
                    deps.insert(g.name.clone(), field_refs);
                }
            }
            Item::ServerBlock(b) | Item::ClientBlock(b) | Item::SharedBlock(b) => {
                collect_guard_deps(&b.items, deps, spans);
            }
            _ => {}
        }
    }
}

/// Extract a direct (non-array) guard type reference from a type annotation.
fn extract_guard_ref(ann: &TypeAnnotation) -> Option<(SmolStr, Span)> {
    match ann {
        TypeAnnotation::Named { name, span } => {
            let builtins = [
                "string", "int", "float", "bool", "void", "null", "never", "datetime",
            ];
            if !builtins.contains(&name.as_str())
                && name.chars().next().is_some_and(|c| c.is_uppercase())
            {
                Some((name.clone(), *span))
            } else {
                None
            }
        }
        // Arrays break the reference cycle — skip.
        TypeAnnotation::Array { .. } => None,
        TypeAnnotation::Optional { inner, .. } => extract_guard_ref(inner),
        TypeAnnotation::Union { types, .. } => {
            // Return the first guard reference found.
            types.iter().find_map(extract_guard_ref)
        }
        _ => None,
    }
}

fn detect_cycle_dfs(
    node: &SmolStr,
    deps: &HashMap<SmolStr, Vec<(SmolStr, Span)>>,
    spans: &HashMap<SmolStr, Span>,
    visited: &mut HashSet<SmolStr>,
    stack: &mut HashSet<SmolStr>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    visited.insert(node.clone());
    stack.insert(node.clone());

    if let Some(neighbours) = deps.get(node) {
        for (dep, dep_span) in neighbours {
            if stack.contains(dep) {
                // Cycle found!
                let span = spans.get(node).copied().unwrap_or(*dep_span);
                diagnostics.push(
                    Diagnostic::with_message(
                        &codes::GX0627,
                        format!(
                            "circular guard reference: `{}` references `{}` which leads back to `{}`",
                            node, dep, node
                        ),
                        span,
                    )
                    .with_help("Break the cycle using an array type or optional type."),
                );
            } else if !visited.contains(dep) {
                detect_cycle_dfs(dep, deps, spans, visited, stack, diagnostics);
            }
        }
    }

    stack.remove(node);
}
