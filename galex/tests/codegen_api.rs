//! Integration tests for API route generation (Phase 11.4).
//!
//! Tests construct AST nodes directly and verify the generated Rust code
//! contains expected patterns for REST API handlers.

use std::collections::HashSet;

use galex::ast::*;
use galex::codegen::emit_api;
use galex::codegen::rust_emitter::RustEmitter;
use galex::codegen::CodegenContext;
use galex::span::Span;
use galex::types::ty::TypeInterner;

fn s() -> Span {
    Span::dummy()
}

fn guards(names: &[&str]) -> HashSet<String> {
    names.iter().map(|n| n.to_string()).collect()
}

fn emit(decl: &ApiDecl, known: &HashSet<String>) -> String {
    let mut e = RustEmitter::new();
    let no_shared = HashSet::new();
    emit_api::emit_api_file(&mut e, decl, known, &no_shared);
    e.finish()
}

fn make_program(items: Vec<Item>) -> Program {
    Program { items, span: s() }
}

fn make_handler(method: HttpMethod, params: Vec<Param>, body: Vec<Stmt>) -> ApiHandler {
    ApiHandler {
        method,
        path_params: vec![],
        params,
        ret_ty: None,
        body: Block {
            stmts: body,
            span: s(),
        },
        span: s(),
    }
}

fn make_handler_with_path(
    method: HttpMethod,
    path_params: Vec<&str>,
    params: Vec<Param>,
) -> ApiHandler {
    ApiHandler {
        method,
        path_params: path_params.into_iter().map(|p| p.into()).collect(),
        params,
        ret_ty: None,
        body: Block {
            stmts: vec![],
            span: s(),
        },
        span: s(),
    }
}

// ── Path convention ────────────────────────────────────────────────────

#[test]
fn api_resource_path_simple() {
    let groups =
        emit_api::api_route_groups("Users", &[make_handler(HttpMethod::Get, vec![], vec![])]);
    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].0, "/api/users");
}

#[test]
fn api_resource_path_multi_word() {
    let groups = emit_api::api_route_groups(
        "BlogPosts",
        &[make_handler(HttpMethod::Get, vec![], vec![])],
    );
    assert_eq!(groups[0].0, "/api/blog-posts");
}

#[test]
fn api_path_with_param() {
    let groups = emit_api::api_route_groups(
        "Users",
        &[make_handler_with_path(HttpMethod::Get, vec!["id"], vec![])],
    );
    assert_eq!(groups[0].0, "/api/users/:id");
}

#[test]
fn api_multiple_path_params() {
    let groups = emit_api::api_route_groups(
        "Posts",
        &[make_handler_with_path(
            HttpMethod::Get,
            vec!["year", "month"],
            vec![],
        )],
    );
    assert_eq!(groups[0].0, "/api/posts/:year/:month");
}

// ── Handler generation ─────────────────────────────────────────────────

#[test]
fn api_get_handler_generated() {
    let out = emit(
        &ApiDecl {
            name: "Users".into(),
            handlers: vec![make_handler(HttpMethod::Get, vec![], vec![])],
            span: s(),
        },
        &guards(&[]),
    );
    assert!(
        out.contains("pub async fn list"),
        "GET collection should generate 'list': {out}"
    );
    assert!(
        out.contains("/// GET /api/users"),
        "should have doc comment: {out}"
    );
}

#[test]
fn api_post_handler_generated() {
    let out = emit(
        &ApiDecl {
            name: "Users".into(),
            handlers: vec![make_handler(
                HttpMethod::Post,
                vec![Param {
                    name: "body".into(),
                    ty_ann: Some(TypeAnnotation::Named {
                        name: "string".into(),
                        span: s(),
                    }),
                    default: None,
                    span: s(),
                }],
                vec![],
            )],
            span: s(),
        },
        &guards(&[]),
    );
    assert!(
        out.contains("pub async fn create"),
        "POST should generate 'create': {out}"
    );
    assert!(
        out.contains("/// POST /api/users"),
        "should have doc comment: {out}"
    );
}

#[test]
fn api_delete_handler_generated() {
    let out = emit(
        &ApiDecl {
            name: "Users".into(),
            handlers: vec![make_handler_with_path(
                HttpMethod::Delete,
                vec!["id"],
                vec![],
            )],
            span: s(),
        },
        &guards(&[]),
    );
    assert!(
        out.contains("pub async fn delete_by_id"),
        "DELETE[id] should generate 'delete_by_id': {out}"
    );
    assert!(
        out.contains("StatusCode::NO_CONTENT"),
        "DELETE should return 204: {out}"
    );
}

#[test]
fn api_put_handler_generated() {
    let out = emit(
        &ApiDecl {
            name: "Users".into(),
            handlers: vec![ApiHandler {
                method: HttpMethod::Put,
                path_params: vec!["id".into()],
                params: vec![Param {
                    name: "body".into(),
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
                span: s(),
            }],
            span: s(),
        },
        &guards(&[]),
    );
    assert!(
        out.contains("pub async fn update_by_id"),
        "PUT[id] should generate 'update_by_id': {out}"
    );
    assert!(out.contains("Path(id)"), "should extract path param: {out}");
    assert!(
        out.contains("Json(input)"),
        "should extract JSON body: {out}"
    );
}

#[test]
fn api_patch_handler_generated() {
    let out = emit(
        &ApiDecl {
            name: "Users".into(),
            handlers: vec![ApiHandler {
                method: HttpMethod::Patch,
                path_params: vec!["id".into()],
                params: vec![Param {
                    name: "data".into(),
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
                span: s(),
            }],
            span: s(),
        },
        &guards(&[]),
    );
    assert!(
        out.contains("pub async fn patch_by_id"),
        "PATCH[id] should generate 'patch_by_id': {out}"
    );
}

// ── Guard-based validation ─────────────────────────────────────────────

#[test]
fn api_get_with_guard_query() {
    let out = emit(
        &ApiDecl {
            name: "Users".into(),
            handlers: vec![make_handler(
                HttpMethod::Get,
                vec![Param {
                    name: "query".into(),
                    ty_ann: Some(TypeAnnotation::Named {
                        name: "Pagination".into(),
                        span: s(),
                    }),
                    default: None,
                    span: s(),
                }],
                vec![],
            )],
            span: s(),
        },
        &guards(&["Pagination"]),
    );
    assert!(
        out.contains("Query(input): Query<Pagination>"),
        "GET with guard should use Query extractor: {out}"
    );
    assert!(
        out.contains("input.validate()"),
        "should validate guard: {out}"
    );
}

#[test]
fn api_post_with_guard_body() {
    let out = emit(
        &ApiDecl {
            name: "Users".into(),
            handlers: vec![make_handler(
                HttpMethod::Post,
                vec![Param {
                    name: "body".into(),
                    ty_ann: Some(TypeAnnotation::Named {
                        name: "CreateUser".into(),
                        span: s(),
                    }),
                    default: None,
                    span: s(),
                }],
                vec![],
            )],
            span: s(),
        },
        &guards(&["CreateUser"]),
    );
    assert!(
        out.contains("Json(input): Json<CreateUser>"),
        "POST with guard should use Json extractor: {out}"
    );
    assert!(
        out.contains("input.validate()"),
        "should validate guard: {out}"
    );
}

// ── Status codes ───────────────────────────────────────────────────────

#[test]
fn api_post_returns_201() {
    let out = emit(
        &ApiDecl {
            name: "Items".into(),
            handlers: vec![make_handler(HttpMethod::Post, vec![], vec![])],
            span: s(),
        },
        &guards(&[]),
    );
    assert!(
        out.contains("StatusCode::CREATED"),
        "POST should return 201 CREATED: {out}"
    );
}

#[test]
fn api_get_returns_200() {
    let out = emit(
        &ApiDecl {
            name: "Items".into(),
            handlers: vec![make_handler(HttpMethod::Get, vec![], vec![])],
            span: s(),
        },
        &guards(&[]),
    );
    // GET returns Ok(Json(...)) which is 200 by default — no explicit StatusCode
    assert!(
        out.contains("Ok(Json("),
        "GET should return Ok(Json(...)): {out}"
    );
    assert!(
        !out.contains("StatusCode::CREATED"),
        "GET should NOT return 201: {out}"
    );
}

#[test]
fn api_delete_returns_204() {
    let out = emit(
        &ApiDecl {
            name: "Items".into(),
            handlers: vec![make_handler_with_path(
                HttpMethod::Delete,
                vec!["id"],
                vec![],
            )],
            span: s(),
        },
        &guards(&[]),
    );
    assert!(
        out.contains("StatusCode::NO_CONTENT"),
        "DELETE should return 204 NO_CONTENT: {out}"
    );
}

#[test]
fn api_validation_returns_422() {
    let out = emit(
        &ApiDecl {
            name: "Users".into(),
            handlers: vec![make_handler(
                HttpMethod::Post,
                vec![Param {
                    name: "body".into(),
                    ty_ann: Some(TypeAnnotation::Named {
                        name: "UserForm".into(),
                        span: s(),
                    }),
                    default: None,
                    span: s(),
                }],
                vec![],
            )],
            span: s(),
        },
        &guards(&["UserForm"]),
    );
    assert!(
        out.contains("UNPROCESSABLE_ENTITY"),
        "validation failure should return 422: {out}"
    );
}

// ── Route registration ─────────────────────────────────────────────────

#[test]
fn api_route_registration() {
    let interner = TypeInterner::new();
    let program = make_program(vec![Item::ApiDecl(ApiDecl {
        name: "Users".into(),
        handlers: vec![make_handler(HttpMethod::Get, vec![], vec![])],
        span: s(),
    })]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    let main = ctx.files.get("src/main.rs").unwrap();
    assert!(
        main.contains("/api/users"),
        "main.rs should contain API route: {main}"
    );
}

#[test]
fn api_method_chaining() {
    let interner = TypeInterner::new();
    let program = make_program(vec![Item::ApiDecl(ApiDecl {
        name: "Users".into(),
        handlers: vec![
            make_handler(HttpMethod::Get, vec![], vec![]),
            make_handler(HttpMethod::Post, vec![], vec![]),
        ],
        span: s(),
    })]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    let main = ctx.files.get("src/main.rs").unwrap();
    // Same path should chain methods: .get(...).post(...)
    assert!(
        main.contains(".post("),
        "should chain methods on same path: {main}"
    );
}

// ── Module structure ───────────────────────────────────────────────────

#[test]
fn api_module_structure() {
    let interner = TypeInterner::new();
    let program = make_program(vec![Item::ApiDecl(ApiDecl {
        name: "Users".into(),
        handlers: vec![make_handler(HttpMethod::Get, vec![], vec![])],
        span: s(),
    })]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    assert!(
        ctx.files.contains("src/api/users.rs"),
        "should create src/api/users.rs"
    );
    assert!(
        ctx.files.contains("src/api/mod.rs"),
        "should create src/api/mod.rs"
    );
}

#[test]
fn api_main_declares_mod() {
    let interner = TypeInterner::new();
    let program = make_program(vec![Item::ApiDecl(ApiDecl {
        name: "Users".into(),
        handlers: vec![make_handler(HttpMethod::Get, vec![], vec![])],
        span: s(),
    })]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    let main = ctx.files.get("src/main.rs").unwrap();
    assert!(
        main.contains("mod api;"),
        "main.rs should declare mod api: {main}"
    );
}

// ── Handler body ───────────────────────────────────────────────────────

#[test]
fn api_handler_body_emitted() {
    let out = emit(
        &ApiDecl {
            name: "Items".into(),
            handlers: vec![make_handler(
                HttpMethod::Get,
                vec![],
                vec![Stmt::Let {
                    name: "result".into(),
                    ty_ann: None,
                    init: Expr::IntLit {
                        value: 42,
                        span: s(),
                    },
                    span: s(),
                }],
            )],
            span: s(),
        },
        &guards(&[]),
    );
    assert!(
        out.contains("let result"),
        "handler body should contain statements: {out}"
    );
}

// ── Extractors ─────────────────────────────────────────────────────────

#[test]
fn api_get_no_params_no_extractor() {
    let out = emit(
        &ApiDecl {
            name: "Items".into(),
            handlers: vec![make_handler(HttpMethod::Get, vec![], vec![])],
            span: s(),
        },
        &guards(&[]),
    );
    assert!(
        !out.contains("Query("),
        "GET with no params should have no Query extractor: {out}"
    );
    assert!(
        !out.contains("Json(input)"),
        "GET with no params should have no Json extractor: {out}"
    );
}

#[test]
fn api_path_param_extractor() {
    let out = emit(
        &ApiDecl {
            name: "Users".into(),
            handlers: vec![make_handler_with_path(HttpMethod::Get, vec!["id"], vec![])],
            span: s(),
        },
        &guards(&[]),
    );
    assert!(
        out.contains("Path(id): Path<String>"),
        "should extract single path param: {out}"
    );
}

#[test]
fn api_multiple_path_params_extractor() {
    let out = emit(
        &ApiDecl {
            name: "Posts".into(),
            handlers: vec![make_handler_with_path(
                HttpMethod::Get,
                vec!["year", "month"],
                vec![],
            )],
            span: s(),
        },
        &guards(&[]),
    );
    assert!(
        out.contains("Path((year, month))"),
        "should extract multiple path params as tuple: {out}"
    );
}

#[test]
fn api_input_struct_for_plain_params() {
    let out = emit(
        &ApiDecl {
            name: "Items".into(),
            handlers: vec![make_handler(
                HttpMethod::Post,
                vec![
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
                vec![],
            )],
            span: s(),
        },
        &guards(&[]),
    );
    assert!(
        out.contains("pub struct ItemsCreateInput"),
        "should generate input struct: {out}"
    );
    assert!(
        out.contains("pub name: String,"),
        "input struct should have name field: {out}"
    );
    assert!(
        out.contains("pub count: i64,"),
        "input struct should have count field: {out}"
    );
}

// ── Imports ────────────────────────────────────────────────────────────

#[test]
fn api_imports_correct() {
    let out = emit(
        &ApiDecl {
            name: "Users".into(),
            handlers: vec![
                make_handler(HttpMethod::Get, vec![], vec![]),
                make_handler_with_path(HttpMethod::Delete, vec!["id"], vec![]),
            ],
            span: s(),
        },
        &guards(&[]),
    );
    assert!(
        out.contains("use axum::extract::Json;"),
        "should import Json: {out}"
    );
    assert!(
        out.contains("use axum::http::StatusCode;"),
        "should import StatusCode: {out}"
    );
    assert!(
        out.contains("use axum::extract::Path;"),
        "should import Path when path params used: {out}"
    );
}

#[test]
fn api_guard_import() {
    let out = emit(
        &ApiDecl {
            name: "Users".into(),
            handlers: vec![make_handler(
                HttpMethod::Post,
                vec![Param {
                    name: "body".into(),
                    ty_ann: Some(TypeAnnotation::Named {
                        name: "CreateUser".into(),
                        span: s(),
                    }),
                    default: None,
                    span: s(),
                }],
                vec![],
            )],
            span: s(),
        },
        &guards(&["CreateUser"]),
    );
    assert!(
        out.contains("use crate::guards::create_user::CreateUser;"),
        "should import guard: {out}"
    );
}

// ── Doc comments ───────────────────────────────────────────────────────

#[test]
fn api_doc_comments_include_method_and_path() {
    let out = emit(
        &ApiDecl {
            name: "BlogPosts".into(),
            handlers: vec![
                make_handler(HttpMethod::Get, vec![], vec![]),
                make_handler_with_path(HttpMethod::Delete, vec!["slug"], vec![]),
            ],
            span: s(),
        },
        &guards(&[]),
    );
    assert!(
        out.contains("/// GET /api/blog-posts"),
        "should have GET doc comment: {out}"
    );
    assert!(
        out.contains("/// DELETE /api/blog-posts/:slug"),
        "should have DELETE doc comment: {out}"
    );
}

// ── Route group merging ────────────────────────────────────────────────

#[test]
fn api_route_groups_merge_same_path() {
    let groups = emit_api::api_route_groups(
        "Users",
        &[
            make_handler(HttpMethod::Get, vec![], vec![]),
            make_handler(HttpMethod::Post, vec![], vec![]),
        ],
    );
    // Both GET and POST on /api/users should be in the same group
    assert_eq!(groups.len(), 1, "same path should be grouped: {groups:?}");
    assert_eq!(
        groups[0].1.len(),
        2,
        "should have 2 methods: {:?}",
        groups[0].1
    );
}

#[test]
fn api_route_groups_separate_paths() {
    let groups = emit_api::api_route_groups(
        "Users",
        &[
            make_handler(HttpMethod::Get, vec![], vec![]),
            make_handler_with_path(HttpMethod::Get, vec!["id"], vec![]),
        ],
    );
    // /api/users and /api/users/:id should be separate groups
    assert_eq!(
        groups.len(),
        2,
        "different paths should be separate: {groups:?}"
    );
}
