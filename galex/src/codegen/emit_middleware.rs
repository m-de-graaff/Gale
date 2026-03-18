//! Middleware handler generator.
//!
//! For each GaleX [`MiddlewareDecl`], generates a Rust file containing
//! an async middleware function compatible with `axum::middleware::from_fn`.
//!
//! The generated code wraps the raw `Request<Body>` in a `GaleRequest`
//! helper and maps `Response` constructors to `GaleResponse`.

use crate::ast::*;
use crate::codegen::emit_expr::emit_expr;
use crate::codegen::rust_emitter::RustEmitter;
use crate::codegen::types::to_snake_case;

/// Emit a complete middleware Rust file.
pub fn emit_middleware_file(e: &mut RustEmitter, decl: &MiddlewareDecl) {
    let mw_name = &decl.name;
    e.emit_file_header(&format!("Middleware: `{mw_name}`."));
    e.newline();

    // Imports
    e.emit_use("axum::body::Body");
    e.emit_use("axum::http::Request");
    e.emit_use("axum::middleware::Next");
    e.emit_use("axum::response::Response");
    e.emit_use("crate::gale_middleware::{GaleRequest, GaleResponse}");
    e.newline();

    // Determine the req/next parameter names from the declaration
    let req_name = param_name_for_kind(&decl.params, "Request").unwrap_or("req");
    let next_name = param_name_for_kind(&decl.params, "Next").unwrap_or("next");

    // Doc comment
    e.emit_doc_comment(&format!("Middleware: `{mw_name}`"));

    // Function signature
    e.block(
        "pub async fn middleware_fn(\n    request: Request<Body>,\n    next: Next,\n) -> Response",
        |e| {
            // Wrap the raw request in GaleRequest
            e.writeln(&format!("let mut {req_name} = GaleRequest(request);"));
            e.newline();

            // Emit the middleware body with special transformations
            emit_middleware_body(e, &decl.body, req_name, next_name);
        },
    );
}

// ── Middleware body emission ────────────────────────────────────────────

/// Emit the middleware body with request/response/next transformations.
///
/// This walks each statement and rewrites middleware-specific patterns:
/// - `next(req)` → `next.run(req.into_inner()).await`
/// - `Response.status(N)` → `GaleResponse::status(N)`
/// - `Response.json(data)` → `GaleResponse::json(data)`
/// - `Response.text(s)` → `GaleResponse::text(s)`
/// - `Response.redirect(url)` → `GaleResponse::redirect(url)`
///
/// All other statements delegate to the standard `emit_stmt` pipeline.
fn emit_middleware_body(e: &mut RustEmitter, block: &Block, req_name: &str, next_name: &str) {
    for stmt in &block.stmts {
        emit_middleware_stmt(e, stmt, req_name, next_name);
    }
}

/// Emit a single middleware statement with transformations.
fn emit_middleware_stmt(e: &mut RustEmitter, stmt: &Stmt, req_name: &str, next_name: &str) {
    match stmt {
        // Let binding: check if init is a next() call or Response builder
        Stmt::Let { name, init, .. } | Stmt::Mut { name, init, .. } => {
            let var_name = to_snake_case(name);
            let is_mut = matches!(stmt, Stmt::Mut { .. });
            if is_mut {
                e.write(&format!("let mut {var_name} = "));
            } else {
                e.write(&format!("let {var_name} = "));
            }
            emit_middleware_expr(e, init, req_name, next_name);
            e.writeln(";");
        }

        // Return: check if returning a Response builder or next result
        Stmt::Return { value, .. } => {
            if let Some(expr) = value {
                e.write("return ");
                emit_middleware_expr(e, expr, req_name, next_name);
                e.writeln(";");
            } else {
                e.writeln("return;");
            }
        }

        // Expression statement: check for next() calls
        Stmt::ExprStmt { expr, .. } => {
            emit_middleware_expr(e, expr, req_name, next_name);
            e.writeln(";");
        }

        // If/when blocks: recurse into branches
        Stmt::If {
            condition,
            then_block,
            else_branch,
            ..
        } => {
            e.write("if ");
            emit_middleware_expr(e, condition, req_name, next_name);
            e.writeln(" {");
            e.indent();
            emit_middleware_body(e, then_block, req_name, next_name);
            e.dedent();
            match else_branch {
                Some(ElseBranch::Else(block)) => {
                    e.writeln("} else {");
                    e.indent();
                    emit_middleware_body(e, block, req_name, next_name);
                    e.dedent();
                    e.writeln("}");
                }
                Some(ElseBranch::ElseIf(inner)) => {
                    e.write("} else ");
                    emit_middleware_stmt(e, inner, req_name, next_name);
                }
                None => {
                    e.writeln("}");
                }
            }
        }

        // Block statement: recurse
        Stmt::Block(block) => {
            e.writeln("{");
            e.indent();
            for s in &block.stmts {
                emit_middleware_stmt(e, s, req_name, next_name);
            }
            e.dedent();
            e.writeln("}");
        }

        // Everything else: delegate to standard emitter
        _ => {
            crate::codegen::emit_stmt::emit_stmt(e, stmt);
        }
    }
}

/// Emit a middleware expression with special-case transformations.
fn emit_middleware_expr(e: &mut RustEmitter, expr: &Expr, req_name: &str, next_name: &str) {
    match expr {
        // next(req) → next.run(req.into_inner()).await
        Expr::FnCall { callee, args, .. } => {
            // Check for next(req) pattern
            if is_ident(callee, next_name) {
                e.write("next.run(");
                if let Some(arg) = args.first() {
                    if is_ident(arg, req_name) {
                        e.write(&format!("{req_name}.into_inner()"));
                    } else {
                        emit_middleware_expr(e, arg, req_name, next_name);
                    }
                }
                e.write(").await");
                return;
            }

            // Check for Response.method(args) → GaleResponse::method(args)
            if let Expr::MemberAccess { object, field, .. } = callee.as_ref() {
                if is_ident(object, "Response") {
                    e.write(&format!("GaleResponse::{}(", to_snake_case(field)));
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 {
                            e.write(", ");
                        }
                        emit_middleware_expr(e, arg, req_name, next_name);
                    }
                    e.write(")");
                    return;
                }
            }

            // Default: standard fn call emission
            emit_middleware_expr(e, callee, req_name, next_name);
            e.write("(");
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    e.write(", ");
                }
                emit_middleware_expr(e, arg, req_name, next_name);
            }
            e.write(")");
        }

        // Response (bare identifier) → GaleResponse
        Expr::Ident { name, .. } if name == "Response" => {
            e.write("GaleResponse");
        }

        // For all other expressions, delegate to the standard emitter
        _ => {
            emit_expr(e, expr);
        }
    }
}

// ── Helpers ────────────────────────────────────────────────────────────

/// Check if an expression is an identifier with the given name.
fn is_ident(expr: &Expr, name: &str) -> bool {
    matches!(expr, Expr::Ident { name: n, .. } if n == name)
}

/// Find the parameter name for a given type annotation (e.g., "Request" → "req").
fn param_name_for_kind<'a>(params: &'a [Param], type_name: &str) -> Option<&'a str> {
    for param in params {
        if let Some(TypeAnnotation::Named { name, .. }) = &param.ty_ann {
            if name == type_name {
                return Some(&param.name);
            }
        }
    }
    None
}
