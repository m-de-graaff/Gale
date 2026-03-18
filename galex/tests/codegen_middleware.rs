//! Integration tests for middleware generation (Phase 11.6).
//!
//! Tests construct AST nodes directly and verify the generated Rust code
//! contains expected patterns for Axum middleware functions.

use galex::ast::*;
use galex::codegen::emit_middleware;
use galex::codegen::project;
use galex::codegen::rust_emitter::RustEmitter;
use galex::codegen::CodegenContext;
use galex::span::Span;
use galex::types::ty::TypeInterner;

fn s() -> Span {
    Span::dummy()
}

fn make_program(items: Vec<Item>) -> Program {
    Program { items, span: s() }
}

fn make_mw(name: &str, target: MiddlewareTarget, body: Vec<Stmt>) -> MiddlewareDecl {
    MiddlewareDecl {
        name: name.into(),
        target,
        params: vec![
            Param {
                name: "req".into(),
                ty_ann: Some(TypeAnnotation::Named {
                    name: "Request".into(),
                    span: s(),
                }),
                default: None,
                span: s(),
            },
            Param {
                name: "next".into(),
                ty_ann: Some(TypeAnnotation::Named {
                    name: "Next".into(),
                    span: s(),
                }),
                default: None,
                span: s(),
            },
        ],
        body: Block {
            stmts: body,
            span: s(),
        },
        span: s(),
    }
}

fn emit_mw(decl: &MiddlewareDecl) -> String {
    let mut e = RustEmitter::new();
    emit_middleware::emit_middleware_file(&mut e, decl);
    e.finish()
}

// ── File generation ────────────────────────────────────────────────────

#[test]
fn middleware_generates_file() {
    let interner = TypeInterner::new();
    let program = make_program(vec![Item::MiddlewareDecl(make_mw(
        "auth",
        MiddlewareTarget::Global,
        vec![],
    ))]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    assert!(
        ctx.files.contains("src/middleware/auth.rs"),
        "should create src/middleware/auth.rs"
    );
}

#[test]
fn middleware_module_structure() {
    let interner = TypeInterner::new();
    let program = make_program(vec![Item::MiddlewareDecl(make_mw(
        "auth",
        MiddlewareTarget::Global,
        vec![],
    ))]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    assert!(
        ctx.files.contains("src/middleware/mod.rs"),
        "should create src/middleware/mod.rs"
    );
    let modrs = ctx.files.get("src/middleware/mod.rs").unwrap();
    assert!(
        modrs.contains("pub mod auth;"),
        "mod.rs should declare auth module: {modrs}"
    );
}

#[test]
fn middleware_main_declares_mod() {
    let interner = TypeInterner::new();
    let program = make_program(vec![Item::MiddlewareDecl(make_mw(
        "auth",
        MiddlewareTarget::Global,
        vec![],
    ))]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    let main = ctx.files.get("src/main.rs").unwrap();
    assert!(
        main.contains("mod middleware;"),
        "main.rs should declare mod middleware: {main}"
    );
    assert!(
        main.contains("mod gale_middleware;"),
        "main.rs should declare mod gale_middleware: {main}"
    );
}

// ── Function signature ─────────────────────────────────────────────────

#[test]
fn middleware_fn_signature() {
    let out = emit_mw(&make_mw("auth", MiddlewareTarget::Global, vec![]));
    assert!(
        out.contains("pub async fn middleware_fn"),
        "should have async middleware_fn: {out}"
    );
    assert!(
        out.contains("request: Request<Body>"),
        "should accept Request<Body>: {out}"
    );
    assert!(out.contains("next: Next"), "should accept Next: {out}");
    assert!(out.contains("-> Response"), "should return Response: {out}");
}

#[test]
fn middleware_imports() {
    let out = emit_mw(&make_mw("auth", MiddlewareTarget::Global, vec![]));
    assert!(
        out.contains("use axum::body::Body;"),
        "should import Body: {out}"
    );
    assert!(
        out.contains("use axum::http::Request;"),
        "should import Request: {out}"
    );
    assert!(
        out.contains("use axum::middleware::Next;"),
        "should import Next: {out}"
    );
    assert!(
        out.contains("use axum::response::Response;"),
        "should import Response: {out}"
    );
    assert!(
        out.contains("GaleRequest"),
        "should import GaleRequest: {out}"
    );
    assert!(
        out.contains("GaleResponse"),
        "should import GaleResponse: {out}"
    );
}

// ── Body emission ──────────────────────────────────────────────────────

#[test]
fn middleware_body_emitted() {
    let out = emit_mw(&make_mw(
        "auth",
        MiddlewareTarget::Global,
        vec![Stmt::Let {
            name: "token".into(),
            ty_ann: None,
            init: Expr::StringLit {
                value: "test".into(),
                span: s(),
            },
            span: s(),
        }],
    ));
    assert!(
        out.contains("let token"),
        "body statements should be emitted: {out}"
    );
}

#[test]
fn middleware_gale_request_wrapper() {
    let out = emit_mw(&make_mw("auth", MiddlewareTarget::Global, vec![]));
    assert!(
        out.contains("let mut req = GaleRequest(request)"),
        "should wrap request in GaleRequest: {out}"
    );
}

// ── Expression rewriting ───────────────────────────────────────────────

#[test]
fn middleware_next_call_rewrite() {
    // let res = next(req)  →  let res = next.run(req.into_inner()).await
    let out = emit_mw(&make_mw(
        "auth",
        MiddlewareTarget::Global,
        vec![Stmt::Let {
            name: "res".into(),
            ty_ann: None,
            init: Expr::FnCall {
                callee: Box::new(Expr::Ident {
                    name: "next".into(),
                    span: s(),
                }),
                args: vec![Expr::Ident {
                    name: "req".into(),
                    span: s(),
                }],
                span: s(),
            },
            span: s(),
        }],
    ));
    assert!(
        out.contains("next.run(req.into_inner()).await"),
        "next(req) should be rewritten: {out}"
    );
}

#[test]
fn middleware_response_status_rewrite() {
    // return Response.status(401)  →  return GaleResponse::status(401)
    let out = emit_mw(&make_mw(
        "auth",
        MiddlewareTarget::Global,
        vec![Stmt::Return {
            value: Some(Expr::FnCall {
                callee: Box::new(Expr::MemberAccess {
                    object: Box::new(Expr::Ident {
                        name: "Response".into(),
                        span: s(),
                    }),
                    field: "status".into(),
                    span: s(),
                }),
                args: vec![Expr::IntLit {
                    value: 401,
                    span: s(),
                }],
                span: s(),
            }),
            span: s(),
        }],
    ));
    assert!(
        out.contains("GaleResponse::status("),
        "Response.status() should be rewritten: {out}"
    );
    assert!(out.contains("401"), "should include status code: {out}");
}

#[test]
fn middleware_response_json_rewrite() {
    // return Response.json(data)  →  return GaleResponse::json(data)
    let out = emit_mw(&make_mw(
        "logger",
        MiddlewareTarget::Global,
        vec![Stmt::Return {
            value: Some(Expr::FnCall {
                callee: Box::new(Expr::MemberAccess {
                    object: Box::new(Expr::Ident {
                        name: "Response".into(),
                        span: s(),
                    }),
                    field: "json".into(),
                    span: s(),
                }),
                args: vec![Expr::Ident {
                    name: "data".into(),
                    span: s(),
                }],
                span: s(),
            }),
            span: s(),
        }],
    ));
    assert!(
        out.contains("GaleResponse::json("),
        "Response.json() should be rewritten: {out}"
    );
}

// ── Router wiring ──────────────────────────────────────────────────────

#[test]
fn middleware_global_layer() {
    let interner = TypeInterner::new();
    let program = make_program(vec![Item::MiddlewareDecl(make_mw(
        "auth",
        MiddlewareTarget::Global,
        vec![],
    ))]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    let main = ctx.files.get("src/main.rs").unwrap();
    assert!(
        main.contains("from_fn(middleware::auth::middleware_fn)"),
        "global middleware should appear as .layer(from_fn(...)): {main}"
    );
}

#[test]
fn middleware_segment_nests_routes() {
    let interner = TypeInterner::new();
    let program = make_program(vec![
        Item::ApiDecl(ApiDecl {
            name: "Users".into(),
            handlers: vec![ApiHandler {
                method: HttpMethod::Get,
                path_params: vec![],
                params: vec![],
                ret_ty: None,
                body: Block {
                    stmts: vec![],
                    span: s(),
                },
                span: s(),
            }],
            span: s(),
        }),
        Item::MiddlewareDecl(make_mw(
            "apiAuth",
            MiddlewareTarget::PathPrefix("/api".into()),
            vec![],
        )),
    ]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    let main = ctx.files.get("src/main.rs").unwrap();
    assert!(
        main.contains(".nest(\"/api\""),
        "segment middleware should create nested router: {main}"
    );
    assert!(
        main.contains("from_fn(middleware::api_auth::middleware_fn)"),
        "segment middleware should be layered on sub-router: {main}"
    );
}

#[test]
fn middleware_leaf_on_resource() {
    let interner = TypeInterner::new();
    let program = make_program(vec![Item::MiddlewareDecl(make_mw(
        "rateLimit",
        MiddlewareTarget::Resource("Users".into()),
        vec![],
    ))]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    let main = ctx.files.get("src/main.rs").unwrap();
    assert!(
        main.contains("from_fn(middleware::rate_limit::middleware_fn)"),
        "resource middleware should appear: {main}"
    );
}

// ── Runtime generation ─────────────────────────────────────────────────

#[test]
fn middleware_runtime_generated() {
    let interner = TypeInterner::new();
    let program = make_program(vec![Item::MiddlewareDecl(make_mw(
        "auth",
        MiddlewareTarget::Global,
        vec![],
    ))]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    assert!(
        ctx.files.contains("src/gale_middleware.rs"),
        "should generate gale_middleware runtime"
    );
}

#[test]
fn middleware_runtime_has_gale_request() {
    let runtime = project::generate_gale_middleware_runtime();
    assert!(
        runtime.contains("pub struct GaleRequest"),
        "runtime should have GaleRequest: {runtime}"
    );
}

#[test]
fn middleware_runtime_has_gale_response() {
    let runtime = project::generate_gale_middleware_runtime();
    assert!(
        runtime.contains("pub struct GaleResponse"),
        "runtime should have GaleResponse: {runtime}"
    );
}

#[test]
fn middleware_runtime_header_method() {
    let runtime = project::generate_gale_middleware_runtime();
    assert!(
        runtime.contains("fn header("),
        "GaleRequest should have header() method: {runtime}"
    );
}

#[test]
fn middleware_runtime_status_method() {
    let runtime = project::generate_gale_middleware_runtime();
    assert!(
        runtime.contains("fn status("),
        "GaleResponse should have status() method: {runtime}"
    );
}

#[test]
fn middleware_runtime_path_method() {
    let runtime = project::generate_gale_middleware_runtime();
    assert!(
        runtime.contains("fn path("),
        "GaleRequest should have path() method: {runtime}"
    );
}

#[test]
fn middleware_runtime_method_method() {
    let runtime = project::generate_gale_middleware_runtime();
    assert!(
        runtime.contains("fn method("),
        "GaleRequest should have method() method: {runtime}"
    );
}

#[test]
fn middleware_runtime_set_header_method() {
    let runtime = project::generate_gale_middleware_runtime();
    assert!(
        runtime.contains("fn set_header("),
        "GaleRequest should have set_header() method: {runtime}"
    );
}

#[test]
fn middleware_runtime_into_inner_method() {
    let runtime = project::generate_gale_middleware_runtime();
    assert!(
        runtime.contains("fn into_inner("),
        "GaleRequest should have into_inner() method: {runtime}"
    );
}

#[test]
fn middleware_runtime_json_method() {
    let runtime = project::generate_gale_middleware_runtime();
    assert!(
        runtime.contains("fn json("),
        "GaleResponse should have json() method: {runtime}"
    );
}

#[test]
fn middleware_runtime_redirect_method() {
    let runtime = project::generate_gale_middleware_runtime();
    assert!(
        runtime.contains("fn redirect("),
        "GaleResponse should have redirect() method: {runtime}"
    );
}

// ── Middleware ordering and multiples ───────────────────────────────────

#[test]
fn middleware_order_global_last() {
    let interner = TypeInterner::new();
    let program = make_program(vec![
        Item::ComponentDecl(ComponentDecl {
            name: "HomePage".into(),
            props: vec![],
            body: ComponentBody {
                stmts: vec![],
                template: vec![],
                head: None,
                span: s(),
            },
            span: s(),
        }),
        Item::MiddlewareDecl(make_mw("auth", MiddlewareTarget::Global, vec![])),
    ]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    let main = ctx.files.get("src/main.rs").unwrap();
    // The .layer() for global middleware should appear after .route()
    let route_pos = main.find(".route(").unwrap_or(0);
    let layer_pos = main.find("from_fn(middleware::auth").unwrap_or(0);
    assert!(
        layer_pos > route_pos,
        "global middleware should be layered after routes: {main}"
    );
}

#[test]
fn middleware_multiple_global() {
    let interner = TypeInterner::new();
    let program = make_program(vec![
        Item::MiddlewareDecl(make_mw("auth", MiddlewareTarget::Global, vec![])),
        Item::MiddlewareDecl(make_mw("logger", MiddlewareTarget::Global, vec![])),
    ]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    let main = ctx.files.get("src/main.rs").unwrap();
    assert!(
        main.contains("middleware::auth::middleware_fn"),
        "first middleware should be present: {main}"
    );
    assert!(
        main.contains("middleware::logger::middleware_fn"),
        "second middleware should be present: {main}"
    );
}

// ── Doc comments ───────────────────────────────────────────────────────

#[test]
fn middleware_doc_comment() {
    let out = emit_mw(&make_mw("auth", MiddlewareTarget::Global, vec![]));
    assert!(
        out.contains("/// Middleware: `auth`"),
        "should have doc comment: {out}"
    );
}

// ── API routes strip prefix in nest ────────────────────────────────────

#[test]
fn api_routes_strip_prefix_in_nest() {
    let interner = TypeInterner::new();
    let program = make_program(vec![
        Item::ApiDecl(ApiDecl {
            name: "Users".into(),
            handlers: vec![ApiHandler {
                method: HttpMethod::Get,
                path_params: vec![],
                params: vec![],
                ret_ty: None,
                body: Block {
                    stmts: vec![],
                    span: s(),
                },
                span: s(),
            }],
            span: s(),
        }),
        Item::MiddlewareDecl(make_mw(
            "apiAuth",
            MiddlewareTarget::PathPrefix("/api".into()),
            vec![],
        )),
    ]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    let main = ctx.files.get("src/main.rs").unwrap();
    // Inside the nested router, routes should have the /api prefix stripped
    // So /api/users becomes /users inside the nest
    assert!(
        main.contains("\"/users\""),
        "routes inside .nest() should have prefix stripped: {main}"
    );
}

// ── File header ────────────────────────────────────────────────────────

#[test]
fn middleware_file_header() {
    let out = emit_mw(&make_mw("auth", MiddlewareTarget::Global, vec![]));
    assert!(
        out.contains("Generated by GaleX compiler"),
        "should have file header: {out}"
    );
    assert!(
        out.contains("Middleware: `auth`"),
        "header should include middleware name: {out}"
    );
}
