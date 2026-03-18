//! Shared type and function emitters.
//!
//! Generates Rust files for items declared in `shared {}` blocks:
//! - **Enums** → `#[derive(…)] pub enum Name { Variant, … }`
//! - **Type aliases** → `pub type Name = T;` or `pub struct Name { … }` for object types
//! - **Functions** → `pub fn name(…) → T { body }`

use crate::ast::*;
use crate::codegen::emit_stmt::{annotation_to_rust, emit_block_body};
use crate::codegen::rust_emitter::RustEmitter;
use crate::codegen::types::to_snake_case;

// ── Enum emitter ───────────────────────────────────────────────────────

/// Emit a complete shared enum Rust file.
///
/// Produces a `#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]`
/// enum with unit variants.
pub fn emit_enum_file(e: &mut RustEmitter, decl: &EnumDecl) {
    e.emit_file_header(&format!("Shared enum: `{}`.", decl.name));
    e.newline();

    e.emit_use("serde::{Deserialize, Serialize}");
    e.newline();

    let variants: Vec<&str> = decl.variants.iter().map(|v| v.as_str()).collect();
    e.emit_unit_enum(
        &["derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)"],
        "pub",
        &decl.name,
        &variants,
    );
}

// ── Type alias / struct emitter ────────────────────────────────────────

/// Emit a complete shared type alias or struct Rust file.
///
/// - For `TypeAnnotation::Object { fields }` → generates a struct with derives.
/// - For all other annotations → generates a `pub type Name = T;` alias.
pub fn emit_type_alias_file(e: &mut RustEmitter, decl: &TypeAliasDecl) {
    e.emit_file_header(&format!("Shared type: `{}`.", decl.name));
    e.newline();

    match &decl.ty {
        TypeAnnotation::Object { fields, .. } => {
            e.emit_use("serde::{Deserialize, Serialize}");
            e.newline();
            emit_object_struct(e, &decl.name, fields);
        }
        other => {
            let rust_ty = annotation_to_rust(other);
            e.writeln(&format!("pub type {} = {};", decl.name, rust_ty));
        }
    }
}

/// Emit a struct from an object type annotation.
fn emit_object_struct(e: &mut RustEmitter, name: &str, fields: &[ObjectTypeField]) {
    e.emit_attribute("derive(Debug, Clone, Serialize, Deserialize)");
    e.block(&format!("pub struct {name}"), |e| {
        for field in fields {
            let field_name = to_snake_case(&field.name);
            let ty = annotation_to_rust(&field.ty);
            let ty = if field.optional {
                format!("Option<{ty}>")
            } else {
                ty
            };
            e.writeln(&format!("pub {field_name}: {ty},"));
        }
    });
}

// ── Shared function emitter ────────────────────────────────────────────

/// Emit a complete shared function Rust file.
///
/// The function is emitted with `pub` visibility so it can be imported
/// from other generated modules.
pub fn emit_shared_fn_file(e: &mut RustEmitter, decl: &FnDecl) {
    e.emit_file_header(&format!("Shared function: `{}`.", decl.name));
    e.newline();

    let name = to_snake_case(&decl.name);

    // Build params list
    let params: Vec<(String, String)> = decl
        .params
        .iter()
        .map(|p| {
            let pname = to_snake_case(&p.name);
            let ty = if let Some(ann) = &p.ty_ann {
                annotation_to_rust(ann)
            } else {
                "serde_json::Value".into()
            };
            (pname, ty)
        })
        .collect();

    let param_refs: Vec<(&str, &str)> = params
        .iter()
        .map(|(n, t)| (n.as_str(), t.as_str()))
        .collect();

    // Return type
    let ret_ty = decl.ret_ty.as_ref().map(annotation_to_rust);
    let ret_ref = ret_ty.as_deref();

    e.emit_fn("pub", decl.is_async, &name, &param_refs, ret_ref, |e| {
        emit_block_body(e, &decl.body);
    });
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::span::Span;

    fn s() -> Span {
        Span::dummy()
    }

    fn emit_enum(decl: &EnumDecl) -> String {
        let mut e = RustEmitter::new();
        emit_enum_file(&mut e, decl);
        e.finish()
    }

    fn emit_alias(decl: &TypeAliasDecl) -> String {
        let mut e = RustEmitter::new();
        emit_type_alias_file(&mut e, decl);
        e.finish()
    }

    fn emit_fn(decl: &FnDecl) -> String {
        let mut e = RustEmitter::new();
        emit_shared_fn_file(&mut e, decl);
        e.finish()
    }

    // ── Enum tests ─────────────────────────────────────────────

    #[test]
    fn enum_simple() {
        let out = emit_enum(&EnumDecl {
            name: "Status".into(),
            variants: vec!["Active".into()],
            span: s(),
        });
        assert!(out.contains("Shared enum: `Status`"));
        assert!(out.contains("use serde::{Deserialize, Serialize};"));
        assert!(out.contains("derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)"));
        assert!(out.contains("pub enum Status"));
        assert!(out.contains("Active,"));
    }

    #[test]
    fn enum_multiple_variants() {
        let out = emit_enum(&EnumDecl {
            name: "Role".into(),
            variants: vec!["Admin".into(), "User".into(), "Guest".into()],
            span: s(),
        });
        assert!(out.contains("pub enum Role"));
        assert!(out.contains("Admin,"));
        assert!(out.contains("User,"));
        assert!(out.contains("Guest,"));
    }

    // ── Type alias tests ───────────────────────────────────────

    #[test]
    fn type_alias_primitive() {
        let out = emit_alias(&TypeAliasDecl {
            name: "UserId".into(),
            ty: TypeAnnotation::Named {
                name: "int".into(),
                span: s(),
            },
            span: s(),
        });
        assert!(out.contains("Shared type: `UserId`"));
        assert!(out.contains("pub type UserId = i64;"));
        // No serde import needed for simple alias
        assert!(!out.contains("serde"));
    }

    #[test]
    fn type_alias_array() {
        let out = emit_alias(&TypeAliasDecl {
            name: "Tags".into(),
            ty: TypeAnnotation::Array {
                element: Box::new(TypeAnnotation::Named {
                    name: "string".into(),
                    span: s(),
                }),
                span: s(),
            },
            span: s(),
        });
        assert!(out.contains("pub type Tags = Vec<String>;"));
    }

    #[test]
    fn type_alias_optional() {
        let out = emit_alias(&TypeAliasDecl {
            name: "MaybeCount".into(),
            ty: TypeAnnotation::Optional {
                inner: Box::new(TypeAnnotation::Named {
                    name: "int".into(),
                    span: s(),
                }),
                span: s(),
            },
            span: s(),
        });
        assert!(out.contains("pub type MaybeCount = Option<i64>;"));
    }

    #[test]
    fn type_alias_object() {
        let out = emit_alias(&TypeAliasDecl {
            name: "UserData".into(),
            ty: TypeAnnotation::Object {
                fields: vec![
                    ObjectTypeField {
                        name: "name".into(),
                        ty: TypeAnnotation::Named {
                            name: "string".into(),
                            span: s(),
                        },
                        optional: false,
                        span: s(),
                    },
                    ObjectTypeField {
                        name: "age".into(),
                        ty: TypeAnnotation::Named {
                            name: "int".into(),
                            span: s(),
                        },
                        optional: false,
                        span: s(),
                    },
                ],
                span: s(),
            },
            span: s(),
        });
        assert!(out.contains("use serde::{Deserialize, Serialize};"));
        assert!(out.contains("derive(Debug, Clone, Serialize, Deserialize)"));
        assert!(out.contains("pub struct UserData"));
        assert!(out.contains("pub name: String,"));
        assert!(out.contains("pub age: i64,"));
    }

    #[test]
    fn type_alias_object_optional_field() {
        let out = emit_alias(&TypeAliasDecl {
            name: "Profile".into(),
            ty: TypeAnnotation::Object {
                fields: vec![ObjectTypeField {
                    name: "bio".into(),
                    ty: TypeAnnotation::Named {
                        name: "string".into(),
                        span: s(),
                    },
                    optional: true,
                    span: s(),
                }],
                span: s(),
            },
            span: s(),
        });
        assert!(out.contains("pub bio: Option<String>,"));
    }

    // ── Shared function tests ──────────────────────────────────

    #[test]
    fn shared_fn_simple() {
        let out = emit_fn(&FnDecl {
            name: "formatName".into(),
            params: vec![
                Param {
                    name: "first".into(),
                    ty_ann: Some(TypeAnnotation::Named {
                        name: "string".into(),
                        span: s(),
                    }),
                    default: None,
                    span: s(),
                },
                Param {
                    name: "last".into(),
                    ty_ann: Some(TypeAnnotation::Named {
                        name: "string".into(),
                        span: s(),
                    }),
                    default: None,
                    span: s(),
                },
            ],
            ret_ty: Some(TypeAnnotation::Named {
                name: "string".into(),
                span: s(),
            }),
            body: Block {
                stmts: vec![Stmt::Return {
                    value: Some(Expr::StringLit {
                        value: "name".into(),
                        span: s(),
                    }),
                    span: s(),
                }],
                span: s(),
            },
            is_async: false,
            span: s(),
        });
        assert!(out.contains("Shared function: `formatName`"));
        assert!(out.contains("pub fn format_name(first: String, last: String) -> String"));
        assert!(out.contains("return"));
    }

    #[test]
    fn shared_fn_async() {
        let out = emit_fn(&FnDecl {
            name: "fetchData".into(),
            params: vec![],
            ret_ty: Some(TypeAnnotation::Named {
                name: "string".into(),
                span: s(),
            }),
            body: Block {
                stmts: vec![],
                span: s(),
            },
            is_async: true,
            span: s(),
        });
        assert!(out.contains("pub async fn fetch_data() -> String"));
    }

    #[test]
    fn shared_fn_no_return_type() {
        let out = emit_fn(&FnDecl {
            name: "logMessage".into(),
            params: vec![Param {
                name: "msg".into(),
                ty_ann: Some(TypeAnnotation::Named {
                    name: "string".into(),
                    span: s(),
                }),
                default: None,
                span: s(),
            }],
            ret_ty: None,
            body: Block {
                stmts: vec![],
                span: s(),
            },
            is_async: false,
            span: s(),
        });
        assert!(out.contains("pub fn log_message(msg: String)"));
        // No return type in output
        assert!(!out.contains("->"));
    }
}
