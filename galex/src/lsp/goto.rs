//! Go-to-definition, find references, and rename.

use std::collections::HashSet;

use lsp_types::{Location, Position, Range, TextEdit, Url, WorkspaceEdit};

use super::document::DocumentManager;
use super::position::{self, NodeInfo};
use crate::ast::*;
use crate::span::Span;
use smol_str::SmolStr;

/// Go to the definition of the symbol at the given offset.
pub fn goto_definition(
    docs: &DocumentManager,
    uri: &Url,
    file_id: u32,
    offset: u32,
) -> Option<Location> {
    let program = docs.merged_program()?;
    let node = position::node_at_offset(program, file_id, offset)?;
    let checker = docs.cached_checker.as_ref()?;

    let name = match &node {
        NodeInfo::Ident { name, .. } => name.clone(),
        _ => return None,
    };

    // Look up the binding to get its declaration span
    let binding = checker.env.lookup(&name)?;
    let source = docs.get_source(uri)?;
    let def_span = binding.span;

    // Find the file URI for this span
    let def_path = docs.file_table.get_path(def_span.file_id)?;
    let def_uri = Url::from_file_path(def_path).ok()?;

    // We need the source of the definition file for range computation
    // For same-file definitions, use the current source
    let range = if def_span.file_id == file_id {
        span_to_lsp_range(&def_span, source)
    } else {
        // Cross-file — use a single-point range at start
        let start = Position {
            line: def_span.line.saturating_sub(1),
            character: def_span.col.saturating_sub(1),
        };
        Range { start, end: start }
    };

    Some(Location {
        uri: def_uri,
        range,
    })
}

/// Find all references to the symbol at the given offset.
pub fn find_references(docs: &DocumentManager, file_id: u32, offset: u32) -> Vec<Location> {
    let program = match docs.merged_program() {
        Some(p) => p,
        None => return vec![],
    };
    let node = match position::node_at_offset(program, file_id, offset) {
        Some(n) => n,
        None => return vec![],
    };

    let target_name = match &node {
        NodeInfo::Ident { name, .. } => name.clone(),
        NodeInfo::Decl { name, .. } => name.clone(),
        _ => return vec![],
    };

    // Walk the entire AST and collect all locations where this name appears
    let mut locations = Vec::new();
    collect_references(&program.items, &target_name, docs, &mut locations);
    locations
}

/// Rename the symbol at the given offset.
pub fn rename_symbol(
    docs: &DocumentManager,
    file_id: u32,
    offset: u32,
    new_name: &str,
) -> Option<WorkspaceEdit> {
    let refs = find_references(docs, file_id, offset);
    if refs.is_empty() {
        return None;
    }

    // Group edits by document URI
    let mut changes: std::collections::HashMap<Url, Vec<TextEdit>> =
        std::collections::HashMap::new();
    for location in refs {
        changes
            .entry(location.uri.clone())
            .or_default()
            .push(TextEdit {
                range: location.range,
                new_text: new_name.to_string(),
            });
    }

    Some(WorkspaceEdit {
        changes: Some(changes),
        ..Default::default()
    })
}

// ── Helpers ────────────────────────────────────────────────────────────

fn collect_references(
    items: &[Item],
    target: &SmolStr,
    docs: &DocumentManager,
    locations: &mut Vec<Location>,
) {
    for item in items {
        match item {
            Item::ComponentDecl(c) => {
                for stmt in &c.body.stmts {
                    collect_refs_in_stmt(stmt, target, docs, locations);
                }
                collect_refs_in_template(&c.body.template, target, docs, locations);
            }
            Item::FnDecl(f) => {
                collect_refs_in_block(&f.body, target, docs, locations);
            }
            Item::ActionDecl(a) => {
                collect_refs_in_block(&a.body, target, docs, locations);
            }
            Item::Out(out) => {
                collect_references(&[*out.inner.clone()], target, docs, locations);
            }
            Item::ServerBlock(b) | Item::ClientBlock(b) | Item::SharedBlock(b) => {
                collect_references(&b.items, target, docs, locations);
            }
            _ => {}
        }
    }
}

fn collect_refs_in_stmt(
    stmt: &Stmt,
    target: &SmolStr,
    docs: &DocumentManager,
    locations: &mut Vec<Location>,
) {
    match stmt {
        Stmt::Let { name, init, .. }
        | Stmt::Mut { name, init, .. }
        | Stmt::Signal { name, init, .. } => {
            if name == target {
                add_span_location(stmt_span(stmt), docs, locations);
            }
            collect_refs_in_expr(init, target, docs, locations);
        }
        Stmt::Derive { name, init, .. } | Stmt::Frozen { name, init, .. } => {
            if name == target {
                add_span_location(stmt_span(stmt), docs, locations);
            }
            collect_refs_in_expr(init, target, docs, locations);
        }
        Stmt::ExprStmt { expr, .. } => collect_refs_in_expr(expr, target, docs, locations),
        Stmt::If {
            condition,
            then_block,
            ..
        } => {
            collect_refs_in_expr(condition, target, docs, locations);
            collect_refs_in_block(then_block, target, docs, locations);
        }
        Stmt::Return { value: Some(e), .. } => collect_refs_in_expr(e, target, docs, locations),
        Stmt::For { iterable, body, .. } => {
            collect_refs_in_expr(iterable, target, docs, locations);
            collect_refs_in_block(body, target, docs, locations);
        }
        _ => {}
    }
}

fn collect_refs_in_expr(
    expr: &Expr,
    target: &SmolStr,
    docs: &DocumentManager,
    locations: &mut Vec<Location>,
) {
    match expr {
        Expr::Ident { name, span } if name == target => {
            add_span_location(*span, docs, locations);
        }
        Expr::BinaryOp { left, right, .. } => {
            collect_refs_in_expr(left, target, docs, locations);
            collect_refs_in_expr(right, target, docs, locations);
        }
        Expr::FnCall { callee, args, .. } => {
            collect_refs_in_expr(callee, target, docs, locations);
            for arg in args {
                collect_refs_in_expr(arg, target, docs, locations);
            }
        }
        Expr::MemberAccess { object, .. } => collect_refs_in_expr(object, target, docs, locations),
        Expr::Ternary {
            condition,
            then_expr,
            else_expr,
            ..
        } => {
            collect_refs_in_expr(condition, target, docs, locations);
            collect_refs_in_expr(then_expr, target, docs, locations);
            collect_refs_in_expr(else_expr, target, docs, locations);
        }
        _ => {}
    }
}

fn collect_refs_in_block(
    block: &Block,
    target: &SmolStr,
    docs: &DocumentManager,
    locations: &mut Vec<Location>,
) {
    for stmt in &block.stmts {
        collect_refs_in_stmt(stmt, target, docs, locations);
    }
}

fn collect_refs_in_template(
    nodes: &[TemplateNode],
    target: &SmolStr,
    docs: &DocumentManager,
    locations: &mut Vec<Location>,
) {
    for node in nodes {
        match node {
            TemplateNode::ExprInterp { expr, .. } => {
                collect_refs_in_expr(expr, target, docs, locations)
            }
            TemplateNode::Element {
                children,
                directives,
                attributes,
                ..
            } => {
                for attr in attributes {
                    if let AttrValue::Expr(e) = &attr.value {
                        collect_refs_in_expr(e, target, docs, locations);
                    }
                }
                for dir in directives {
                    if let Directive::On { handler, .. } = dir {
                        collect_refs_in_expr(handler, target, docs, locations);
                    }
                    if let Directive::Class { condition, .. } = dir {
                        collect_refs_in_expr(condition, target, docs, locations);
                    }
                }
                collect_refs_in_template(children, target, docs, locations);
            }
            TemplateNode::When {
                condition, body, ..
            } => {
                collect_refs_in_expr(condition, target, docs, locations);
                collect_refs_in_template(body, target, docs, locations);
            }
            TemplateNode::Each { iterable, body, .. } => {
                collect_refs_in_expr(iterable, target, docs, locations);
                collect_refs_in_template(body, target, docs, locations);
            }
            _ => {}
        }
    }
}

fn add_span_location(span: Span, docs: &DocumentManager, locations: &mut Vec<Location>) {
    if let Some(path) = docs.file_table.get_path(span.file_id) {
        if let Ok(uri) = Url::from_file_path(path) {
            let start = Position {
                line: span.line.saturating_sub(1),
                character: span.col.saturating_sub(1),
            };
            locations.push(Location {
                uri,
                range: Range { start, end: start },
            });
        }
    }
}

fn span_to_lsp_range(span: &Span, source: &str) -> Range {
    let start = Position {
        line: span.line.saturating_sub(1),
        character: span.col.saturating_sub(1),
    };
    let (end_line, end_col) = span.end_position(source);
    let end = Position {
        line: end_line.saturating_sub(1),
        character: end_col.saturating_sub(1),
    };
    Range { start, end }
}

fn stmt_span(stmt: &Stmt) -> Span {
    match stmt {
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
    }
}
