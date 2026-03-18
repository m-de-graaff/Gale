//! GaleX expression → JavaScript expression converter.
//!
//! Converts AST [`Expr`] nodes into JavaScript source strings for use
//! in generated per-page hydration scripts. Signal-aware: identifiers
//! known to be signals emit `.get()` calls for reactive tracking.

use std::collections::HashSet;

use crate::ast::*;

/// Convert a GaleX expression to a JavaScript expression string.
///
/// `signal_names` is the set of identifiers that are signals — these
/// get `.get()` appended when read, enabling reactive dependency tracking.
pub fn expr_to_js(expr: &Expr, signal_names: &HashSet<String>) -> String {
    match expr {
        // ── Literals ────────────────────────────────────────────
        Expr::IntLit { value, .. } => format!("{value}"),
        Expr::FloatLit { value, .. } => format!("{value}"),
        Expr::StringLit { value, .. } => format!("{:?}", value.as_str()),
        Expr::BoolLit { value, .. } => format!("{value}"),
        Expr::NullLit { .. } => "null".into(),
        Expr::RegexLit { pattern, flags, .. } => format!("/{pattern}/{flags}"),

        // ── Identifier ──────────────────────────────────────────
        Expr::Ident { name, .. } => {
            if signal_names.contains(name.as_str()) {
                format!("{name}.get()")
            } else {
                name.to_string()
            }
        }

        // ── Template literal ────────────────────────────────────
        Expr::TemplateLit { parts, .. } => {
            let mut out = String::from("`");
            for part in parts {
                match part {
                    TemplatePart::Text(text) => {
                        // Escape backticks and dollar-braces for JS template literals
                        out.push_str(&text.replace('`', "\\`").replace("${", "\\${"));
                    }
                    TemplatePart::Expr(e) => {
                        out.push_str("${");
                        out.push_str(&expr_to_js(e, signal_names));
                        out.push('}');
                    }
                }
            }
            out.push('`');
            out
        }

        // ── Binary operations ───────────────────────────────────
        Expr::BinaryOp {
            left, op, right, ..
        } => {
            let l = expr_to_js(left, signal_names);
            let r = expr_to_js(right, signal_names);
            let op_str = match op {
                BinOp::Add => "+",
                BinOp::Sub => "-",
                BinOp::Mul => "*",
                BinOp::Div => "/",
                BinOp::Mod => "%",
                BinOp::Eq => "===", // GaleX == maps to JS strict equality
                BinOp::NotEq => "!==",
                BinOp::Lt => "<",
                BinOp::Gt => ">",
                BinOp::LtEq => "<=",
                BinOp::GtEq => ">=",
                BinOp::And => "&&",
                BinOp::Or => "||",
                BinOp::DotDot => "/* .. */", // ranges don't exist in JS
            };
            format!("({l} {op_str} {r})")
        }

        // ── Unary operations ────────────────────────────────────
        Expr::UnaryOp { op, operand, .. } => {
            let o = expr_to_js(operand, signal_names);
            match op {
                UnaryOp::Neg => format!("(-{o})"),
                UnaryOp::Not => format!("(!{o})"),
            }
        }

        // ── Ternary ─────────────────────────────────────────────
        Expr::Ternary {
            condition,
            then_expr,
            else_expr,
            ..
        } => {
            let c = expr_to_js(condition, signal_names);
            let t = expr_to_js(then_expr, signal_names);
            let e = expr_to_js(else_expr, signal_names);
            format!("({c} ? {t} : {e})")
        }

        // ── Null coalescing ─────────────────────────────────────
        Expr::NullCoalesce { left, right, .. } => {
            let l = expr_to_js(left, signal_names);
            let r = expr_to_js(right, signal_names);
            format!("({l} ?? {r})")
        }

        // ── Function call ───────────────────────────────────────
        Expr::FnCall { callee, args, .. } => {
            let c = expr_to_js(callee, signal_names);
            let a: Vec<String> = args.iter().map(|a| expr_to_js(a, signal_names)).collect();
            format!("{c}({})", a.join(", "))
        }

        // ── Member access ───────────────────────────────────────
        Expr::MemberAccess { object, field, .. } => {
            let o = expr_to_js(object, signal_names);
            format!("{o}.{field}")
        }

        // ── Optional chaining ───────────────────────────────────
        Expr::OptionalChain { object, field, .. } => {
            let o = expr_to_js(object, signal_names);
            format!("{o}?.{field}")
        }

        // ── Index access ────────────────────────────────────────
        Expr::IndexAccess { object, index, .. } => {
            let o = expr_to_js(object, signal_names);
            let i = expr_to_js(index, signal_names);
            format!("{o}[{i}]")
        }

        // ── Array literal ───────────────────────────────────────
        Expr::ArrayLit { elements, .. } => {
            let elems: Vec<String> = elements
                .iter()
                .map(|e| expr_to_js(e, signal_names))
                .collect();
            format!("[{}]", elems.join(", "))
        }

        // ── Object literal ──────────────────────────────────────
        Expr::ObjectLit { fields, .. } => {
            let pairs: Vec<String> = fields
                .iter()
                .map(|f| {
                    let v = expr_to_js(&f.value, signal_names);
                    format!("{}: {v}", f.key)
                })
                .collect();
            format!("{{ {} }}", pairs.join(", "))
        }

        // ── Arrow function ──────────────────────────────────────
        Expr::ArrowFn { params, body, .. } => {
            let p: Vec<&str> = params.iter().map(|p| p.name.as_str()).collect();
            let b = match body {
                ArrowBody::Expr(e) => expr_to_js(e, signal_names),
                ArrowBody::Block(block) => {
                    let stmts: Vec<String> = block
                        .stmts
                        .iter()
                        .map(|s| stmt_to_js_brief(s, signal_names))
                        .collect();
                    format!("{{ {} }}", stmts.join(" "))
                }
            };
            if p.len() == 1 {
                format!("{} => {b}", p[0])
            } else {
                format!("({}) => {b}", p.join(", "))
            }
        }

        // ── Spread ──────────────────────────────────────────────
        Expr::Spread { expr, .. } => {
            let e = expr_to_js(expr, signal_names);
            format!("...{e}")
        }

        // ── Range ───────────────────────────────────────────────
        Expr::Range { start, end, .. } => {
            let s = expr_to_js(start, signal_names);
            let e = expr_to_js(end, signal_names);
            format!("/* {s}..{e} */[]")
        }

        // ── Pipe ────────────────────────────────────────────────
        Expr::Pipe { left, right, .. } => {
            let l = expr_to_js(left, signal_names);
            let r = expr_to_js(right, signal_names);
            format!("{r}({l})")
        }

        // ── Await ───────────────────────────────────────────────
        Expr::Await { expr, .. } => {
            let e = expr_to_js(expr, signal_names);
            format!("await {e}")
        }

        // ── Assign ──────────────────────────────────────────────
        Expr::Assign {
            target, op, value, ..
        } => {
            let t = expr_to_js(target, signal_names);
            let v = expr_to_js(value, signal_names);
            // Check if target is a signal — use .set() instead of =
            if let Expr::Ident { name, .. } = target.as_ref() {
                if signal_names.contains(name.as_str()) {
                    return match op {
                        AssignOp::Assign => format!("{name}.set({v})"),
                        AssignOp::AddAssign => {
                            format!("{name}.set({name}.get() + {v})")
                        }
                        AssignOp::SubAssign => {
                            format!("{name}.set({name}.get() - {v})")
                        }
                    };
                }
            }
            let op_str = match op {
                AssignOp::Assign => "=",
                AssignOp::AddAssign => "+=",
                AssignOp::SubAssign => "-=",
            };
            format!("{t} {op_str} {v}")
        }

        // ── Assert ──────────────────────────────────────────────
        Expr::Assert { expr, .. } => {
            let e = expr_to_js(expr, signal_names);
            format!("console.assert({e})")
        }

        // ── env.KEY ─────────────────────────────────────────────
        Expr::EnvAccess { key, .. } => {
            format!("$env[{:?}]", key.as_str())
        }
    }
}

/// Brief JS statement emission for arrow function block bodies.
fn stmt_to_js_brief(stmt: &Stmt, signal_names: &HashSet<String>) -> String {
    match stmt {
        Stmt::Return { value, .. } => match value {
            Some(e) => format!("return {};", expr_to_js(e, signal_names)),
            None => "return;".into(),
        },
        Stmt::ExprStmt { expr, .. } => format!("{};", expr_to_js(expr, signal_names)),
        Stmt::Let { name, init, .. } => {
            format!("const {} = {};", name, expr_to_js(init, signal_names))
        }
        _ => "/* ... */".into(),
    }
}
