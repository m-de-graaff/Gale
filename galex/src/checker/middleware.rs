//! Middleware validation (GX1300-GX1304).
//!
//! Validates middleware declarations for correct structure and ensures
//! no client-only code (signals, derives) appears in middleware bodies.

use crate::ast::*;
use crate::errors::{codes, Diagnostic};

/// Known valid head property names (used for quick reference).
const EXPECTED_PARAM_COUNT: usize = 2;

/// Validate a middleware declaration.
///
/// Checks:
/// - GX1301: Parameter count must be exactly 2 (`req`, `next`).
/// - GX1304: No signal/derive statements in the middleware body.
pub fn validate_middleware(decl: &MiddlewareDecl) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // GX1301: Check parameter count — expected `(req: Request, next: Next)`
    let param_count = decl.params.len();
    if param_count != EXPECTED_PARAM_COUNT {
        diagnostics.push(Diagnostic::with_message(
            &codes::GX1301,
            format!(
                "Middleware `{}` expects 2 parameters (req, next), found {}",
                decl.name, param_count
            ),
            decl.span,
        ));
    }

    // GX1304: Check for signal/client code in middleware body
    check_block_for_client_code(&decl.body, &mut diagnostics);

    diagnostics
}

/// Recursively check a block for client-only code inside middleware.
fn check_block_for_client_code(block: &Block, diagnostics: &mut Vec<Diagnostic>) {
    for stmt in &block.stmts {
        check_stmt_for_client_code(stmt, diagnostics);
    }
}

/// Check a single statement for client-only constructs.
fn check_stmt_for_client_code(stmt: &Stmt, diagnostics: &mut Vec<Diagnostic>) {
    match stmt {
        Stmt::Signal { span, name, .. } => {
            diagnostics.push(Diagnostic::with_message(
                &codes::GX1304,
                format!(
                    "Middleware contains signal `{}` — middleware is server-only",
                    name
                ),
                *span,
            ));
        }
        Stmt::Derive { span, name, .. } => {
            diagnostics.push(Diagnostic::with_message(
                &codes::GX1304,
                format!(
                    "Middleware contains derive `{}` — middleware is server-only",
                    name
                ),
                *span,
            ));
        }
        Stmt::Effect { span, .. } => {
            diagnostics.push(Diagnostic::with_message(
                &codes::GX1304,
                "Middleware contains `effect` — middleware is server-only".to_string(),
                *span,
            ));
        }
        Stmt::Watch { span, .. } => {
            diagnostics.push(Diagnostic::with_message(
                &codes::GX1304,
                "Middleware contains `watch` — middleware is server-only".to_string(),
                *span,
            ));
        }
        Stmt::RefDecl { span, name, .. } => {
            diagnostics.push(Diagnostic::with_message(
                &codes::GX1304,
                format!(
                    "Middleware contains ref `{}` — middleware is server-only",
                    name
                ),
                *span,
            ));
        }
        // Recurse into nested blocks
        Stmt::If {
            then_block,
            else_branch,
            ..
        } => {
            check_block_for_client_code(then_block, diagnostics);
            if let Some(ElseBranch::Else(block)) = else_branch {
                check_block_for_client_code(block, diagnostics);
            } else if let Some(ElseBranch::ElseIf(stmt)) = else_branch {
                check_stmt_for_client_code(stmt, diagnostics);
            }
        }
        Stmt::For { body, .. } => {
            check_block_for_client_code(body, diagnostics);
        }
        Stmt::Block(block) => {
            check_block_for_client_code(block, diagnostics);
        }
        Stmt::FnDecl(fn_decl) => {
            check_block_for_client_code(&fn_decl.body, diagnostics);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::span::Span;

    fn s() -> Span {
        Span::dummy()
    }

    #[test]
    fn middleware_with_wrong_param_count() {
        let decl = MiddlewareDecl {
            name: "auth".into(),
            target: MiddlewareTarget::Global,
            params: vec![Param {
                name: "req".into(),
                ty_ann: None,
                default: None,
                span: s(),
            }],
            body: Block {
                stmts: vec![],
                span: s(),
            },
            span: s(),
        };
        let diags = validate_middleware(&decl);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code.code, 1301);
    }

    #[test]
    fn middleware_with_signal_rejected() {
        let decl = MiddlewareDecl {
            name: "auth".into(),
            target: MiddlewareTarget::Global,
            params: vec![
                Param {
                    name: "req".into(),
                    ty_ann: None,
                    default: None,
                    span: s(),
                },
                Param {
                    name: "next".into(),
                    ty_ann: None,
                    default: None,
                    span: s(),
                },
            ],
            body: Block {
                stmts: vec![Stmt::Signal {
                    name: "count".into(),
                    ty_ann: None,
                    init: Expr::IntLit {
                        value: 0,
                        span: s(),
                    },
                    span: s(),
                }],
                span: s(),
            },
            span: s(),
        };
        let diags = validate_middleware(&decl);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code.code, 1304);
    }

    #[test]
    fn valid_middleware_no_errors() {
        let decl = MiddlewareDecl {
            name: "auth".into(),
            target: MiddlewareTarget::Global,
            params: vec![
                Param {
                    name: "req".into(),
                    ty_ann: None,
                    default: None,
                    span: s(),
                },
                Param {
                    name: "next".into(),
                    ty_ann: None,
                    default: None,
                    span: s(),
                },
            ],
            body: Block {
                stmts: vec![],
                span: s(),
            },
            span: s(),
        };
        let diags = validate_middleware(&decl);
        assert!(diags.is_empty());
    }
}
