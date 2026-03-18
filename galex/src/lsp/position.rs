//! Position-to-AST-node lookup.
//!
//! Given a cursor position (byte offset), find the innermost AST node
//! at that location. Used for hover, go-to-definition, etc.

use crate::ast::*;
use crate::span::Span;
use smol_str::SmolStr;

/// Information about the AST node at a given position.
#[derive(Debug, Clone)]
pub enum NodeInfo {
    /// An identifier reference.
    Ident { name: SmolStr, span: Span },
    /// A type name reference.
    TypeRef { name: SmolStr, span: Span },
    /// A declaration name (function, guard, store, etc.).
    Decl {
        name: SmolStr,
        kind: DeclKind,
        span: Span,
    },
    /// An HTML element tag.
    HtmlTag { tag: SmolStr, span: Span },
    /// A directive.
    DirectiveRef { kind: String, span: Span },
    /// A generic expression (for hover type display).
    ExprNode { span: Span },
}

/// What kind of declaration a node represents.
#[derive(Debug, Clone, Copy)]
pub enum DeclKind {
    Function,
    Guard,
    Store,
    Action,
    Query,
    Channel,
    Component,
    Layout,
    Api,
    Middleware,
    TypeAlias,
    Enum,
}

/// Find the innermost AST node at the given byte offset.
pub fn node_at_offset(program: &Program, file_id: u32, offset: u32) -> Option<NodeInfo> {
    for item in &program.items {
        if let Some(info) = node_in_item(item, file_id, offset) {
            return Some(info);
        }
    }
    None
}

fn node_in_item(item: &Item, file_id: u32, offset: u32) -> Option<NodeInfo> {
    match item {
        Item::FnDecl(d) if d.span.file_id == file_id && d.span.contains_offset(offset) => {
            // Check if cursor is on the name
            if let Some(info) = check_ident_at(&d.name, &d.span, offset) {
                return Some(NodeInfo::Decl {
                    name: d.name.clone(),
                    kind: DeclKind::Function,
                    span: d.span,
                });
            }
            node_in_block(&d.body, file_id, offset)
        }
        Item::GuardDecl(d) if d.span.file_id == file_id && d.span.contains_offset(offset) => {
            Some(NodeInfo::Decl {
                name: d.name.clone(),
                kind: DeclKind::Guard,
                span: d.span,
            })
        }
        Item::StoreDecl(d) if d.span.file_id == file_id && d.span.contains_offset(offset) => {
            Some(NodeInfo::Decl {
                name: d.name.clone(),
                kind: DeclKind::Store,
                span: d.span,
            })
        }
        Item::ActionDecl(d) if d.span.file_id == file_id && d.span.contains_offset(offset) => {
            if let Some(info) = node_in_block(&d.body, file_id, offset) {
                return Some(info);
            }
            Some(NodeInfo::Decl {
                name: d.name.clone(),
                kind: DeclKind::Action,
                span: d.span,
            })
        }
        Item::ComponentDecl(d) if d.span.file_id == file_id && d.span.contains_offset(offset) => {
            for stmt in &d.body.stmts {
                if let Some(info) = node_in_stmt(stmt, file_id, offset) {
                    return Some(info);
                }
            }
            for node in &d.body.template {
                if let Some(info) = node_in_template(node, file_id, offset) {
                    return Some(info);
                }
            }
            Some(NodeInfo::Decl {
                name: d.name.clone(),
                kind: DeclKind::Component,
                span: d.span,
            })
        }
        Item::LayoutDecl(d) if d.span.file_id == file_id && d.span.contains_offset(offset) => {
            for stmt in &d.body.stmts {
                if let Some(info) = node_in_stmt(stmt, file_id, offset) {
                    return Some(info);
                }
            }
            for node in &d.body.template {
                if let Some(info) = node_in_template(node, file_id, offset) {
                    return Some(info);
                }
            }
            Some(NodeInfo::Decl {
                name: d.name.clone(),
                kind: DeclKind::Layout,
                span: d.span,
            })
        }
        Item::Out(out) => node_in_item(&out.inner, file_id, offset),
        Item::ServerBlock(b) | Item::ClientBlock(b) | Item::SharedBlock(b) => {
            for inner in &b.items {
                if let Some(info) = node_in_item(inner, file_id, offset) {
                    return Some(info);
                }
            }
            None
        }
        Item::Stmt(stmt) => node_in_stmt(stmt, file_id, offset),
        _ => None,
    }
}

fn node_in_stmt(stmt: &Stmt, file_id: u32, offset: u32) -> Option<NodeInfo> {
    match stmt {
        Stmt::Let { init, span, .. }
        | Stmt::Mut { init, span, .. }
        | Stmt::Signal { init, span, .. }
            if span.file_id == file_id && span.contains_offset(offset) =>
        {
            node_in_expr(init, file_id, offset)
        }
        Stmt::Derive { init, span, .. } | Stmt::Frozen { init, span, .. }
            if span.file_id == file_id && span.contains_offset(offset) =>
        {
            node_in_expr(init, file_id, offset)
        }
        Stmt::ExprStmt { expr, span }
            if span.file_id == file_id && span.contains_offset(offset) =>
        {
            node_in_expr(expr, file_id, offset)
        }
        Stmt::If {
            condition,
            then_block,
            span,
            ..
        } if span.file_id == file_id && span.contains_offset(offset) => {
            if let Some(info) = node_in_expr(condition, file_id, offset) {
                return Some(info);
            }
            node_in_block(then_block, file_id, offset)
        }
        Stmt::Return {
            value: Some(expr),
            span,
        } if span.file_id == file_id && span.contains_offset(offset) => {
            node_in_expr(expr, file_id, offset)
        }
        _ => None,
    }
}

fn node_in_expr(expr: &Expr, file_id: u32, offset: u32) -> Option<NodeInfo> {
    let span = expr.span();
    if span.file_id != file_id || !span.contains_offset(offset) {
        return None;
    }
    match expr {
        Expr::Ident { name, span } => Some(NodeInfo::Ident {
            name: name.clone(),
            span: *span,
        }),
        Expr::MemberAccess {
            object,
            field,
            span,
        } => {
            if let Some(info) = node_in_expr(object, file_id, offset) {
                return Some(info);
            }
            Some(NodeInfo::Ident {
                name: field.clone(),
                span: *span,
            })
        }
        Expr::FnCall { callee, args, .. } => {
            if let Some(info) = node_in_expr(callee, file_id, offset) {
                return Some(info);
            }
            for arg in args {
                if let Some(info) = node_in_expr(arg, file_id, offset) {
                    return Some(info);
                }
            }
            Some(NodeInfo::ExprNode { span })
        }
        Expr::BinaryOp { left, right, .. } => {
            if let Some(info) = node_in_expr(left, file_id, offset) {
                return Some(info);
            }
            if let Some(info) = node_in_expr(right, file_id, offset) {
                return Some(info);
            }
            Some(NodeInfo::ExprNode { span })
        }
        Expr::Ternary {
            condition,
            then_expr,
            else_expr,
            ..
        } => {
            if let Some(info) = node_in_expr(condition, file_id, offset) {
                return Some(info);
            }
            if let Some(info) = node_in_expr(then_expr, file_id, offset) {
                return Some(info);
            }
            if let Some(info) = node_in_expr(else_expr, file_id, offset) {
                return Some(info);
            }
            Some(NodeInfo::ExprNode { span })
        }
        Expr::ArrayLit { elements, .. } => {
            for el in elements {
                if let Some(info) = node_in_expr(el, file_id, offset) {
                    return Some(info);
                }
            }
            Some(NodeInfo::ExprNode { span })
        }
        _ => Some(NodeInfo::ExprNode { span }),
    }
}

fn node_in_block(block: &Block, file_id: u32, offset: u32) -> Option<NodeInfo> {
    for stmt in &block.stmts {
        if let Some(info) = node_in_stmt(stmt, file_id, offset) {
            return Some(info);
        }
    }
    None
}

fn node_in_template(node: &TemplateNode, file_id: u32, offset: u32) -> Option<NodeInfo> {
    match node {
        TemplateNode::Element {
            tag,
            children,
            span,
            ..
        } if span.file_id == file_id && span.contains_offset(offset) => {
            for child in children {
                if let Some(info) = node_in_template(child, file_id, offset) {
                    return Some(info);
                }
            }
            Some(NodeInfo::HtmlTag {
                tag: tag.clone(),
                span: *span,
            })
        }
        TemplateNode::ExprInterp { expr, span }
            if span.file_id == file_id && span.contains_offset(offset) =>
        {
            node_in_expr(expr, file_id, offset)
        }
        TemplateNode::When {
            condition,
            body,
            span,
            ..
        } if span.file_id == file_id && span.contains_offset(offset) => {
            if let Some(info) = node_in_expr(condition, file_id, offset) {
                return Some(info);
            }
            for child in body {
                if let Some(info) = node_in_template(child, file_id, offset) {
                    return Some(info);
                }
            }
            None
        }
        TemplateNode::Each {
            iterable,
            body,
            span,
            ..
        } if span.file_id == file_id && span.contains_offset(offset) => {
            if let Some(info) = node_in_expr(iterable, file_id, offset) {
                return Some(info);
            }
            for child in body {
                if let Some(info) = node_in_template(child, file_id, offset) {
                    return Some(info);
                }
            }
            None
        }
        _ => None,
    }
}

fn check_ident_at(_name: &SmolStr, _span: &Span, _offset: u32) -> Option<NodeInfo> {
    // Simple heuristic — if offset is near the start of the span, it's the name
    None // Refine later with precise name span tracking
}
