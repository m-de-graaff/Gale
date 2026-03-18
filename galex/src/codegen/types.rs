//! GaleX type → Rust type string mapping.
//!
//! Converts resolved [`TypeData`] from the type interner into valid Rust
//! type syntax strings for use in generated code.

use std::collections::HashSet;

use crate::ast::TypeAnnotation;
use crate::types::ty::{TypeData, TypeId, TypeInterner};

/// Convert a resolved GaleX type to its Rust equivalent as a string.
///
/// This is the primary entry point for type mapping. It handles all
/// [`TypeData`] variants and recurses for compound types.
///
/// # Client-only types
///
/// Types that exist only on the client side (`Signal`, `Derived`, `Store`,
/// `Component`, `DomRef`) have no Rust representation and map to
/// `serde_json::Value` as a fallback.
pub fn type_to_rust(interner: &TypeInterner, ty: TypeId) -> String {
    match interner.get(ty) {
        // ── Primitives ─────────────────────────────────────────
        TypeData::String => "String".into(),
        TypeData::Int => "i64".into(),
        TypeData::Float => "f64".into(),
        TypeData::Bool => "bool".into(),
        TypeData::Void => "()".into(),
        TypeData::Null => "()".into(),
        TypeData::Never => "std::convert::Infallible".into(),

        // ── Literal types (erase to base) ──────────────────────
        TypeData::StringLiteral(_) => "String".into(),
        TypeData::IntLiteral(_) => "i64".into(),

        // ── Compound types ─────────────────────────────────────
        TypeData::Array(elem) => {
            let inner = type_to_rust(interner, *elem);
            format!("Vec<{inner}>")
        }
        TypeData::Tuple(elems) => {
            let parts: Vec<String> = elems.iter().map(|e| type_to_rust(interner, *e)).collect();
            format!("({})", parts.join(", "))
        }
        TypeData::Optional(inner) => {
            let inner_str = type_to_rust(interner, *inner);
            format!("Option<{inner_str}>")
        }
        TypeData::Union(_) => {
            // Heterogeneous unions have no direct Rust equivalent;
            // fall back to dynamic JSON. Future: string-literal unions
            // could become Rust enums with #[serde(rename)] variants.
            "serde_json::Value".into()
        }
        TypeData::Object(_) => {
            // Anonymous object types use dynamic JSON.
            // Named guards get their own generated structs and are handled
            // via the Guard variant below.
            "serde_json::Value".into()
        }
        TypeData::Function(sig) => {
            let params: Vec<String> = sig
                .params
                .iter()
                .map(|p| type_to_rust(interner, p.ty))
                .collect();
            let ret = type_to_rust(interner, sig.ret);
            if sig.is_async {
                // Async functions use Pin<Box<dyn Future>> in trait contexts,
                // but for codegen we represent as a simple fn pointer.
                format!("fn({}) -> {ret}", params.join(", "))
            } else {
                format!("fn({}) -> {ret}", params.join(", "))
            }
        }

        // ── GaleX-specific types ───────────────────────────────
        TypeData::Guard(def) => def.name.to_string(),
        TypeData::Enum(def) => def.name.to_string(),

        TypeData::Query { result } => {
            // Queries resolve to their result type on the Rust side
            type_to_rust(interner, *result)
        }
        TypeData::Channel(_) => {
            // Channels don't have a single Rust type; they're emitted
            // as WebSocket handler infrastructure. Placeholder for references.
            "()".into()
        }

        // ── Client-only types (no Rust representation) ─────────
        TypeData::Signal(inner) => type_to_rust(interner, *inner),
        TypeData::Derived(inner) => type_to_rust(interner, *inner),
        TypeData::Store(_) | TypeData::Component(_) | TypeData::DomRef(_) => {
            "serde_json::Value".into()
        }

        // ── Inference artifacts (should be resolved, but handle gracefully)
        TypeData::TypeVar(_) | TypeData::Named(_) => "serde_json::Value".into(),
    }
}

/// Convert function parameters to Rust `(name, type)` pairs.
pub fn fn_params_to_rust(
    interner: &TypeInterner,
    params: &[crate::types::ty::FnParam],
) -> Vec<(String, String)> {
    params
        .iter()
        .map(|p| (to_snake_case(&p.name), type_to_rust(interner, p.ty)))
        .collect()
}

/// Convert guard fields to Rust `(name, type, optional)` triples.
pub fn guard_fields_to_rust(
    interner: &TypeInterner,
    fields: &[crate::types::ty::GuardField],
) -> Vec<(String, String, bool)> {
    fields
        .iter()
        .map(|f| {
            let is_optional = f
                .validations
                .iter()
                .any(|v| matches!(v, crate::types::validation::Validation::Optional));
            let ty = type_to_rust(interner, f.ty);
            let rust_ty = if is_optional {
                format!("Option<{ty}>")
            } else {
                ty
            };
            (to_snake_case(&f.name), rust_ty, is_optional)
        })
        .collect()
}

/// Check if a TypeData variant is client-only (no server-side representation).
pub fn is_client_only_type(data: &TypeData) -> bool {
    matches!(
        data,
        TypeData::Signal(_)
            | TypeData::Derived(_)
            | TypeData::Store(_)
            | TypeData::Component(_)
            | TypeData::DomRef(_)
    )
}

/// Convert a GaleX camelCase name to Rust snake_case.
pub fn to_snake_case(name: &str) -> String {
    let mut result = String::with_capacity(name.len() + 4);
    for (i, ch) in name.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(ch.to_lowercase().next().unwrap_or(ch));
        } else {
            result.push(ch);
        }
    }
    result
}

/// Convert a GaleX PascalCase name to a Rust snake_case module filename.
///
/// E.g. `CreateUser` → `create_user`, `LoginForm` → `login_form`.
pub fn to_module_name(name: &str) -> String {
    to_snake_case(name)
}

// ── Shared type reference scanning ─────────────────────────────────────

/// Collect names of shared types referenced in a set of type annotations.
///
/// Walks each `TypeAnnotation` recursively, collecting any `Named` type
/// whose name appears in `known_shared`. Returns a sorted, deduplicated
/// list of shared type names found.
pub fn collect_shared_type_refs(
    annotations: &[&TypeAnnotation],
    known_shared: &HashSet<String>,
) -> Vec<String> {
    let mut found = Vec::new();
    for ann in annotations {
        collect_from_annotation(ann, known_shared, &mut found);
    }
    found.sort();
    found.dedup();
    found
}

/// Recursively walk a type annotation collecting shared type references.
fn collect_from_annotation(ann: &TypeAnnotation, known: &HashSet<String>, found: &mut Vec<String>) {
    match ann {
        TypeAnnotation::Named { name, .. } => {
            if known.contains(name.as_str()) {
                found.push(name.to_string());
            }
        }
        TypeAnnotation::Array { element, .. } => {
            collect_from_annotation(element, known, found);
        }
        TypeAnnotation::Optional { inner, .. } => {
            collect_from_annotation(inner, known, found);
        }
        TypeAnnotation::Tuple { elements, .. } => {
            for el in elements {
                collect_from_annotation(el, known, found);
            }
        }
        TypeAnnotation::Union { types, .. } => {
            for ty in types {
                collect_from_annotation(ty, known, found);
            }
        }
        TypeAnnotation::Function { params, ret, .. } => {
            for p in params {
                collect_from_annotation(p, known, found);
            }
            collect_from_annotation(ret, known, found);
        }
        TypeAnnotation::Object { fields, .. } => {
            for f in fields {
                collect_from_annotation(&f.ty, known, found);
            }
        }
        TypeAnnotation::StringLiteral { .. } => {}
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::TypeAnnotation;
    use crate::span::Span;
    use crate::types::ty::TypeInterner;

    #[test]
    fn primitives() {
        let int = TypeInterner::new();
        assert_eq!(type_to_rust(&int, int.string), "String");
        assert_eq!(type_to_rust(&int, int.int), "i64");
        assert_eq!(type_to_rust(&int, int.float), "f64");
        assert_eq!(type_to_rust(&int, int.bool_), "bool");
        assert_eq!(type_to_rust(&int, int.void), "()");
        assert_eq!(type_to_rust(&int, int.null), "()");
        assert_eq!(type_to_rust(&int, int.never), "std::convert::Infallible");
    }

    #[test]
    fn array_type() {
        let mut int = TypeInterner::new();
        let arr = int.make_array(int.int);
        assert_eq!(type_to_rust(&int, arr), "Vec<i64>");
    }

    #[test]
    fn optional_type() {
        let mut int = TypeInterner::new();
        let opt = int.make_optional(int.string);
        assert_eq!(type_to_rust(&int, opt), "Option<String>");
    }

    #[test]
    fn nested_compound() {
        let mut int = TypeInterner::new();
        let inner_opt = int.make_optional(int.int);
        let arr = int.make_array(inner_opt);
        assert_eq!(type_to_rust(&int, arr), "Vec<Option<i64>>");
    }

    #[test]
    fn tuple_type() {
        let mut int = TypeInterner::new();
        let tup = int.intern(TypeData::Tuple(vec![int.int, int.string, int.bool_]));
        assert_eq!(type_to_rust(&int, tup), "(i64, String, bool)");
    }

    #[test]
    fn guard_reference() {
        let mut int = TypeInterner::new();
        let guard = int.make_guard(crate::types::ty::GuardDef {
            name: "LoginForm".into(),
            fields: vec![],
            extends: None,
            has_validators: false,
        });
        assert_eq!(type_to_rust(&int, guard), "LoginForm");
    }

    #[test]
    fn enum_reference() {
        let mut int = TypeInterner::new();
        let en = int.intern(TypeData::Enum(crate::types::ty::EnumDef {
            name: "Status".into(),
            variants: vec!["Active".into(), "Inactive".into()],
        }));
        assert_eq!(type_to_rust(&int, en), "Status");
    }

    #[test]
    fn string_literal_erases() {
        let mut int = TypeInterner::new();
        let lit = int.make_string_literal("primary");
        assert_eq!(type_to_rust(&int, lit), "String");
    }

    #[test]
    fn snake_case_conversion() {
        assert_eq!(to_snake_case("createUser"), "create_user");
        assert_eq!(to_snake_case("LoginForm"), "login_form");
        assert_eq!(to_snake_case("simple"), "simple");
        assert_eq!(to_snake_case("HTMLElement"), "h_t_m_l_element");
        assert_eq!(to_snake_case("a"), "a");
    }

    #[test]
    fn client_only_detection() {
        assert!(is_client_only_type(&TypeData::Signal(TypeId::from_raw(0))));
        assert!(is_client_only_type(&TypeData::Derived(TypeId::from_raw(0))));
        assert!(!is_client_only_type(&TypeData::String));
        assert!(!is_client_only_type(&TypeData::Int));
    }

    fn s() -> Span {
        Span::dummy()
    }

    fn shared(names: &[&str]) -> HashSet<String> {
        names.iter().map(|n| n.to_string()).collect()
    }

    #[test]
    fn collect_shared_refs_named() {
        let known = shared(&["Status", "Role"]);
        let ann = TypeAnnotation::Named {
            name: "Status".into(),
            span: s(),
        };
        let refs = collect_shared_type_refs(&[&ann], &known);
        assert_eq!(refs, vec!["Status"]);
    }

    #[test]
    fn collect_shared_refs_nested() {
        let known = shared(&["Role"]);
        let ann = TypeAnnotation::Array {
            element: Box::new(TypeAnnotation::Optional {
                inner: Box::new(TypeAnnotation::Named {
                    name: "Role".into(),
                    span: s(),
                }),
                span: s(),
            }),
            span: s(),
        };
        let refs = collect_shared_type_refs(&[&ann], &known);
        assert_eq!(refs, vec!["Role"]);
    }

    #[test]
    fn collect_shared_refs_none_for_primitives() {
        let known = shared(&["Status"]);
        let ann = TypeAnnotation::Named {
            name: "string".into(),
            span: s(),
        };
        let refs = collect_shared_type_refs(&[&ann], &known);
        assert!(refs.is_empty());
    }

    #[test]
    fn collect_shared_refs_deduplicates() {
        let known = shared(&["Status"]);
        let a1 = TypeAnnotation::Named {
            name: "Status".into(),
            span: s(),
        };
        let a2 = TypeAnnotation::Named {
            name: "Status".into(),
            span: s(),
        };
        let refs = collect_shared_type_refs(&[&a1, &a2], &known);
        assert_eq!(refs, vec!["Status"]);
    }
}
