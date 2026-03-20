//! Action endpoint handler generator.
//!
//! For each GaleX [`ActionDecl`], generates a Rust file containing:
//! - An input struct (from action params)
//! - An async Axum handler function
//! - Guard validation (if the param type is a known guard)
//! - JSON request/response (de)serialization

use std::collections::HashSet;

use crate::ast::*;
use crate::codegen::emit_stmt::annotation_to_rust;
use crate::codegen::rust_emitter::RustEmitter;
use crate::codegen::types::{collect_shared_type_refs, to_module_name, to_snake_case};

/// Emit a complete action handler Rust file.
///
/// `known_guards` is the set of guard names declared in the program —
/// used to detect when an action param is guard-typed (triggers validation).
///
/// `guards_with_transforms` lists guard names that have `.trim()`, `.precision()`,
/// or `.default()` validators — these need a `sanitize()` call before `validate()`.
pub fn emit_action_file(
    e: &mut RustEmitter,
    decl: &ActionDecl,
    known_guards: &HashSet<String>,
    guards_with_transforms: &HashSet<String>,
    known_shared_types: &HashSet<String>,
) {
    let action_name = &decl.name;
    e.emit_file_header(&format!("Action handler: `{action_name}`."));
    e.newline();

    // Determine if any param is a guard (triggers validation import + call)
    let guard_param = find_guard_param(&decl.params, known_guards);
    let has_guard_param = guard_param.is_some();
    let needs_input_struct = guard_param.is_none() && !decl.params.is_empty();
    let guard_needs_sanitize = guard_param
        .as_ref()
        .is_some_and(|(_, name)| guards_with_transforms.contains(name));

    // ── Imports ────────────────────────────────────────────────
    e.emit_use("axum::extract::Json");
    e.emit_use("axum::http::StatusCode");
    if has_guard_param {
        let (_, guard_name) = guard_param.as_ref().unwrap();
        let guard_mod = to_module_name(guard_name);
        e.emit_use(&format!("crate::guards::{guard_mod}::{guard_name}"));
        e.emit_use("crate::shared::validation::Validate");
        if guard_needs_sanitize {
            e.emit_use("crate::shared::validation::Sanitize");
        }
    }

    // Shared type imports (enums, type aliases referenced in param annotations)
    if !known_shared_types.is_empty() {
        let annotations: Vec<&TypeAnnotation> = decl
            .params
            .iter()
            .filter_map(|p| p.ty_ann.as_ref())
            .collect();
        let ann_refs: Vec<&TypeAnnotation> = annotations.to_vec();
        for name in collect_shared_type_refs(&ann_refs, known_shared_types) {
            let mod_name = to_module_name(&name);
            e.emit_use(&format!("crate::shared::{mod_name}::{name}"));
        }
    }
    e.newline();

    // ── fetch() wrapper (when action body uses fetch) ─────────
    let uses_fetch = block_uses_fetch(&decl.body);
    if uses_fetch {
        emit_fetch_wrapper(e);
        e.newline();
    }

    // ── Input struct (for non-guard params) ────────────────────
    if needs_input_struct {
        emit_input_struct(e, decl);
        e.newline();
    }

    // ── Handler function ───────────────────────────────────────
    emit_handler_fn(e, decl, known_guards, guard_needs_sanitize);
}

/// Emit the input struct for actions with plain (non-guard) params.
///
/// ```text
/// #[derive(Debug, serde::Deserialize)]
/// pub struct CreateUserInput {
///     pub name: String,
///     pub age: i64,
/// }
/// ```
fn emit_input_struct(e: &mut RustEmitter, decl: &ActionDecl) {
    let struct_name = format!("{}Input", pascal_case(&decl.name));
    e.emit_attribute("derive(Debug, serde::Deserialize)");
    e.block(&format!("pub struct {struct_name}"), |e| {
        for p in &decl.params {
            let field_name = to_snake_case(&p.name);
            let ty = if let Some(ann) = &p.ty_ann {
                annotation_to_rust(ann)
            } else {
                "serde_json::Value".into()
            };
            e.writeln(&format!("pub {field_name}: {ty},"));
        }
    });
}

/// Emit the async handler function.
fn emit_handler_fn(
    e: &mut RustEmitter,
    decl: &ActionDecl,
    known_guards: &HashSet<String>,
    guard_needs_sanitize: bool,
) {
    let guard_param = find_guard_param(&decl.params, known_guards);
    let has_guard_param = guard_param.is_some();
    let needs_input_struct = guard_param.is_none() && !decl.params.is_empty();

    // Determine the Json<T> extractor type
    let input_type = if let Some((_, guard_name)) = &guard_param {
        guard_name.to_string()
    } else if needs_input_struct {
        format!("{}Input", pascal_case(&decl.name))
    } else {
        "serde_json::Value".into()
    };

    // Doc comment
    e.emit_doc_comment(&format!("POST /api/__gx/actions/{}", decl.name));

    // Function signature
    let has_params = !decl.params.is_empty();
    let input_binding = if guard_needs_sanitize {
        "mut input"
    } else {
        "input"
    };
    let sig = if has_params {
        format!(
            "pub async fn handler(\n    Json({input_binding}): Json<{input_type}>,\n) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)>"
        )
    } else {
        "pub async fn handler() -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)>".into()
    };

    e.block(&sig, |e| {
        // Sanitize transforms (trim, precision, defaults) before validation
        if guard_needs_sanitize {
            e.writeln("input.sanitize();");
        }

        // Guard validation
        if has_guard_param {
            e.block("if let Err(errors) = input.validate()", |e| {
                e.writeln("return Err((StatusCode::BAD_REQUEST, Json(serde_json::json!({");
                e.indent();
                e.writeln("\"error\": \"validation_failed\",");
                e.writeln("\"details\": errors,");
                e.dedent();
                e.writeln("}))));");
            });
            e.newline();
        }

        // Bind params to local variables
        if has_params {
            if decl.params.len() == 1 {
                let p = &decl.params[0];
                let pname = to_snake_case(&p.name);
                e.writeln(&format!("let {pname} = input;"));
            } else if has_guard_param {
                // Single guard param — already bound above
                let p = &decl.params[0];
                let pname = to_snake_case(&p.name);
                e.writeln(&format!("let {pname} = input;"));
            } else {
                // Multiple plain params — destructure from input struct
                for p in &decl.params {
                    let pname = to_snake_case(&p.name);
                    e.writeln(&format!("let {pname} = input.{pname};"));
                }
            }
            e.newline();
        }

        // Action body — emit statements with return wrapping.
        // `return expr;` → `return Ok(Json(serde_json::json!(expr)));`
        e.emit_comment("--- Action body ---");
        let has_explicit_return = emit_action_body(e, &decl.body);

        // Default return (only if body doesn't explicitly return)
        if !has_explicit_return {
            e.newline();
            e.writeln("Ok(Json(serde_json::json!(null)))");
        }
    });
}

/// Emit the action body, wrapping `return expr;` in `Ok(Json(json!(...)))`.
///
/// Action handlers return `Result<Json<Value>, ...>`, so bare `return val;`
/// must become `return Ok(Json(serde_json::json!(val)));`.
/// Returns `true` if the body ends with an explicit `return` statement.
fn emit_action_body(e: &mut RustEmitter, block: &Block) -> bool {
    use crate::codegen::emit_expr::emit_expr;
    use crate::codegen::emit_stmt::emit_stmt;

    let mut has_return = false;
    for stmt in &block.stmts {
        if let Stmt::Return {
            value: Some(expr), ..
        } = stmt
        {
            // Wrap the return value for the Axum handler
            e.write("return Ok(Json(serde_json::json!(");
            emit_expr(e, expr);
            e.writeln(")));");
            has_return = true;
        } else {
            emit_stmt(e, stmt);
        }
    }
    has_return
}

/// Find the first param whose type annotation references a known guard.
///
/// Returns `(param_index, guard_name)`.
fn find_guard_param(params: &[Param], known_guards: &HashSet<String>) -> Option<(usize, String)> {
    for (i, p) in params.iter().enumerate() {
        if let Some(TypeAnnotation::Named { name, .. }) = &p.ty_ann {
            if known_guards.contains(name.as_str()) {
                return Some((i, name.to_string()));
            }
        }
    }
    None
}

/// Convert a camelCase or snake_case name to PascalCase.
///
/// `createUser` → `CreateUser`, `delete_item` → `DeleteItem`.
fn pascal_case(name: &str) -> String {
    let mut result = String::with_capacity(name.len());
    let mut next_upper = true;
    for ch in name.chars() {
        if ch == '_' {
            next_upper = true;
        } else if next_upper {
            result.push(ch.to_uppercase().next().unwrap_or(ch));
            next_upper = false;
        } else {
            result.push(ch);
        }
    }
    result
}

// ── fetch() detection and wrapper ──────────────────────────────────────

/// Emit a local `fetch` async helper that wraps `gale_lib::http::get()`.
///
/// Placed at file scope (before the handler fn) so the action body can
/// simply call `fetch(url).await`.  Returns the response body as a
/// `String`, or an empty string on error.
fn emit_fetch_wrapper(e: &mut RustEmitter) {
    e.writeln("/// HTTP fetch helper — wraps `gale_lib::http::get()`.");
    e.writeln("/// Accepts `&str`, `String`, or any `AsRef<str>` type.");
    e.writeln("#[allow(dead_code)]");
    e.block("async fn fetch(url: impl AsRef<str>) -> String", |e| {
        e.writeln("gale_lib::http::get(url.as_ref()).await.unwrap_or_default()");
    });
}

/// Check whether a block (action body) contains any call to `fetch()`.
pub fn block_uses_fetch(block: &Block) -> bool {
    block.stmts.iter().any(|s| stmt_uses_fetch(s))
}

fn stmt_uses_fetch(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Let { init, .. } | Stmt::Mut { init, .. } | Stmt::Frozen { init, .. } => {
            expr_uses_fetch(init)
        }
        Stmt::Return { value, .. } => value.as_ref().is_some_and(|e| expr_uses_fetch(e)),
        Stmt::ExprStmt { expr, .. } => expr_uses_fetch(expr),
        Stmt::If {
            condition,
            then_block,
            else_branch,
            ..
        } => {
            expr_uses_fetch(condition)
                || block_uses_fetch(then_block)
                || match else_branch {
                    Some(ElseBranch::Else(b)) => block_uses_fetch(b),
                    Some(ElseBranch::ElseIf(s)) => stmt_uses_fetch(s),
                    None => false,
                }
        }
        Stmt::For { iterable, body, .. } => expr_uses_fetch(iterable) || block_uses_fetch(body),
        Stmt::Block(b) => block_uses_fetch(b),
        Stmt::FnDecl(decl) => block_uses_fetch(&decl.body),
        _ => false,
    }
}

fn expr_uses_fetch(expr: &Expr) -> bool {
    match expr {
        Expr::FnCall { callee, args, .. } => {
            // Direct `fetch(url)` call
            if matches!(callee.as_ref(), Expr::Ident { name, .. } if name == "fetch") {
                return true;
            }
            expr_uses_fetch(callee) || args.iter().any(|a| expr_uses_fetch(a))
        }
        Expr::Await { expr: inner, .. } => expr_uses_fetch(inner),
        Expr::BinaryOp { left, right, .. } => expr_uses_fetch(left) || expr_uses_fetch(right),
        Expr::UnaryOp { operand, .. } => expr_uses_fetch(operand),
        Expr::MemberAccess { object, .. } => expr_uses_fetch(object),
        Expr::Ternary {
            condition,
            then_expr,
            else_expr,
            ..
        } => expr_uses_fetch(condition) || expr_uses_fetch(then_expr) || expr_uses_fetch(else_expr),
        Expr::TemplateLit { parts, .. } => parts.iter().any(|p| {
            if let TemplatePart::Expr(e) = p {
                expr_uses_fetch(e)
            } else {
                false
            }
        }),
        _ => false,
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::span::Span;

    fn s() -> Span {
        Span::dummy()
    }

    fn guards(names: &[&str]) -> HashSet<String> {
        names.iter().map(|n| n.to_string()).collect()
    }

    fn emit(decl: &ActionDecl, known: &HashSet<String>) -> String {
        let mut e = RustEmitter::new();
        let no_transforms = HashSet::new();
        let no_shared = HashSet::new();
        emit_action_file(&mut e, decl, known, &no_transforms, &no_shared);
        e.finish()
    }

    #[test]
    fn action_with_guard_param() {
        let out = emit(
            &ActionDecl {
                name: "createUser".into(),
                params: vec![Param {
                    name: "data".into(),
                    ty_ann: Some(TypeAnnotation::Named {
                        name: "UserForm".into(),
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
                span: s(),
            },
            &guards(&["UserForm"]),
        );
        assert!(out.contains("use crate::guards::user_form::UserForm;"));
        assert!(out.contains("Json(input): Json<UserForm>"));
        assert!(out.contains("input.validate()"));
        assert!(out.contains("validation_failed"));
    }

    #[test]
    fn action_with_plain_params() {
        let out = emit(
            &ActionDecl {
                name: "addItem".into(),
                params: vec![
                    Param {
                        name: "name".into(),
                        ty_ann: Some(TypeAnnotation::Named {
                            name: "string".into(),
                            span: s(),
                        }),
                        default: None,
                        span: s(),
                    },
                    Param {
                        name: "count".into(),
                        ty_ann: Some(TypeAnnotation::Named {
                            name: "int".into(),
                            span: s(),
                        }),
                        default: None,
                        span: s(),
                    },
                ],
                ret_ty: None,
                body: Block {
                    stmts: vec![],
                    span: s(),
                },
                span: s(),
            },
            &guards(&[]),
        );
        assert!(out.contains("pub struct AddItemInput {"));
        assert!(out.contains("pub name: String,"));
        assert!(out.contains("pub count: i64,"));
        assert!(out.contains("Json(input): Json<AddItemInput>"));
        assert!(!out.contains("validate()")); // No guard validation
    }

    #[test]
    fn action_empty_body() {
        let out = emit(
            &ActionDecl {
                name: "noop".into(),
                params: vec![],
                ret_ty: None,
                body: Block {
                    stmts: vec![],
                    span: s(),
                },
                span: s(),
            },
            &guards(&[]),
        );
        assert!(out.contains("pub async fn handler()"));
        assert!(out.contains("Ok(Json(serde_json::json!(null)))"));
    }

    #[test]
    fn action_with_body_statements() {
        let out = emit(
            &ActionDecl {
                name: "compute".into(),
                params: vec![Param {
                    name: "x".into(),
                    ty_ann: Some(TypeAnnotation::Named {
                        name: "int".into(),
                        span: s(),
                    }),
                    default: None,
                    span: s(),
                }],
                ret_ty: None,
                body: Block {
                    stmts: vec![
                        Stmt::Let {
                            name: "result".into(),
                            ty_ann: None,
                            init: Expr::BinaryOp {
                                left: Box::new(Expr::Ident {
                                    name: "x".into(),
                                    span: s(),
                                }),
                                op: BinOp::Mul,
                                right: Box::new(Expr::IntLit {
                                    value: 2,
                                    span: s(),
                                }),
                                span: s(),
                            },
                            span: s(),
                        },
                        Stmt::Return {
                            value: Some(Expr::ObjectLit {
                                fields: vec![ObjectFieldExpr {
                                    key: "answer".into(),
                                    value: Expr::Ident {
                                        name: "result".into(),
                                        span: s(),
                                    },
                                    span: s(),
                                }],
                                span: s(),
                            }),
                            span: s(),
                        },
                    ],
                    span: s(),
                },
                span: s(),
            },
            &guards(&[]),
        );
        assert!(out.contains("let result = (x * 2_i64);"));
        assert!(out.contains("return serde_json::json!({\"answer\": result});"));
    }

    #[test]
    fn action_has_route_doc_comment() {
        let out = emit(
            &ActionDecl {
                name: "login".into(),
                params: vec![],
                ret_ty: None,
                body: Block {
                    stmts: vec![],
                    span: s(),
                },
                span: s(),
            },
            &guards(&[]),
        );
        assert!(out.contains("/// POST /api/__gx/actions/login"));
    }

    #[test]
    fn pascal_case_conversion() {
        assert_eq!(pascal_case("createUser"), "CreateUser");
        assert_eq!(pascal_case("delete_item"), "DeleteItem");
        assert_eq!(pascal_case("simple"), "Simple");
        assert_eq!(pascal_case("a"), "A");
    }

    #[test]
    fn action_with_shared_type_param() {
        let shared: HashSet<String> = ["Status"].iter().map(|s| s.to_string()).collect();
        let mut e = RustEmitter::new();
        emit_action_file(
            &mut e,
            &ActionDecl {
                name: "setStatus".into(),
                params: vec![Param {
                    name: "status".into(),
                    ty_ann: Some(TypeAnnotation::Named {
                        name: "Status".into(),
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
                span: s(),
            },
            &guards(&[]),
            &HashSet::new(),
            &shared,
        );
        let out = e.finish();
        assert!(out.contains("use crate::shared::status::Status;"));
    }
}
