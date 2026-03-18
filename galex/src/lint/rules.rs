//! Individual lint rule implementations.

use std::collections::HashSet;

use super::{LintWarning, Severity};
use crate::ast::*;
use crate::span::Span;

// ── unused-signal: Signal declared but never read ──────────────────────

pub fn check_unused_signals(program: &Program, warnings: &mut Vec<LintWarning>) {
    for item in &program.items {
        if let Item::ComponentDecl(comp) = item {
            check_unused_signals_in_component(comp, warnings);
        }
        if let Item::Out(out) = item {
            if let Item::ComponentDecl(comp) = out.inner.as_ref() {
                check_unused_signals_in_component(comp, warnings);
            }
        }
    }
}

fn check_unused_signals_in_component(comp: &ComponentDecl, warnings: &mut Vec<LintWarning>) {
    let mut declared = Vec::new();
    let mut referenced = HashSet::new();

    // Collect signal declarations
    for stmt in &comp.body.stmts {
        if let Stmt::Signal { name, span, .. } = stmt {
            declared.push((name.clone(), *span));
        }
    }
    if declared.is_empty() {
        return;
    }

    // Collect all identifier references in stmts + template
    for stmt in &comp.body.stmts {
        collect_idents_in_stmt(stmt, &mut referenced);
    }
    collect_idents_in_template(&comp.body.template, &mut referenced);

    // Check for unused
    for (name, span) in &declared {
        if !referenced.contains(name.as_str()) {
            warnings.push(LintWarning {
                rule: "unused-signal",
                message: format!("signal `{name}` is declared but never read"),
                span: *span,
                severity: Severity::Warning,
            });
        }
    }
}

// ── unused-derive: Derive declared but never read ──────────────────────

pub fn check_unused_derives(program: &Program, warnings: &mut Vec<LintWarning>) {
    for item in &program.items {
        if let Item::ComponentDecl(comp) = item {
            check_unused_derives_in_component(comp, warnings);
        }
        if let Item::Out(out) = item {
            if let Item::ComponentDecl(comp) = out.inner.as_ref() {
                check_unused_derives_in_component(comp, warnings);
            }
        }
    }
}

fn check_unused_derives_in_component(comp: &ComponentDecl, warnings: &mut Vec<LintWarning>) {
    let mut declared = Vec::new();
    let mut referenced = HashSet::new();

    for stmt in &comp.body.stmts {
        if let Stmt::Derive { name, span, .. } = stmt {
            declared.push((name.clone(), *span));
        }
    }
    if declared.is_empty() {
        return;
    }

    for stmt in &comp.body.stmts {
        collect_idents_in_stmt(stmt, &mut referenced);
    }
    collect_idents_in_template(&comp.body.template, &mut referenced);

    for (name, span) in &declared {
        if !referenced.contains(name.as_str()) {
            warnings.push(LintWarning {
                rule: "unused-derive",
                message: format!("derive `{name}` is declared but never read"),
                span: *span,
                severity: Severity::Warning,
            });
        }
    }
}

// ── empty-block: Empty when/each template blocks ───────────────────────

pub fn check_empty_blocks(program: &Program, warnings: &mut Vec<LintWarning>) {
    for item in &program.items {
        match item {
            Item::ComponentDecl(c) => check_empty_in_template(&c.body.template, warnings),
            Item::LayoutDecl(l) => check_empty_in_template(&l.body.template, warnings),
            Item::Out(out) => match out.inner.as_ref() {
                Item::ComponentDecl(c) => check_empty_in_template(&c.body.template, warnings),
                Item::LayoutDecl(l) => check_empty_in_template(&l.body.template, warnings),
                _ => {}
            },
            _ => {}
        }
    }
}

fn check_empty_in_template(nodes: &[TemplateNode], warnings: &mut Vec<LintWarning>) {
    for node in nodes {
        match node {
            TemplateNode::When { body, span, .. } if body.is_empty() => {
                warnings.push(LintWarning {
                    rule: "empty-block",
                    message: "empty `when` block".into(),
                    span: *span,
                    severity: Severity::Warning,
                });
            }
            TemplateNode::Each { body, span, .. } if body.is_empty() => {
                warnings.push(LintWarning {
                    rule: "empty-block",
                    message: "empty `each` block".into(),
                    span: *span,
                    severity: Severity::Warning,
                });
            }
            TemplateNode::Element { children, .. } => {
                check_empty_in_template(children, warnings);
            }
            TemplateNode::When {
                body, else_branch, ..
            } => {
                check_empty_in_template(body, warnings);
                if let Some(WhenElse::Else(nodes)) = else_branch {
                    check_empty_in_template(nodes, warnings);
                }
            }
            TemplateNode::Each { body, empty, .. } => {
                check_empty_in_template(body, warnings);
                if let Some(nodes) = empty {
                    check_empty_in_template(nodes, warnings);
                }
            }
            _ => {}
        }
    }
}

// ── missing-key: each without key directive ────────────────────────────

pub fn check_missing_key_on_each(program: &Program, warnings: &mut Vec<LintWarning>) {
    for item in &program.items {
        match item {
            Item::ComponentDecl(c) => check_missing_key_in_template(&c.body.template, warnings),
            Item::Out(out) => {
                if let Item::ComponentDecl(c) = out.inner.as_ref() {
                    check_missing_key_in_template(&c.body.template, warnings);
                }
            }
            _ => {}
        }
    }
}

fn check_missing_key_in_template(nodes: &[TemplateNode], warnings: &mut Vec<LintWarning>) {
    for node in nodes {
        match node {
            TemplateNode::Each { body, span, .. } => {
                // Check if the first element child has a key directive
                let has_key = body.iter().any(|child| match child {
                    TemplateNode::Element { directives, .. }
                    | TemplateNode::SelfClosing { directives, .. } => directives
                        .iter()
                        .any(|d| matches!(d, Directive::Key { .. })),
                    _ => false,
                });
                if !has_key && !body.is_empty() {
                    warnings.push(LintWarning {
                        rule: "missing-key",
                        message:
                            "each block items should have a `key` directive for efficient updates"
                                .into(),
                        span: *span,
                        severity: Severity::Warning,
                    });
                }
                check_missing_key_in_template(body, warnings);
            }
            TemplateNode::Element { children, .. } => {
                check_missing_key_in_template(children, warnings);
            }
            TemplateNode::When { body, .. } => {
                check_missing_key_in_template(body, warnings);
            }
            _ => {}
        }
    }
}

// ── missing-alt: <img> without alt attribute ───────────────────────────

pub fn check_missing_alt_on_img(program: &Program, warnings: &mut Vec<LintWarning>) {
    for item in &program.items {
        match item {
            Item::ComponentDecl(c) => check_alt_in_template(&c.body.template, warnings),
            Item::LayoutDecl(l) => check_alt_in_template(&l.body.template, warnings),
            Item::Out(out) => match out.inner.as_ref() {
                Item::ComponentDecl(c) => check_alt_in_template(&c.body.template, warnings),
                Item::LayoutDecl(l) => check_alt_in_template(&l.body.template, warnings),
                _ => {}
            },
            _ => {}
        }
    }
}

fn check_alt_in_template(nodes: &[TemplateNode], warnings: &mut Vec<LintWarning>) {
    for node in nodes {
        match node {
            TemplateNode::SelfClosing {
                tag,
                attributes,
                span,
                ..
            } if tag.as_str() == "img" => {
                let has_alt = attributes.iter().any(|a| a.name == "alt");
                if !has_alt {
                    warnings.push(LintWarning {
                        rule: "missing-alt",
                        message: "<img> element should have an `alt` attribute for accessibility"
                            .into(),
                        span: *span,
                        severity: Severity::Warning,
                    });
                }
            }
            TemplateNode::Element {
                tag,
                attributes,
                children,
                span,
                ..
            } if tag.as_str() == "img" => {
                let has_alt = attributes.iter().any(|a| a.name == "alt");
                if !has_alt {
                    warnings.push(LintWarning {
                        rule: "missing-alt",
                        message: "<img> element should have an `alt` attribute for accessibility"
                            .into(),
                        span: *span,
                        severity: Severity::Warning,
                    });
                }
                check_alt_in_template(children, warnings);
            }
            TemplateNode::Element { children, .. } => {
                check_alt_in_template(children, warnings);
            }
            TemplateNode::When { body, .. } => check_alt_in_template(body, warnings),
            TemplateNode::Each { body, .. } => check_alt_in_template(body, warnings),
            _ => {}
        }
    }
}

// ── unreachable-code: Statements after return ──────────────────────────

pub fn check_unreachable_after_return(program: &Program, warnings: &mut Vec<LintWarning>) {
    for item in &program.items {
        match item {
            Item::FnDecl(f) => check_unreachable_in_block(&f.body, warnings),
            Item::ActionDecl(a) => check_unreachable_in_block(&a.body, warnings),
            Item::Out(out) => match out.inner.as_ref() {
                Item::FnDecl(f) => check_unreachable_in_block(&f.body, warnings),
                _ => {}
            },
            _ => {}
        }
    }
}

fn check_unreachable_in_block(block: &Block, warnings: &mut Vec<LintWarning>) {
    let mut found_return = false;
    for stmt in &block.stmts {
        if found_return {
            let span = match stmt {
                Stmt::Let { span, .. }
                | Stmt::Mut { span, .. }
                | Stmt::Signal { span, .. }
                | Stmt::Derive { span, .. }
                | Stmt::Frozen { span, .. }
                | Stmt::RefDecl { span, .. }
                | Stmt::If { span, .. }
                | Stmt::For { span, .. }
                | Stmt::Return { span, .. }
                | Stmt::Effect { span, .. }
                | Stmt::Watch { span, .. }
                | Stmt::ExprStmt { span, .. } => *span,
                Stmt::FnDecl(f) => f.span,
                Stmt::Block(b) => b.span,
            };
            warnings.push(LintWarning {
                rule: "unreachable-code",
                message: "unreachable code after return statement".into(),
                span,
                severity: Severity::Warning,
            });
            break;
        }
        if matches!(stmt, Stmt::Return { .. }) {
            found_return = true;
        }
        // Recurse into nested blocks
        match stmt {
            Stmt::If {
                then_block,
                else_branch,
                ..
            } => {
                check_unreachable_in_block(then_block, warnings);
                if let Some(ElseBranch::Else(block)) = else_branch {
                    check_unreachable_in_block(block, warnings);
                }
            }
            Stmt::For { body, .. } => check_unreachable_in_block(body, warnings),
            Stmt::FnDecl(f) => check_unreachable_in_block(&f.body, warnings),
            _ => {}
        }
    }
}

// ── Helpers ────────────────────────────────────────────────────────────

/// Collect all identifier references from an expression tree.
fn collect_idents_in_expr(expr: &Expr, idents: &mut HashSet<String>) {
    match expr {
        Expr::Ident { name, .. } => {
            idents.insert(name.to_string());
        }
        Expr::BinaryOp { left, right, .. } => {
            collect_idents_in_expr(left, idents);
            collect_idents_in_expr(right, idents);
        }
        Expr::UnaryOp { operand, .. } => collect_idents_in_expr(operand, idents),
        Expr::FnCall { callee, args, .. } => {
            collect_idents_in_expr(callee, idents);
            for arg in args {
                collect_idents_in_expr(arg, idents);
            }
        }
        Expr::MemberAccess { object, .. } | Expr::OptionalChain { object, .. } => {
            collect_idents_in_expr(object, idents);
        }
        Expr::IndexAccess { object, index, .. } => {
            collect_idents_in_expr(object, idents);
            collect_idents_in_expr(index, idents);
        }
        Expr::Ternary {
            condition,
            then_expr,
            else_expr,
            ..
        } => {
            collect_idents_in_expr(condition, idents);
            collect_idents_in_expr(then_expr, idents);
            collect_idents_in_expr(else_expr, idents);
        }
        Expr::Assign { target, value, .. } => {
            collect_idents_in_expr(target, idents);
            collect_idents_in_expr(value, idents);
        }
        Expr::ArrayLit { elements, .. } => {
            for el in elements {
                collect_idents_in_expr(el, idents);
            }
        }
        Expr::ObjectLit { fields, .. } => {
            for f in fields {
                collect_idents_in_expr(&f.value, idents);
            }
        }
        Expr::TemplateLit { parts, .. } => {
            for part in parts {
                if let TemplatePart::Expr(e) = part {
                    collect_idents_in_expr(e, idents);
                }
            }
        }
        Expr::Await { expr, .. } | Expr::Spread { expr, .. } | Expr::Assert { expr, .. } => {
            collect_idents_in_expr(expr, idents)
        }
        Expr::NullCoalesce { left, right, .. }
        | Expr::Pipe { left, right, .. }
        | Expr::Range {
            start: left,
            end: right,
            ..
        } => {
            collect_idents_in_expr(left, idents);
            collect_idents_in_expr(right, idents);
        }
        Expr::ArrowFn { body, .. } => match body {
            ArrowBody::Expr(e) => collect_idents_in_expr(e, idents),
            ArrowBody::Block(b) => {
                for stmt in &b.stmts {
                    collect_idents_in_stmt(stmt, idents);
                }
            }
        },
        _ => {} // literals, env access, etc. — no ident references
    }
}

fn collect_idents_in_stmt(stmt: &Stmt, idents: &mut HashSet<String>) {
    match stmt {
        Stmt::Let { init, .. }
        | Stmt::Mut { init, .. }
        | Stmt::Signal { init, .. }
        | Stmt::Derive { init, .. }
        | Stmt::Frozen { init, .. } => collect_idents_in_expr(init, idents),
        Stmt::ExprStmt { expr, .. } => collect_idents_in_expr(expr, idents),
        Stmt::Return { value: Some(e), .. } => collect_idents_in_expr(e, idents),
        Stmt::If {
            condition,
            then_block,
            else_branch,
            ..
        } => {
            collect_idents_in_expr(condition, idents);
            for s in &then_block.stmts {
                collect_idents_in_stmt(s, idents);
            }
            if let Some(ElseBranch::Else(block)) = else_branch {
                for s in &block.stmts {
                    collect_idents_in_stmt(s, idents);
                }
            }
        }
        Stmt::For { iterable, body, .. } => {
            collect_idents_in_expr(iterable, idents);
            for s in &body.stmts {
                collect_idents_in_stmt(s, idents);
            }
        }
        Stmt::Watch { target, body, .. } => {
            collect_idents_in_expr(target, idents);
            for s in &body.stmts {
                collect_idents_in_stmt(s, idents);
            }
        }
        Stmt::Effect { body, .. } => {
            for s in &body.stmts {
                collect_idents_in_stmt(s, idents);
            }
        }
        _ => {}
    }
}

fn collect_idents_in_template(nodes: &[TemplateNode], idents: &mut HashSet<String>) {
    for node in nodes {
        match node {
            TemplateNode::ExprInterp { expr, .. } => collect_idents_in_expr(expr, idents),
            TemplateNode::Element {
                attributes,
                directives,
                children,
                ..
            } => {
                for attr in attributes {
                    if let AttrValue::Expr(e) = &attr.value {
                        collect_idents_in_expr(e, idents);
                    }
                }
                for dir in directives {
                    match dir {
                        Directive::On { handler, .. } => collect_idents_in_expr(handler, idents),
                        Directive::Class { condition, .. } => {
                            collect_idents_in_expr(condition, idents)
                        }
                        Directive::Key { expr, .. } => collect_idents_in_expr(expr, idents),
                        Directive::FormAction { action, .. } => {
                            collect_idents_in_expr(action, idents)
                        }
                        Directive::FormGuard { guard, .. } => collect_idents_in_expr(guard, idents),
                        _ => {}
                    }
                }
                collect_idents_in_template(children, idents);
            }
            TemplateNode::SelfClosing {
                attributes,
                directives,
                ..
            } => {
                for attr in attributes {
                    if let AttrValue::Expr(e) = &attr.value {
                        collect_idents_in_expr(e, idents);
                    }
                }
                for dir in directives {
                    if let Directive::Class { condition, .. } = dir {
                        collect_idents_in_expr(condition, idents);
                    }
                }
            }
            TemplateNode::When {
                condition,
                body,
                else_branch,
                ..
            } => {
                collect_idents_in_expr(condition, idents);
                collect_idents_in_template(body, idents);
                if let Some(WhenElse::Else(nodes)) = else_branch {
                    collect_idents_in_template(nodes, idents);
                }
            }
            TemplateNode::Each {
                iterable,
                body,
                empty,
                ..
            } => {
                collect_idents_in_expr(iterable, idents);
                collect_idents_in_template(body, idents);
                if let Some(nodes) = empty {
                    collect_idents_in_template(nodes, idents);
                }
            }
            _ => {}
        }
    }
}
