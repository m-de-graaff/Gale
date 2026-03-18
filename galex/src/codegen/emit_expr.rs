//! GaleX expression → Rust source emitter.
//!
//! Converts each [`Expr`] variant into idiomatic Rust code. This covers
//! the server-relevant subset; client-only constructs emit placeholder
//! comments.

use crate::ast::*;
use crate::codegen::rust_emitter::RustEmitter;
use crate::codegen::types::to_snake_case;

/// Emit a GaleX expression as Rust source code into the buffer.
pub fn emit_expr(e: &mut RustEmitter, expr: &Expr) {
    match expr {
        // ── Literals ───────────────────────────────────────────
        Expr::IntLit { value, .. } => {
            e.write(&format!("{value}_i64"));
        }
        Expr::FloatLit { value, .. } => {
            // Ensure the float always has a decimal point for Rust
            let s = if value.fract() == 0.0 {
                format!("{value:.1}_f64")
            } else {
                format!("{value}_f64")
            };
            e.write(&s);
        }
        Expr::StringLit { value, .. } => {
            let escaped = escape_rust_string(value);
            e.write(&format!("String::from(\"{escaped}\")"));
        }
        Expr::BoolLit { value, .. } => {
            e.write(if *value { "true" } else { "false" });
        }
        Expr::NullLit { .. } => {
            e.write("None");
        }
        Expr::RegexLit { pattern, flags, .. } => {
            e.write(&format!(
                "/* regex /{pattern}/{flags} — not supported server-side */ String::new()"
            ));
        }

        // ── Identifiers ────────────────────────────────────────
        Expr::Ident { name, .. } => {
            e.write(&to_snake_case(name));
        }

        // ── Template literals ──────────────────────────────────
        Expr::TemplateLit { parts, .. } => {
            emit_template_literal(e, parts);
        }

        // ── Binary operations ──────────────────────────────────
        Expr::BinaryOp {
            left, op, right, ..
        } => {
            // DotDot (range) is special — produces a collected Vec
            if *op == BinOp::DotDot {
                e.write("(");
                emit_expr(e, left);
                e.write("..");
                emit_expr(e, right);
                e.write(").collect::<Vec<i64>>()");
            } else {
                e.write("(");
                emit_expr(e, left);
                e.write(&format!(" {} ", binop_to_rust(*op)));
                emit_expr(e, right);
                e.write(")");
            }
        }

        // ── Unary operations ───────────────────────────────────
        Expr::UnaryOp { op, operand, .. } => match op {
            UnaryOp::Neg => {
                e.write("(-");
                emit_expr(e, operand);
                e.write(")");
            }
            UnaryOp::Not => {
                e.write("(!");
                emit_expr(e, operand);
                e.write(")");
            }
        },

        // ── Ternary ────────────────────────────────────────────
        Expr::Ternary {
            condition,
            then_expr,
            else_expr,
            ..
        } => {
            e.write("if ");
            emit_expr(e, condition);
            e.write(" { ");
            emit_expr(e, then_expr);
            e.write(" } else { ");
            emit_expr(e, else_expr);
            e.write(" }");
        }

        // ── Null coalesce ──────────────────────────────────────
        Expr::NullCoalesce { left, right, .. } => {
            emit_expr(e, left);
            e.write(".unwrap_or_else(|| ");
            emit_expr(e, right);
            e.write(")");
        }

        // ── Function call ──────────────────────────────────────
        Expr::FnCall { callee, args, .. } => {
            emit_expr(e, callee);
            e.write("(");
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    e.write(", ");
                }
                emit_expr(e, arg);
            }
            e.write(")");
        }

        // ── Member access ──────────────────────────────────────
        Expr::MemberAccess { object, field, .. } => {
            emit_expr(e, object);
            e.write(&format!(".{}", to_snake_case(field)));
        }

        // ── Optional chain ─────────────────────────────────────
        Expr::OptionalChain { object, field, .. } => {
            emit_expr(e, object);
            e.write(&format!(
                ".as_ref().map(|__v| __v.{}.clone())",
                to_snake_case(field)
            ));
        }

        // ── Index access ───────────────────────────────────────
        Expr::IndexAccess { object, index, .. } => {
            emit_expr(e, object);
            e.write("[");
            emit_expr(e, index);
            e.write(" as usize]");
        }

        // ── Array literal ──────────────────────────────────────
        Expr::ArrayLit { elements, .. } => {
            if elements.is_empty() {
                e.write("vec![]");
            } else {
                e.write("vec![");
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        e.write(", ");
                    }
                    emit_expr(e, elem);
                }
                e.write("]");
            }
        }

        // ── Object literal ─────────────────────────────────────
        Expr::ObjectLit { fields, .. } => {
            if fields.is_empty() {
                e.write("serde_json::json!({})");
            } else {
                e.write("serde_json::json!({");
                for (i, field) in fields.iter().enumerate() {
                    if i > 0 {
                        e.write(", ");
                    }
                    e.write(&format!("\"{}\": ", field.key));
                    emit_expr(e, &field.value);
                }
                e.write("})");
            }
        }

        // ── Arrow function ─────────────────────────────────────
        Expr::ArrowFn { params, body, .. } => {
            e.write("|");
            for (i, p) in params.iter().enumerate() {
                if i > 0 {
                    e.write(", ");
                }
                e.write(&to_snake_case(&p.name));
            }
            e.write("| ");
            match body {
                ArrowBody::Expr(expr) => emit_expr(e, expr),
                ArrowBody::Block(block) => {
                    e.write("{\n");
                    e.indent();
                    super::emit_stmt::emit_block_body(e, block);
                    e.dedent();
                    e.write("}");
                }
            }
        }

        // ── Spread ─────────────────────────────────────────────
        Expr::Spread { expr, .. } => {
            // In Rust, spread doesn't exist directly. Emit the inner
            // expression with a comment marker.
            emit_expr(e, expr);
        }

        // ── Range ──────────────────────────────────────────────
        Expr::Range { start, end, .. } => {
            e.write("(");
            emit_expr(e, start);
            e.write("..");
            emit_expr(e, end);
            e.write(").collect::<Vec<i64>>()");
        }

        // ── Pipe ───────────────────────────────────────────────
        Expr::Pipe { left, right, .. } => {
            // `left |> right` → `right(left)`
            emit_expr(e, right);
            e.write("(");
            emit_expr(e, left);
            e.write(")");
        }

        // ── Await ──────────────────────────────────────────────
        Expr::Await { expr, .. } => {
            emit_expr(e, expr);
            e.write(".await");
        }

        // ── Assignment ─────────────────────────────────────────
        Expr::Assign {
            target, op, value, ..
        } => {
            emit_expr(e, target);
            e.write(match op {
                AssignOp::Assign => " = ",
                AssignOp::AddAssign => " += ",
                AssignOp::SubAssign => " -= ",
            });
            emit_expr(e, value);
        }

        // ── Assert ─────────────────────────────────────────────
        Expr::Assert { expr, .. } => {
            e.write("assert!(");
            emit_expr(e, expr);
            e.write(")");
        }

        // ── Env access ─────────────────────────────────────────
        Expr::EnvAccess { key, .. } => {
            let declared = crate::codegen::expr::get_declared_env_keys();
            e.write(&crate::codegen::emit_env::env_access_expr(key, &declared));
        }
    }
}

// ── Helpers ────────────────────────────────────────────────────────────

/// Map a GaleX binary operator to its Rust equivalent.
fn binop_to_rust(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%",
        BinOp::Eq => "==",
        BinOp::NotEq => "!=",
        BinOp::Lt => "<",
        BinOp::Gt => ">",
        BinOp::LtEq => "<=",
        BinOp::GtEq => ">=",
        BinOp::And => "&&",
        BinOp::Or => "||",
        BinOp::DotDot => "..", // handled specially in emit_expr
    }
}

/// Emit a template literal as `format!("...", args)`.
fn emit_template_literal(e: &mut RustEmitter, parts: &[TemplatePart]) {
    // Build the format string and collect argument expressions
    let mut fmt_str = String::new();
    let mut args: Vec<&Expr> = Vec::new();

    for part in parts {
        match part {
            TemplatePart::Text(text) => {
                // Escape braces for format! and escape Rust string chars
                fmt_str.push_str(
                    &escape_rust_string(text)
                        .replace('{', "{{")
                        .replace('}', "}}"),
                );
            }
            TemplatePart::Expr(expr) => {
                fmt_str.push_str("{}");
                args.push(expr);
            }
        }
    }

    if args.is_empty() {
        // No interpolation — just a plain string
        e.write(&format!("String::from(\"{fmt_str}\")"));
    } else {
        e.write(&format!("format!(\"{fmt_str}\""));
        for arg in args {
            e.write(", ");
            emit_expr(e, arg);
        }
        e.write(")");
    }
}

/// Escape a string for use inside Rust double quotes.
fn escape_rust_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
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

    fn emit(expr: &Expr) -> String {
        let mut e = RustEmitter::new();
        emit_expr(&mut e, expr);
        e.finish()
    }

    #[test]
    fn int_literal() {
        assert_eq!(
            emit(&Expr::IntLit {
                value: 42,
                span: s()
            }),
            "42_i64"
        );
    }

    #[test]
    fn float_literal() {
        assert_eq!(
            emit(&Expr::FloatLit {
                value: 3.14,
                span: s()
            }),
            "3.14_f64"
        );
    }

    #[test]
    fn string_literal() {
        assert_eq!(
            emit(&Expr::StringLit {
                value: "hello".into(),
                span: s()
            }),
            "String::from(\"hello\")"
        );
    }

    #[test]
    fn string_literal_escapes() {
        assert_eq!(
            emit(&Expr::StringLit {
                value: "say \"hi\"".into(),
                span: s()
            }),
            "String::from(\"say \\\"hi\\\"\")"
        );
    }

    #[test]
    fn bool_literal() {
        assert_eq!(
            emit(&Expr::BoolLit {
                value: true,
                span: s()
            }),
            "true"
        );
        assert_eq!(
            emit(&Expr::BoolLit {
                value: false,
                span: s()
            }),
            "false"
        );
    }

    #[test]
    fn null_literal() {
        assert_eq!(emit(&Expr::NullLit { span: s() }), "None");
    }

    #[test]
    fn identifier() {
        assert_eq!(
            emit(&Expr::Ident {
                name: "userName".into(),
                span: s()
            }),
            "user_name"
        );
    }

    #[test]
    fn binary_add() {
        let expr = Expr::BinaryOp {
            left: Box::new(Expr::IntLit {
                value: 1,
                span: s(),
            }),
            op: BinOp::Add,
            right: Box::new(Expr::IntLit {
                value: 2,
                span: s(),
            }),
            span: s(),
        };
        assert_eq!(emit(&expr), "(1_i64 + 2_i64)");
    }

    #[test]
    fn binary_and() {
        let expr = Expr::BinaryOp {
            left: Box::new(Expr::BoolLit {
                value: true,
                span: s(),
            }),
            op: BinOp::And,
            right: Box::new(Expr::BoolLit {
                value: false,
                span: s(),
            }),
            span: s(),
        };
        assert_eq!(emit(&expr), "(true && false)");
    }

    #[test]
    fn unary_neg() {
        let expr = Expr::UnaryOp {
            op: UnaryOp::Neg,
            operand: Box::new(Expr::IntLit {
                value: 5,
                span: s(),
            }),
            span: s(),
        };
        assert_eq!(emit(&expr), "(-5_i64)");
    }

    #[test]
    fn ternary() {
        let expr = Expr::Ternary {
            condition: Box::new(Expr::BoolLit {
                value: true,
                span: s(),
            }),
            then_expr: Box::new(Expr::IntLit {
                value: 1,
                span: s(),
            }),
            else_expr: Box::new(Expr::IntLit {
                value: 2,
                span: s(),
            }),
            span: s(),
        };
        assert_eq!(emit(&expr), "if true { 1_i64 } else { 2_i64 }");
    }

    #[test]
    fn fn_call() {
        let expr = Expr::FnCall {
            callee: Box::new(Expr::Ident {
                name: "foo".into(),
                span: s(),
            }),
            args: vec![
                Expr::IntLit {
                    value: 1,
                    span: s(),
                },
                Expr::StringLit {
                    value: "a".into(),
                    span: s(),
                },
            ],
            span: s(),
        };
        assert_eq!(emit(&expr), "foo(1_i64, String::from(\"a\"))");
    }

    #[test]
    fn member_access() {
        let expr = Expr::MemberAccess {
            object: Box::new(Expr::Ident {
                name: "user".into(),
                span: s(),
            }),
            field: "firstName".into(),
            span: s(),
        };
        assert_eq!(emit(&expr), "user.first_name");
    }

    #[test]
    fn array_literal() {
        let expr = Expr::ArrayLit {
            elements: vec![
                Expr::IntLit {
                    value: 1,
                    span: s(),
                },
                Expr::IntLit {
                    value: 2,
                    span: s(),
                },
            ],
            span: s(),
        };
        assert_eq!(emit(&expr), "vec![1_i64, 2_i64]");
    }

    #[test]
    fn empty_array() {
        let expr = Expr::ArrayLit {
            elements: vec![],
            span: s(),
        };
        assert_eq!(emit(&expr), "vec![]");
    }

    #[test]
    fn object_literal() {
        let expr = Expr::ObjectLit {
            fields: vec![ObjectFieldExpr {
                key: "id".into(),
                value: Expr::IntLit {
                    value: 1,
                    span: s(),
                },
                span: s(),
            }],
            span: s(),
        };
        assert_eq!(emit(&expr), "serde_json::json!({\"id\": 1_i64})");
    }

    #[test]
    fn template_literal() {
        let expr = Expr::TemplateLit {
            parts: vec![
                TemplatePart::Text("Hello, ".into()),
                TemplatePart::Expr(Expr::Ident {
                    name: "name".into(),
                    span: s(),
                }),
                TemplatePart::Text("!".into()),
            ],
            span: s(),
        };
        assert_eq!(emit(&expr), "format!(\"Hello, {}!\", name)");
    }

    #[test]
    fn template_literal_no_interp() {
        let expr = Expr::TemplateLit {
            parts: vec![TemplatePart::Text("plain".into())],
            span: s(),
        };
        assert_eq!(emit(&expr), "String::from(\"plain\")");
    }

    #[test]
    fn env_access() {
        let expr = Expr::EnvAccess {
            key: "DB_URL".into(),
            span: s(),
        };
        assert_eq!(emit(&expr), "std::env::var(\"DB_URL\").unwrap_or_default()");
    }

    #[test]
    fn pipe_expression() {
        let expr = Expr::Pipe {
            left: Box::new(Expr::IntLit {
                value: 5,
                span: s(),
            }),
            right: Box::new(Expr::Ident {
                name: "double".into(),
                span: s(),
            }),
            span: s(),
        };
        assert_eq!(emit(&expr), "double(5_i64)");
    }

    #[test]
    fn await_expression() {
        let expr = Expr::Await {
            expr: Box::new(Expr::FnCall {
                callee: Box::new(Expr::Ident {
                    name: "fetch".into(),
                    span: s(),
                }),
                args: vec![],
                span: s(),
            }),
            span: s(),
        };
        assert_eq!(emit(&expr), "fetch().await");
    }

    #[test]
    fn assign_expression() {
        let expr = Expr::Assign {
            target: Box::new(Expr::Ident {
                name: "x".into(),
                span: s(),
            }),
            op: AssignOp::AddAssign,
            value: Box::new(Expr::IntLit {
                value: 1,
                span: s(),
            }),
            span: s(),
        };
        assert_eq!(emit(&expr), "x += 1_i64");
    }
}
