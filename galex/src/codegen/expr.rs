//! GaleX expression → Rust expression string conversion.
//!
//! Converts AST [`Expr`] nodes into Rust source code strings for use
//! in generated SSR template rendering code. Only handles expression
//! forms that appear in templates, server data loading, and attributes.

use crate::ast::*;
use crate::codegen::types::to_snake_case;

/// Convert a GaleX expression to a Rust expression string.
///
/// The result is a valid Rust expression that can be used in generated
/// code (e.g., inside a `format!()` or as a function argument).
pub fn expr_to_rust(expr: &Expr) -> String {
    match expr {
        // ── Literals ────────────────────────────────────────────
        Expr::IntLit { value, .. } => format!("{value}_i64"),
        Expr::FloatLit { value, .. } => format!("{value}_f64"),
        Expr::StringLit { value, .. } => format!("String::from({:?})", value.as_str()),
        Expr::BoolLit { value, .. } => format!("{value}"),
        Expr::NullLit { .. } => "()".into(),
        Expr::RegexLit { pattern, flags, .. } => {
            format!("/* regex /{pattern}/{flags} */ String::new()")
        }

        // ── Identifier ──────────────────────────────────────────
        Expr::Ident { name, .. } => to_snake_case(name),

        // ── Template literal ────────────────────────────────────
        Expr::TemplateLit { parts, .. } => {
            if parts.is_empty() {
                return "String::new()".into();
            }
            let mut fmt_str = String::new();
            let mut fmt_args = Vec::new();
            for part in parts {
                match part {
                    TemplatePart::Text(text) => {
                        // Escape braces for format! string
                        fmt_str.push_str(&text.replace('{', "{{").replace('}', "}}"));
                    }
                    TemplatePart::Expr(e) => {
                        fmt_str.push_str("{}");
                        fmt_args.push(expr_to_rust(e));
                    }
                }
            }
            if fmt_args.is_empty() {
                format!("String::from({:?})", fmt_str)
            } else {
                format!("format!({:?}, {})", fmt_str, fmt_args.join(", "))
            }
        }

        // ── Binary operations ───────────────────────────────────
        Expr::BinaryOp {
            left, op, right, ..
        } => {
            let l = expr_to_rust(left);
            let r = expr_to_rust(right);
            let op_str = match op {
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
                BinOp::DotDot => "..",
            };
            format!("({l} {op_str} {r})")
        }

        // ── Unary operations ────────────────────────────────────
        Expr::UnaryOp { op, operand, .. } => {
            let o = expr_to_rust(operand);
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
            let c = expr_to_rust(condition);
            let t = expr_to_rust(then_expr);
            let e = expr_to_rust(else_expr);
            format!("if {c} {{ {t} }} else {{ {e} }}")
        }

        // ── Null coalescing ─────────────────────────────────────
        Expr::NullCoalesce { left, right, .. } => {
            let l = expr_to_rust(left);
            let r = expr_to_rust(right);
            format!("{l}.unwrap_or_else(|| {r})")
        }

        // ── Function call ───────────────────────────────────────
        Expr::FnCall { callee, args, .. } => {
            let c = expr_to_rust(callee);
            let a: Vec<String> = args.iter().map(expr_to_rust).collect();
            format!("{c}({})", a.join(", "))
        }

        // ── Member access ───────────────────────────────────────
        Expr::MemberAccess { object, field, .. } => {
            let o = expr_to_rust(object);
            let f = to_snake_case(field);
            format!("{o}.{f}")
        }

        // ── Optional chaining ───────────────────────────────────
        Expr::OptionalChain { object, field, .. } => {
            let o = expr_to_rust(object);
            let f = to_snake_case(field);
            format!("{o}.as_ref().map(|v| &v.{f})")
        }

        // ── Index access ────────────────────────────────────────
        Expr::IndexAccess { object, index, .. } => {
            let o = expr_to_rust(object);
            let i = expr_to_rust(index);
            format!("{o}[{i} as usize]")
        }

        // ── Array literal ───────────────────────────────────────
        Expr::ArrayLit { elements, .. } => {
            let elems: Vec<String> = elements.iter().map(expr_to_rust).collect();
            format!("vec![{}]", elems.join(", "))
        }

        // ── Object literal ──────────────────────────────────────
        Expr::ObjectLit { fields, .. } => {
            let pairs: Vec<String> = fields
                .iter()
                .map(|f| format!("{:?}: {}", f.key.as_str(), expr_to_rust(&f.value)))
                .collect();
            format!("serde_json::json!({{ {} }})", pairs.join(", "))
        }

        // ── Arrow function ──────────────────────────────────────
        Expr::ArrowFn { params, body, .. } => {
            let p: Vec<String> = params.iter().map(|p| to_snake_case(&p.name)).collect();
            let b = match body {
                ArrowBody::Expr(e) => expr_to_rust(e),
                ArrowBody::Block(_) => "{ /* block */ }".into(),
            };
            format!("|{}| {b}", p.join(", "))
        }

        // ── Spread ──────────────────────────────────────────────
        Expr::Spread { expr, .. } => expr_to_rust(expr),

        // ── Range ───────────────────────────────────────────────
        Expr::Range { start, end, .. } => {
            let s = expr_to_rust(start);
            let e = expr_to_rust(end);
            format!("({s}..{e})")
        }

        // ── Pipe ────────────────────────────────────────────────
        Expr::Pipe { left, right, .. } => {
            let l = expr_to_rust(left);
            let r = expr_to_rust(right);
            format!("{r}({l})")
        }

        // ── Await ───────────────────────────────────────────────
        Expr::Await { expr, .. } => {
            let e = expr_to_rust(expr);
            format!("{e}.await")
        }

        // ── Assign ──────────────────────────────────────────────
        Expr::Assign {
            target, op, value, ..
        } => {
            let t = expr_to_rust(target);
            let v = expr_to_rust(value);
            let op_str = match op {
                AssignOp::Assign => "=",
                AssignOp::AddAssign => "+=",
                AssignOp::SubAssign => "-=",
            };
            format!("{t} {op_str} {v}")
        }

        // ── Assert ──────────────────────────────────────────────
        Expr::Assert { expr, .. } => {
            let e = expr_to_rust(expr);
            format!("assert!({e})")
        }

        // ── env.KEY ─────────────────────────────────────────────
        Expr::EnvAccess { key, .. } => {
            ENV_KEYS.with(|keys| crate::codegen::emit_env::env_access_expr(key, &keys.borrow()))
        }
    }
}

use std::cell::RefCell;
use std::collections::HashSet;

thread_local! {
    /// Declared env keys — populated by `CodegenContext` before template emission.
    static ENV_KEYS: RefCell<HashSet<String>> = RefCell::new(HashSet::new());
}

/// Set the declared env keys for expression codegen.
///
/// Called by `CodegenContext` after scanning items so that `expr_to_rust()`
/// can emit typed `crate::env_config::ENV.*` accessors for declared keys.
pub fn set_declared_env_keys(keys: &HashSet<String>) {
    ENV_KEYS.with(|k| {
        *k.borrow_mut() = keys.clone();
    });
}

/// Get the current declared env keys (for use by other emitters).
pub fn get_declared_env_keys() -> HashSet<String> {
    ENV_KEYS.with(|k| k.borrow().clone())
}

/// Convert an expression to a Rust string suitable for display in HTML.
///
/// Wraps the expression in a `.to_string()` call for types that aren't
/// already `String`.
pub fn expr_to_display_string(expr: &Expr) -> String {
    match expr {
        Expr::StringLit { value, .. } => format!("{:?}", value.as_str()),
        Expr::TemplateLit { .. } => {
            // Template literals already produce String
            expr_to_rust(expr)
        }
        _ => {
            let e = expr_to_rust(expr);
            format!("{e}.to_string()")
        }
    }
}
