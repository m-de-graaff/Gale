//! GaleX statement → Rust source emitter.
//!
//! Converts each [`Stmt`] variant into Rust code. Client-only constructs
//! (Signal, Derive, Watch, Effect, RefDecl) emit comment placeholders.

use crate::ast::*;
use crate::codegen::emit_expr::emit_expr;
use crate::codegen::rust_emitter::RustEmitter;
use crate::codegen::types::to_snake_case;

/// Emit a single GaleX statement as Rust source code.
pub fn emit_stmt(e: &mut RustEmitter, stmt: &Stmt) {
    match stmt {
        // ── Variable bindings ──────────────────────────────────
        Stmt::Let { name, init, .. } => {
            e.write(&format!("let {} = ", to_snake_case(name)));
            emit_expr(e, init);
            e.writeln(";");
        }
        Stmt::Mut { name, init, .. } => {
            e.write(&format!("let mut {} = ", to_snake_case(name)));
            emit_expr(e, init);
            e.writeln(";");
        }
        Stmt::Frozen { name, init, .. } => {
            // Frozen is immutable by default in Rust (same as `let`)
            e.write(&format!("let {} = ", to_snake_case(name)));
            emit_expr(e, init);
            e.writeln(";");
        }

        // ── Function declaration ───────────────────────────────
        Stmt::FnDecl(fn_decl) => {
            emit_fn_decl(e, fn_decl);
        }

        // ── Control flow ───────────────────────────────────────
        Stmt::If {
            condition,
            then_block,
            else_branch,
            ..
        } => {
            e.write("if ");
            emit_expr(e, condition);
            e.writeln(" {");
            e.indent();
            emit_block_body(e, then_block);
            e.dedent();
            match else_branch {
                Some(ElseBranch::Else(block)) => {
                    e.writeln("} else {");
                    e.indent();
                    emit_block_body(e, block);
                    e.dedent();
                    e.writeln("}");
                }
                Some(ElseBranch::ElseIf(stmt)) => {
                    e.write("} else ");
                    emit_stmt(e, stmt);
                }
                None => {
                    e.writeln("}");
                }
            }
        }

        Stmt::For {
            binding,
            index,
            iterable,
            body,
            ..
        } => {
            if let Some(idx) = index {
                e.write(&format!(
                    "for ({}, {}) in (",
                    to_snake_case(idx),
                    to_snake_case(binding)
                ));
                emit_expr(e, iterable);
                e.writeln(").iter().enumerate() {");
            } else {
                e.write(&format!("for {} in ", to_snake_case(binding)));
                emit_expr(e, iterable);
                e.writeln(" {");
            }
            e.indent();
            emit_block_body(e, body);
            e.dedent();
            e.writeln("}");
        }

        Stmt::Return { value, .. } => {
            if let Some(expr) = value {
                e.write("return ");
                emit_expr(e, expr);
                e.writeln(";");
            } else {
                e.writeln("return;");
            }
        }

        // ── Expression statement ───────────────────────────────
        Stmt::ExprStmt { expr, .. } => {
            emit_expr(e, expr);
            e.writeln(";");
        }

        // ── Nested block ───────────────────────────────────────
        Stmt::Block(block) => {
            e.writeln("{");
            e.indent();
            emit_block_body(e, block);
            e.dedent();
            e.writeln("}");
        }

        // ── Client-only constructs (placeholder) ───────────────
        Stmt::Signal { name, .. } => {
            e.writeln(&format!("// [client-only] signal {name}"));
        }
        Stmt::Derive { name, .. } => {
            e.writeln(&format!("// [client-only] derive {name}"));
        }
        Stmt::RefDecl { name, .. } => {
            e.writeln(&format!("// [client-only] ref {name}"));
        }
        Stmt::Effect { .. } => {
            e.writeln("// [client-only] effect { ... }");
        }
        Stmt::Watch { .. } => {
            e.writeln("// [client-only] watch { ... }");
        }
    }
}

/// Emit the body of a block (just the statements, no braces).
pub fn emit_block_body(e: &mut RustEmitter, block: &Block) {
    for stmt in &block.stmts {
        emit_stmt(e, stmt);
    }
}

/// Emit a function declaration.
fn emit_fn_decl(e: &mut RustEmitter, decl: &FnDecl) {
    let name = to_snake_case(&decl.name);
    let async_kw = if decl.is_async { "async " } else { "" };

    e.write(&format!("{async_kw}fn {name}("));
    for (i, p) in decl.params.iter().enumerate() {
        if i > 0 {
            e.write(", ");
        }
        let pname = to_snake_case(&p.name);
        if let Some(ann) = &p.ty_ann {
            let ty = annotation_to_rust(ann);
            e.write(&format!("{pname}: {ty}"));
        } else {
            // No annotation — use a dynamic fallback
            e.write(&format!("{pname}: serde_json::Value"));
        }
    }
    e.write(")");

    // Return type
    if let Some(ann) = &decl.ret_ty {
        let ty = annotation_to_rust(ann);
        if ty != "()" {
            e.write(&format!(" -> {ty}"));
        }
    }

    e.writeln(" {");
    e.indent();
    emit_block_body(e, &decl.body);
    e.dedent();
    e.writeln("}");
}

/// Convert a TypeAnnotation to a Rust type string.
///
/// This is a simpler, AST-level mapping (vs. the TypeId-based `type_to_rust`
/// in `types.rs` which works from resolved types). Used when we only have
/// the AST annotation, not the resolved TypeId.
pub fn annotation_to_rust(ann: &TypeAnnotation) -> String {
    match ann {
        TypeAnnotation::Named { name, .. } => match name.as_str() {
            "string" => "String".into(),
            "int" => "i64".into(),
            "float" => "f64".into(),
            "bool" => "bool".into(),
            "void" => "()".into(),
            other => other.to_string(), // Guard/Enum/TypeAlias names pass through
        },
        TypeAnnotation::Array { element, .. } => {
            format!("Vec<{}>", annotation_to_rust(element))
        }
        TypeAnnotation::Optional { inner, .. } => {
            format!("Option<{}>", annotation_to_rust(inner))
        }
        TypeAnnotation::Tuple { elements, .. } => {
            let parts: Vec<String> = elements.iter().map(annotation_to_rust).collect();
            format!("({})", parts.join(", "))
        }
        TypeAnnotation::Union { .. } => "serde_json::Value".into(),
        TypeAnnotation::StringLiteral { .. } => "String".into(),
        TypeAnnotation::Function { params, ret, .. } => {
            let p: Vec<String> = params.iter().map(annotation_to_rust).collect();
            format!("fn({}) -> {}", p.join(", "), annotation_to_rust(ret))
        }
        TypeAnnotation::Object { .. } => "serde_json::Value".into(),
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::rust_emitter::RustEmitter;
    use crate::span::Span;

    fn s() -> Span {
        Span::dummy()
    }

    fn emit(stmt: &Stmt) -> String {
        let mut e = RustEmitter::new();
        emit_stmt(&mut e, stmt);
        e.finish()
    }

    #[test]
    fn let_statement() {
        let out = emit(&Stmt::Let {
            name: "count".into(),
            ty_ann: None,
            init: Expr::IntLit {
                value: 42,
                span: s(),
            },
            span: s(),
        });
        assert_eq!(out, "let count = 42_i64;\n");
    }

    #[test]
    fn mut_statement() {
        let out = emit(&Stmt::Mut {
            name: "total".into(),
            ty_ann: None,
            init: Expr::IntLit {
                value: 0,
                span: s(),
            },
            span: s(),
        });
        assert_eq!(out, "let mut total = 0_i64;\n");
    }

    #[test]
    fn frozen_statement() {
        let out = emit(&Stmt::Frozen {
            name: "pi".into(),
            init: Expr::FloatLit {
                value: 3.14,
                span: s(),
            },
            span: s(),
        });
        assert_eq!(out, "let pi = 3.14_f64;\n");
    }

    #[test]
    fn return_with_value() {
        let out = emit(&Stmt::Return {
            value: Some(Expr::Ident {
                name: "result".into(),
                span: s(),
            }),
            span: s(),
        });
        assert_eq!(out, "return result;\n");
    }

    #[test]
    fn return_void() {
        let out = emit(&Stmt::Return {
            value: None,
            span: s(),
        });
        assert_eq!(out, "return;\n");
    }

    #[test]
    fn if_else() {
        let out = emit(&Stmt::If {
            condition: Expr::BoolLit {
                value: true,
                span: s(),
            },
            then_block: Block {
                stmts: vec![Stmt::Return {
                    value: Some(Expr::IntLit {
                        value: 1,
                        span: s(),
                    }),
                    span: s(),
                }],
                span: s(),
            },
            else_branch: Some(ElseBranch::Else(Block {
                stmts: vec![Stmt::Return {
                    value: Some(Expr::IntLit {
                        value: 2,
                        span: s(),
                    }),
                    span: s(),
                }],
                span: s(),
            })),
            span: s(),
        });
        assert!(out.contains("if true {"));
        assert!(out.contains("return 1_i64;"));
        assert!(out.contains("} else {"));
        assert!(out.contains("return 2_i64;"));
    }

    #[test]
    fn for_loop() {
        let out = emit(&Stmt::For {
            binding: "item".into(),
            index: None,
            iterable: Expr::Ident {
                name: "items".into(),
                span: s(),
            },
            body: Block {
                stmts: vec![Stmt::ExprStmt {
                    expr: Expr::Ident {
                        name: "item".into(),
                        span: s(),
                    },
                    span: s(),
                }],
                span: s(),
            },
            span: s(),
        });
        assert!(out.contains("for item in items {"));
    }

    #[test]
    fn client_only_skipped() {
        let out = emit(&Stmt::Signal {
            name: "count".into(),
            ty_ann: None,
            init: Expr::IntLit {
                value: 0,
                span: s(),
            },
            span: s(),
        });
        assert!(out.contains("[client-only]"));
    }

    #[test]
    fn annotation_to_rust_primitives() {
        assert_eq!(
            annotation_to_rust(&TypeAnnotation::Named {
                name: "string".into(),
                span: s()
            }),
            "String"
        );
        assert_eq!(
            annotation_to_rust(&TypeAnnotation::Named {
                name: "int".into(),
                span: s()
            }),
            "i64"
        );
        assert_eq!(
            annotation_to_rust(&TypeAnnotation::Named {
                name: "bool".into(),
                span: s()
            }),
            "bool"
        );
    }

    #[test]
    fn annotation_to_rust_compound() {
        let arr = TypeAnnotation::Array {
            element: Box::new(TypeAnnotation::Named {
                name: "int".into(),
                span: s(),
            }),
            span: s(),
        };
        assert_eq!(annotation_to_rust(&arr), "Vec<i64>");

        let opt = TypeAnnotation::Optional {
            inner: Box::new(TypeAnnotation::Named {
                name: "string".into(),
                span: s(),
            }),
            span: s(),
        };
        assert_eq!(annotation_to_rust(&opt), "Option<String>");
    }
}
