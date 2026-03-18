//! Reactivity validation (GX1600-GX1610).
//!
//! Validates reactive declarations (signals, derives, effects, watches)
//! ensuring they are used in the correct context.

use crate::ast::*;
use crate::errors::{codes, Diagnostic};

/// Validate reactivity rules across a program.
///
/// Checks:
/// - GX1600: Signal declared outside component, client block, or store
pub fn validate_reactivity(program: &Program) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    for item in &program.items {
        match item {
            // Signals at the top level are invalid
            Item::Stmt(Stmt::Signal { span, name, .. }) => {
                diagnostics.push(Diagnostic::with_message(
                    &codes::GX1600,
                    format!(
                        "`signal {}` declared outside component or client block",
                        name
                    ),
                    *span,
                ));
            }
            // Check inside component bodies for reactivity issues
            Item::ComponentDecl(comp) => {
                validate_component_reactivity(comp, &mut diagnostics);
            }
            Item::LayoutDecl(layout) => {
                validate_layout_reactivity(layout, &mut diagnostics);
            }
            // Signals inside server blocks are invalid (but that's GX0507, handled by boundary)
            // Signals inside middleware are invalid (GX1304, handled by middleware validator)
            Item::Out(out) => match out.inner.as_ref() {
                Item::ComponentDecl(comp) => {
                    validate_component_reactivity(comp, &mut diagnostics);
                }
                Item::LayoutDecl(layout) => {
                    validate_layout_reactivity(layout, &mut diagnostics);
                }
                _ => {}
            },
            _ => {}
        }
    }

    diagnostics
}

/// Check reactivity within a component body.
fn validate_component_reactivity(comp: &ComponentDecl, diagnostics: &mut Vec<Diagnostic>) {
    for stmt in &comp.body.stmts {
        check_stmt_reactivity(stmt, diagnostics);
    }
}

/// Check reactivity within a layout body.
fn validate_layout_reactivity(layout: &LayoutDecl, diagnostics: &mut Vec<Diagnostic>) {
    for stmt in &layout.body.stmts {
        check_stmt_reactivity(stmt, diagnostics);
    }
}

/// Check individual reactive statements for common issues.
fn check_stmt_reactivity(stmt: &Stmt, diagnostics: &mut Vec<Diagnostic>) {
    match stmt {
        // GX1605: Signal mutated inside derive
        Stmt::Derive { init, span, .. } => {
            if expr_contains_signal_mutation(init) {
                diagnostics.push(Diagnostic::with_message(
                    &codes::GX1605,
                    "Derive body must be pure — it cannot write to signals".to_string(),
                    *span,
                ));
            }
        }
        // Recurse into nested blocks
        Stmt::If {
            then_block,
            else_branch,
            ..
        } => {
            for s in &then_block.stmts {
                check_stmt_reactivity(s, diagnostics);
            }
            if let Some(ElseBranch::Else(block)) = else_branch {
                for s in &block.stmts {
                    check_stmt_reactivity(s, diagnostics);
                }
            }
        }
        Stmt::For { body, .. } => {
            for s in &body.stmts {
                check_stmt_reactivity(s, diagnostics);
            }
        }
        Stmt::Block(block) => {
            for s in &block.stmts {
                check_stmt_reactivity(s, diagnostics);
            }
        }
        _ => {}
    }
}

/// Check if an expression contains a signal mutation (assignment).
///
/// This is a basic heuristic — it looks for `=`, `+=`, `-=` assignments
/// in the expression tree. A full analysis would require type information
/// to determine if the target is actually a signal.
fn expr_contains_signal_mutation(expr: &Expr) -> bool {
    match expr {
        Expr::Assign { .. } => true,
        Expr::FnCall { callee, args, .. } => {
            expr_contains_signal_mutation(callee) || args.iter().any(expr_contains_signal_mutation)
        }
        Expr::BinaryOp { left, right, .. } => {
            expr_contains_signal_mutation(left) || expr_contains_signal_mutation(right)
        }
        Expr::UnaryOp { operand, .. } => expr_contains_signal_mutation(operand),
        Expr::Ternary {
            condition,
            then_expr,
            else_expr,
            ..
        } => {
            expr_contains_signal_mutation(condition)
                || expr_contains_signal_mutation(then_expr)
                || expr_contains_signal_mutation(else_expr)
        }
        Expr::MemberAccess { object, .. } | Expr::OptionalChain { object, .. } => {
            expr_contains_signal_mutation(object)
        }
        Expr::Pipe { left, right, .. } | Expr::NullCoalesce { left, right, .. } => {
            expr_contains_signal_mutation(left) || expr_contains_signal_mutation(right)
        }
        Expr::ArrowFn { .. } => {
            // Arrow functions inside derives are closures — they don't execute
            // during the derive evaluation, so mutations inside them are fine
            // (they'd be caught when the closure is called)
            false
        }
        _ => false,
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
    fn top_level_signal_rejected() {
        let program = Program {
            items: vec![Item::Stmt(Stmt::Signal {
                name: "count".into(),
                ty_ann: None,
                init: Expr::IntLit {
                    value: 0,
                    span: s(),
                },
                span: s(),
            })],
            span: s(),
        };
        let diags = validate_reactivity(&program);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code.code, 1600);
    }

    #[test]
    fn signal_inside_component_ok() {
        let program = Program {
            items: vec![Item::ComponentDecl(ComponentDecl {
                name: "Counter".into(),
                props: vec![],
                body: ComponentBody {
                    stmts: vec![Stmt::Signal {
                        name: "count".into(),
                        ty_ann: None,
                        init: Expr::IntLit {
                            value: 0,
                            span: s(),
                        },
                        span: s(),
                    }],
                    template: vec![],
                    head: None,
                    span: s(),
                },
                span: s(),
            })],
            span: s(),
        };
        let diags = validate_reactivity(&program);
        // No GX1600 errors — signals inside components are valid
        assert!(diags.iter().all(|d| d.code.code != 1600));
    }

    #[test]
    fn derive_with_mutation_rejected() {
        let program = Program {
            items: vec![Item::ComponentDecl(ComponentDecl {
                name: "Test".into(),
                props: vec![],
                body: ComponentBody {
                    stmts: vec![Stmt::Derive {
                        name: "doubled".into(),
                        init: Expr::Assign {
                            target: Box::new(Expr::Ident {
                                name: "x".into(),
                                span: s(),
                            }),
                            op: AssignOp::Assign,
                            value: Box::new(Expr::IntLit {
                                value: 5,
                                span: s(),
                            }),
                            span: s(),
                        },
                        span: s(),
                    }],
                    template: vec![],
                    head: None,
                    span: s(),
                },
                span: s(),
            })],
            span: s(),
        };
        let diags = validate_reactivity(&program);
        assert!(diags.iter().any(|d| d.code.code == 1605));
    }
}
