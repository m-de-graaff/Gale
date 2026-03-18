//! Individual lint rule implementations.

use std::collections::HashSet;

use super::{LintWarning, Severity};
use crate::ast::*;
use crate::errors::codes;

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
    for stmt in &comp.body.stmts {
        if let Stmt::Signal { name, span, .. } = stmt {
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
                code: &codes::GX1606,
                rule: "unused-signal",
                message: format!("signal `{name}` is declared but never read"),
                span: *span,
                severity: Severity::Warning,
            });
        }
    }
}

// ── unused-derive ──────────────────────────────────────────────────────

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
                code: &codes::GX1606,
                rule: "unused-derive",
                message: format!("derive `{name}` is declared but never read"),
                span: *span,
                severity: Severity::Warning,
            });
        }
    }
}

// ── empty-block ────────────────────────────────────────────────────────

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
                    code: &codes::GX1705,
                    rule: "empty-block",
                    message: "empty `when` block".into(),
                    span: *span,
                    severity: Severity::Warning,
                });
            }
            TemplateNode::Each { body, span, .. } if body.is_empty() => {
                warnings.push(LintWarning {
                    code: &codes::GX1705,
                    rule: "empty-block",
                    message: "empty `each` block".into(),
                    span: *span,
                    severity: Severity::Warning,
                });
            }
            TemplateNode::Element { children, .. } => check_empty_in_template(children, warnings),
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

// ── missing-key ────────────────────────────────────────────────────────

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
                let has_key = body.iter().any(|child| match child {
                    TemplateNode::Element { directives, .. }
                    | TemplateNode::SelfClosing { directives, .. } => directives
                        .iter()
                        .any(|d| matches!(d, Directive::Key { .. })),
                    _ => false,
                });
                if !has_key && !body.is_empty() {
                    warnings.push(LintWarning {
                        code: &codes::GX0705,
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
                check_missing_key_in_template(children, warnings)
            }
            TemplateNode::When { body, .. } => check_missing_key_in_template(body, warnings),
            _ => {}
        }
    }
}

// ── missing-alt ────────────────────────────────────────────────────────

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
                if !attributes.iter().any(|a| a.name == "alt") {
                    warnings.push(LintWarning {
                        code: &codes::GX1708,
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
                if !attributes.iter().any(|a| a.name == "alt") {
                    warnings.push(LintWarning {
                        code: &codes::GX1708,
                        rule: "missing-alt",
                        message: "<img> element should have an `alt` attribute for accessibility"
                            .into(),
                        span: *span,
                        severity: Severity::Warning,
                    });
                }
                check_alt_in_template(children, warnings);
            }
            TemplateNode::Element { children, .. } => check_alt_in_template(children, warnings),
            TemplateNode::When { body, .. } => check_alt_in_template(body, warnings),
            TemplateNode::Each { body, .. } => check_alt_in_template(body, warnings),
            _ => {}
        }
    }
}

// ── unreachable-code ───────────────────────────────────────────────────

pub fn check_unreachable_after_return(program: &Program, warnings: &mut Vec<LintWarning>) {
    for item in &program.items {
        match item {
            Item::FnDecl(f) => check_unreachable_in_block(&f.body, warnings),
            Item::ActionDecl(a) => check_unreachable_in_block(&a.body, warnings),
            Item::Out(out) => {
                if let Item::FnDecl(f) = out.inner.as_ref() {
                    check_unreachable_in_block(&f.body, warnings);
                }
            }
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
                code: &codes::GX0326,
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

// ── GX1700: unused-variable ────────────────────────────────────────────

pub fn check_unused_variables(program: &Program, warnings: &mut Vec<LintWarning>) {
    for item in &program.items {
        match item {
            Item::FnDecl(f) => check_unused_vars_in_block(&f.body, warnings),
            Item::ComponentDecl(c) => {
                check_unused_vars_in_stmts(&c.body.stmts, &c.body.template, warnings)
            }
            Item::Out(out) => match out.inner.as_ref() {
                Item::FnDecl(f) => check_unused_vars_in_block(&f.body, warnings),
                Item::ComponentDecl(c) => {
                    check_unused_vars_in_stmts(&c.body.stmts, &c.body.template, warnings)
                }
                _ => {}
            },
            _ => {}
        }
    }
}

fn check_unused_vars_in_block(block: &Block, warnings: &mut Vec<LintWarning>) {
    let mut declared = Vec::new();
    let mut referenced = HashSet::new();
    for stmt in &block.stmts {
        match stmt {
            Stmt::Let { name, span, .. }
            | Stmt::Mut { name, span, .. }
            | Stmt::Frozen { name, span, .. } => {
                if !name.starts_with('_') {
                    declared.push((name.clone(), *span));
                }
            }
            _ => {}
        }
        collect_idents_in_stmt(stmt, &mut referenced);
    }
    for (name, span) in &declared {
        if !referenced.contains(name.as_str()) {
            warnings.push(LintWarning {
                code: &codes::GX1700,
                rule: "unused-variable",
                message: format!("variable `{name}` is declared but never used"),
                span: *span,
                severity: Severity::Warning,
            });
        }
    }
}

fn check_unused_vars_in_stmts(
    stmts: &[Stmt],
    template: &[TemplateNode],
    warnings: &mut Vec<LintWarning>,
) {
    let mut declared = Vec::new();
    let mut referenced = HashSet::new();
    for stmt in stmts {
        match stmt {
            Stmt::Let { name, span, .. }
            | Stmt::Mut { name, span, .. }
            | Stmt::Frozen { name, span, .. } => {
                if !name.starts_with('_') {
                    declared.push((name.clone(), *span));
                }
            }
            _ => {}
        }
        collect_idents_in_stmt(stmt, &mut referenced);
    }
    collect_idents_in_template(template, &mut referenced);
    for (name, span) in &declared {
        if !referenced.contains(name.as_str()) {
            warnings.push(LintWarning {
                code: &codes::GX1700,
                rule: "unused-variable",
                message: format!("variable `{name}` is declared but never used"),
                span: *span,
                severity: Severity::Warning,
            });
        }
    }
}

// ── GX1704: console.log detection ──────────────────────────────────────

pub fn check_console_log(program: &Program, warnings: &mut Vec<LintWarning>) {
    for item in &program.items {
        match item {
            Item::FnDecl(f) => check_console_log_in_block(&f.body, warnings),
            Item::ActionDecl(a) => check_console_log_in_block(&a.body, warnings),
            Item::ComponentDecl(c) => {
                for stmt in &c.body.stmts {
                    check_console_log_in_stmt(stmt, warnings);
                }
            }
            Item::Out(out) => match out.inner.as_ref() {
                Item::FnDecl(f) => check_console_log_in_block(&f.body, warnings),
                Item::ComponentDecl(c) => {
                    for stmt in &c.body.stmts {
                        check_console_log_in_stmt(stmt, warnings);
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
}

fn check_console_log_in_block(block: &Block, warnings: &mut Vec<LintWarning>) {
    for stmt in &block.stmts {
        check_console_log_in_stmt(stmt, warnings);
    }
}

fn check_console_log_in_stmt(stmt: &Stmt, warnings: &mut Vec<LintWarning>) {
    if let Stmt::ExprStmt { expr, span } = stmt {
        if let Expr::FnCall { callee, .. } = expr {
            if let Expr::MemberAccess {
                object,
                field: member,
                ..
            } = callee.as_ref()
            {
                if let Expr::Ident { name, .. } = object.as_ref() {
                    if name.as_str() == "console"
                        && matches!(member.as_str(), "log" | "warn" | "error" | "info" | "debug")
                    {
                        warnings.push(LintWarning {
                            code: &codes::GX1704,
                            rule: "console-log",
                            message: "`console.log` should be removed from production code".into(),
                            span: *span,
                            severity: Severity::Warning,
                        });
                    }
                }
            }
        }
    }
    match stmt {
        Stmt::If {
            then_block,
            else_branch,
            ..
        } => {
            check_console_log_in_block(then_block, warnings);
            if let Some(ElseBranch::Else(block)) = else_branch {
                check_console_log_in_block(block, warnings);
            }
        }
        Stmt::For { body, .. } => check_console_log_in_block(body, warnings),
        Stmt::FnDecl(f) => check_console_log_in_block(&f.body, warnings),
        Stmt::Effect { body, .. } => check_console_log_in_block(body, warnings),
        _ => {}
    }
}

// ── GX1707: unnecessary else after return ──────────────────────────────

pub fn check_unnecessary_else_after_return(program: &Program, warnings: &mut Vec<LintWarning>) {
    for item in &program.items {
        match item {
            Item::FnDecl(f) => check_else_after_return(&f.body, warnings),
            Item::Out(out) => {
                if let Item::FnDecl(f) = out.inner.as_ref() {
                    check_else_after_return(&f.body, warnings);
                }
            }
            _ => {}
        }
    }
}

fn check_else_after_return(block: &Block, warnings: &mut Vec<LintWarning>) {
    for stmt in &block.stmts {
        if let Stmt::If {
            then_block,
            else_branch,
            span,
            ..
        } = stmt
        {
            if then_block
                .stmts
                .last()
                .is_some_and(|s| matches!(s, Stmt::Return { .. }))
                && else_branch.is_some()
            {
                warnings.push(LintWarning {
                    code: &codes::GX1707,
                    rule: "unnecessary-else",
                    message: "unnecessary `else` after `return`".into(),
                    span: *span,
                    severity: Severity::Warning,
                });
            }
            check_else_after_return(then_block, warnings);
            if let Some(ElseBranch::Else(block)) = else_branch {
                check_else_after_return(block, warnings);
            }
        }
        if let Stmt::For { body, .. } = stmt {
            check_else_after_return(body, warnings);
        }
        if let Stmt::FnDecl(f) = stmt {
            check_else_after_return(&f.body, warnings);
        }
    }
}

// ── GX1709: missing label for form input ───────────────────────────────

pub fn check_missing_label_for_input(program: &Program, warnings: &mut Vec<LintWarning>) {
    for item in &program.items {
        match item {
            Item::ComponentDecl(c) => check_labels(&c.body.template, warnings),
            Item::LayoutDecl(l) => check_labels(&l.body.template, warnings),
            Item::Out(out) => match out.inner.as_ref() {
                Item::ComponentDecl(c) => check_labels(&c.body.template, warnings),
                Item::LayoutDecl(l) => check_labels(&l.body.template, warnings),
                _ => {}
            },
            _ => {}
        }
    }
}

fn check_labels(nodes: &[TemplateNode], warnings: &mut Vec<LintWarning>) {
    let mut label_targets: HashSet<String> = HashSet::new();
    collect_label_fors(nodes, &mut label_targets);
    check_inputs(nodes, &label_targets, warnings);
}

fn collect_label_fors(nodes: &[TemplateNode], targets: &mut HashSet<String>) {
    for node in nodes {
        if let TemplateNode::Element {
            tag,
            attributes,
            children,
            ..
        } = node
        {
            if tag.as_str() == "label" {
                for attr in attributes {
                    if attr.name.as_str() == "for" {
                        if let AttrValue::String(val) = &attr.value {
                            targets.insert(val.to_string());
                        }
                    }
                }
            }
            collect_label_fors(children, targets);
        }
        if let TemplateNode::When { body, .. } = node {
            collect_label_fors(body, targets);
        }
        if let TemplateNode::Each { body, .. } = node {
            collect_label_fors(body, targets);
        }
    }
}

fn check_inputs(nodes: &[TemplateNode], labels: &HashSet<String>, warnings: &mut Vec<LintWarning>) {
    for node in nodes {
        if let TemplateNode::SelfClosing {
            tag,
            attributes,
            span,
            ..
        } = node
        {
            if tag.as_str() == "input" {
                let has_label = attributes.iter().any(|a| {
                    a.name.as_str() == "id"
                        && matches!(&a.value, AttrValue::String(v) if labels.contains(v.as_str()))
                });
                let has_aria = attributes.iter().any(|a| {
                    a.name.as_str() == "aria-label" || a.name.as_str() == "aria-labelledby"
                });
                let is_hidden = attributes.iter().any(|a| {
                    a.name.as_str() == "type"
                        && matches!(&a.value, AttrValue::String(v) if v.as_str() == "hidden")
                });
                if !has_label && !has_aria && !is_hidden {
                    warnings.push(LintWarning {
                        code: &codes::GX1709,
                        rule: "missing-label",
                        message: "<input> should have an associated <label> or `aria-label`".into(),
                        span: *span,
                        severity: Severity::Warning,
                    });
                }
            }
        }
        if let TemplateNode::Element { children, .. } = node {
            check_inputs(children, labels, warnings);
        }
        if let TemplateNode::When { body, .. } = node {
            check_inputs(body, labels, warnings);
        }
        if let TemplateNode::Each { body, .. } = node {
            check_inputs(body, labels, warnings);
        }
    }
}

// ── GX1712: function too long ──────────────────────────────────────────

pub fn check_function_too_long(program: &Program, warnings: &mut Vec<LintWarning>) {
    for item in &program.items {
        if let Item::FnDecl(f) = item {
            check_fn_len(f, warnings);
        }
        if let Item::Out(out) = item {
            if let Item::FnDecl(f) = out.inner.as_ref() {
                check_fn_len(f, warnings);
            }
        }
    }
}

fn check_fn_len(f: &FnDecl, warnings: &mut Vec<LintWarning>) {
    let lines = if f.body.span.line > 0 && f.span.line > 0 {
        f.body.span.line.saturating_sub(f.span.line)
    } else {
        f.body.span.end.saturating_sub(f.body.span.start) / 50
    };
    if lines > 50 {
        warnings.push(LintWarning {
            code: &codes::GX1712,
            rule: "function-too-long",
            message: format!(
                "function `{}` is ~{} lines — consider splitting",
                f.name, lines
            ),
            span: f.span,
            severity: Severity::Warning,
        });
    }
}

// ── GX1713: file too long ──────────────────────────────────────────────

pub fn check_file_too_long(program: &Program, warnings: &mut Vec<LintWarning>) {
    let last_line = program
        .items
        .last()
        .map(|i| match i {
            Item::FnDecl(f) => f.span.line,
            Item::ComponentDecl(c) => c.span.line,
            Item::GuardDecl(g) => g.span.line,
            Item::StoreDecl(s) => s.span.line,
            _ => 0,
        })
        .unwrap_or(0);
    if last_line > 300 {
        warnings.push(LintWarning {
            code: &codes::GX1713,
            rule: "file-too-long",
            message: format!("file has ~{} lines — consider splitting", last_line),
            span: program.span,
            severity: Severity::Warning,
        });
    }
}

// ── GX1717: TODO/FIXME detection ───────────────────────────────────────

pub fn check_todo_comments(program: &Program, warnings: &mut Vec<LintWarning>) {
    for item in &program.items {
        check_todo_item(item, warnings);
    }
}

fn check_todo_item(item: &Item, warnings: &mut Vec<LintWarning>) {
    match item {
        Item::FnDecl(f) => check_todo_block(&f.body, warnings),
        Item::ComponentDecl(c) => {
            for s in &c.body.stmts {
                check_todo_stmt(s, warnings);
            }
            check_todo_template(&c.body.template, warnings);
        }
        Item::Out(out) => check_todo_item(&out.inner, warnings),
        Item::ServerBlock(b) | Item::ClientBlock(b) | Item::SharedBlock(b) => {
            for sub in &b.items {
                check_todo_item(sub, warnings);
            }
        }
        Item::Stmt(s) => check_todo_stmt(s, warnings),
        _ => {}
    }
}

fn check_todo_block(block: &Block, warnings: &mut Vec<LintWarning>) {
    for s in &block.stmts {
        check_todo_stmt(s, warnings);
    }
}

fn check_todo_stmt(stmt: &Stmt, warnings: &mut Vec<LintWarning>) {
    match stmt {
        Stmt::Let { init, .. }
        | Stmt::Mut { init, .. }
        | Stmt::Signal { init, .. }
        | Stmt::Derive { init, .. }
        | Stmt::Frozen { init, .. } => check_todo_expr(init, warnings),
        Stmt::ExprStmt { expr, .. } => check_todo_expr(expr, warnings),
        Stmt::If {
            then_block,
            else_branch,
            ..
        } => {
            check_todo_block(then_block, warnings);
            if let Some(ElseBranch::Else(b)) = else_branch {
                check_todo_block(b, warnings);
            }
        }
        Stmt::For { body, .. } => check_todo_block(body, warnings),
        Stmt::FnDecl(f) => check_todo_block(&f.body, warnings),
        _ => {}
    }
}

fn check_todo_expr(expr: &Expr, warnings: &mut Vec<LintWarning>) {
    if let Expr::StringLit { value, span } = expr {
        let u = value.to_uppercase();
        if u.contains("TODO") || u.contains("FIXME") || u.contains("HACK") || u.contains("XXX") {
            let trunc = if value.len() > 40 {
                format!("{}...", &value[..40])
            } else {
                value.to_string()
            };
            warnings.push(LintWarning {
                code: &codes::GX1717,
                rule: "todo-comment",
                message: format!("TODO/FIXME found: \"{}\"", trunc),
                span: *span,
                severity: Severity::Warning,
            });
        }
    }
}

fn check_todo_template(nodes: &[TemplateNode], warnings: &mut Vec<LintWarning>) {
    for node in nodes {
        if let TemplateNode::Text { value, span } = node {
            let u = value.to_uppercase();
            if u.contains("TODO") || u.contains("FIXME") || u.contains("HACK") || u.contains("XXX")
            {
                let trunc = if value.len() > 40 {
                    format!("{}...", &value[..40])
                } else {
                    value.to_string()
                };
                warnings.push(LintWarning {
                    code: &codes::GX1717,
                    rule: "todo-comment",
                    message: format!("TODO/FIXME found: \"{}\"", trunc),
                    span: *span,
                    severity: Severity::Warning,
                });
            }
        }
        if let TemplateNode::Element { children, .. } = node {
            check_todo_template(children, warnings);
        }
        if let TemplateNode::When { body, .. } = node {
            check_todo_template(body, warnings);
        }
        if let TemplateNode::Each { body, .. } = node {
            check_todo_template(body, warnings);
        }
    }
}

// ── Helpers ────────────────────────────────────────────────────────────

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
            for a in args {
                collect_idents_in_expr(a, idents);
            }
        }
        Expr::MemberAccess { object, .. } | Expr::OptionalChain { object, .. } => {
            collect_idents_in_expr(object, idents)
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
            for e in elements {
                collect_idents_in_expr(e, idents);
            }
        }
        Expr::ObjectLit { fields, .. } => {
            for f in fields {
                collect_idents_in_expr(&f.value, idents);
            }
        }
        Expr::TemplateLit { parts, .. } => {
            for p in parts {
                if let TemplatePart::Expr(e) = p {
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
                for s in &b.stmts {
                    collect_idents_in_stmt(s, idents);
                }
            }
        },
        _ => {}
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
                for a in attributes {
                    if let AttrValue::Expr(e) = &a.value {
                        collect_idents_in_expr(e, idents);
                    }
                }
                for d in directives {
                    match d {
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
                for a in attributes {
                    if let AttrValue::Expr(e) = &a.value {
                        collect_idents_in_expr(e, idents);
                    }
                }
                for d in directives {
                    if let Directive::Class { condition, .. } = d {
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
                if let Some(WhenElse::Else(n)) = else_branch {
                    collect_idents_in_template(n, idents);
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
                if let Some(n) = empty {
                    collect_idents_in_template(n, idents);
                }
            }
            _ => {}
        }
    }
}
