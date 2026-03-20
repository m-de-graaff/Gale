//! Position-to-AST-node lookup and LSP coordinate conversion.
//!
//! Given a cursor position (byte offset), find the innermost AST node
//! at that location. Used for hover, go-to-definition, etc.
//!
//! Also provides shared utilities for converting between LSP Positions
//! (0-based line/character) and byte offsets.

use lsp_types::{Position, Range};

use crate::ast::*;
use crate::span::Span;
use smol_str::SmolStr;

// ── Coordinate conversion ──────────────────────────────────────────────

/// Convert an LSP Position (0-based line/character) to a byte offset.
///
/// Handles both LF (`\n`) and CRLF (`\r\n`) line endings correctly.
pub fn position_to_offset(source: &str, pos: Position) -> u32 {
    let mut line = 0u32;
    let mut byte_offset = 0usize;
    let bytes = source.as_bytes();

    // Advance to the target line
    while line < pos.line && byte_offset < bytes.len() {
        if bytes[byte_offset] == b'\n' {
            line += 1;
        }
        byte_offset += 1;
    }

    // Advance within the line by character count (skip \r)
    let mut col = 0u32;
    while col < pos.character && byte_offset < bytes.len() {
        match bytes[byte_offset] {
            b'\n' => break,
            b'\r' => byte_offset += 1,
            _ => {
                col += 1;
                byte_offset += 1;
            }
        }
    }

    byte_offset as u32
}

/// Convert a byte offset to an LSP Position (0-based line/character).
///
/// Handles both LF and CRLF line endings.
pub fn offset_to_position(source: &str, offset: u32) -> Position {
    let target = (offset as usize).min(source.len());
    let mut line = 0u32;
    let mut col = 0u32;

    for (i, b) in source.bytes().enumerate() {
        if i >= target {
            break;
        }
        if b == b'\n' {
            line += 1;
            col = 0;
        } else if b != b'\r' {
            col += 1;
        }
    }

    Position {
        line,
        character: col,
    }
}

/// Convert a [`Span`] (1-based line/col) to an LSP [`Range`] (0-based).
///
/// Uses the source text to compute the end position since the Span only
/// stores the start line/col.
pub fn span_to_lsp_range(span: &Span, source: &str) -> Range {
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

/// Extract the [`Span`] from a [`NodeInfo`].
pub fn node_info_span(node: &NodeInfo) -> Span {
    match node {
        NodeInfo::Ident { span, .. }
        | NodeInfo::TypeRef { span, .. }
        | NodeInfo::Decl { span, .. }
        | NodeInfo::HtmlTag { span, .. }
        | NodeInfo::DirectiveRef { span, .. }
        | NodeInfo::ExprNode { span } => *span,
    }
}

// ── AST node lookup ────────────────────────────────────────────────────

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
            if let Some(info) = node_in_block(&d.body, file_id, offset) {
                return Some(info);
            }
            Some(NodeInfo::Decl {
                name: d.name.clone(),
                kind: DeclKind::Function,
                span: d.span,
            })
        }
        Item::GuardDecl(d) if d.span.file_id == file_id && d.span.contains_offset(offset) => {
            // Check guard fields for references
            for field in &d.fields {
                if field.span.contains_offset(offset) {
                    // Check validator args
                    for v in &field.validators {
                        for arg in &v.args {
                            if let Some(info) = node_in_expr(arg, file_id, offset) {
                                return Some(info);
                            }
                        }
                    }
                    // Check type annotation
                    if let Some(info) = node_in_type_ann(&field.ty, file_id, offset) {
                        return Some(info);
                    }
                }
            }
            Some(NodeInfo::Decl {
                name: d.name.clone(),
                kind: DeclKind::Guard,
                span: d.span,
            })
        }
        Item::StoreDecl(d) if d.span.file_id == file_id && d.span.contains_offset(offset) => {
            for member in &d.members {
                match member {
                    StoreMember::Signal(stmt) | StoreMember::Derive(stmt) => {
                        if let Some(info) = node_in_stmt(stmt, file_id, offset) {
                            return Some(info);
                        }
                    }
                    StoreMember::Method(f) => {
                        if f.span.contains_offset(offset) {
                            if let Some(info) = node_in_block(&f.body, file_id, offset) {
                                return Some(info);
                            }
                            return Some(NodeInfo::Decl {
                                name: f.name.clone(),
                                kind: DeclKind::Function,
                                span: f.span,
                            });
                        }
                    }
                }
            }
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
        Item::QueryDecl(d) if d.span.file_id == file_id && d.span.contains_offset(offset) => {
            if let Some(info) = node_in_expr(&d.url_pattern, file_id, offset) {
                return Some(info);
            }
            Some(NodeInfo::Decl {
                name: d.name.clone(),
                kind: DeclKind::Query,
                span: d.span,
            })
        }
        Item::ChannelDecl(d) if d.span.file_id == file_id && d.span.contains_offset(offset) => {
            for handler in &d.handlers {
                if handler.span.contains_offset(offset) {
                    if let Some(info) = node_in_block(&handler.body, file_id, offset) {
                        return Some(info);
                    }
                }
            }
            Some(NodeInfo::Decl {
                name: d.name.clone(),
                kind: DeclKind::Channel,
                span: d.span,
            })
        }
        Item::TypeAlias(d) if d.span.file_id == file_id && d.span.contains_offset(offset) => {
            if let Some(info) = node_in_type_ann(&d.ty, file_id, offset) {
                return Some(info);
            }
            Some(NodeInfo::Decl {
                name: d.name.clone(),
                kind: DeclKind::TypeAlias,
                span: d.span,
            })
        }
        Item::EnumDecl(d) if d.span.file_id == file_id && d.span.contains_offset(offset) => {
            Some(NodeInfo::Decl {
                name: d.name.clone(),
                kind: DeclKind::Enum,
                span: d.span,
            })
        }
        Item::TestDecl(d) if d.span.file_id == file_id && d.span.contains_offset(offset) => {
            node_in_block(&d.body, file_id, offset)
        }
        Item::ComponentDecl(d) if d.span.file_id == file_id && d.span.contains_offset(offset) => {
            if let Some(info) = node_in_component_body(&d.body, file_id, offset) {
                return Some(info);
            }
            Some(NodeInfo::Decl {
                name: d.name.clone(),
                kind: DeclKind::Component,
                span: d.span,
            })
        }
        Item::LayoutDecl(d) if d.span.file_id == file_id && d.span.contains_offset(offset) => {
            if let Some(info) = node_in_component_body(&d.body, file_id, offset) {
                return Some(info);
            }
            Some(NodeInfo::Decl {
                name: d.name.clone(),
                kind: DeclKind::Layout,
                span: d.span,
            })
        }
        Item::ApiDecl(d) if d.span.file_id == file_id && d.span.contains_offset(offset) => {
            for handler in &d.handlers {
                if handler.span.contains_offset(offset) {
                    if let Some(info) = node_in_block(&handler.body, file_id, offset) {
                        return Some(info);
                    }
                }
            }
            Some(NodeInfo::Decl {
                name: d.name.clone(),
                kind: DeclKind::Api,
                span: d.span,
            })
        }
        Item::MiddlewareDecl(d) if d.span.file_id == file_id && d.span.contains_offset(offset) => {
            if let Some(info) = node_in_block(&d.body, file_id, offset) {
                return Some(info);
            }
            Some(NodeInfo::Decl {
                name: d.name.clone(),
                kind: DeclKind::Middleware,
                span: d.span,
            })
        }
        Item::EnvDecl(d) if d.span.file_id == file_id && d.span.contains_offset(offset) => {
            for var in &d.vars {
                if var.span.contains_offset(offset) {
                    if let Some(info) = node_in_type_ann(&var.ty, file_id, offset) {
                        return Some(info);
                    }
                }
            }
            None
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

fn node_in_component_body(body: &ComponentBody, file_id: u32, offset: u32) -> Option<NodeInfo> {
    for stmt in &body.stmts {
        if let Some(info) = node_in_stmt(stmt, file_id, offset) {
            return Some(info);
        }
    }
    if let Some(ref head) = body.head {
        if head.span.contains_offset(offset) {
            for field in &head.fields {
                if let Some(info) = node_in_expr(&field.value, file_id, offset) {
                    return Some(info);
                }
            }
        }
    }
    for node in &body.template {
        if let Some(info) = node_in_template(node, file_id, offset) {
            return Some(info);
        }
    }
    None
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
        Stmt::RefDecl { span, .. } if span.file_id == file_id && span.contains_offset(offset) => {
            None // ref declarations don't have sub-expressions to descend into
        }
        Stmt::FnDecl(f) if f.span.file_id == file_id && f.span.contains_offset(offset) => {
            if let Some(info) = node_in_block(&f.body, file_id, offset) {
                return Some(info);
            }
            Some(NodeInfo::Decl {
                name: f.name.clone(),
                kind: DeclKind::Function,
                span: f.span,
            })
        }
        Stmt::ExprStmt { expr, span }
            if span.file_id == file_id && span.contains_offset(offset) =>
        {
            node_in_expr(expr, file_id, offset)
        }
        Stmt::If {
            condition,
            then_block,
            else_branch,
            span,
        } if span.file_id == file_id && span.contains_offset(offset) => {
            if let Some(info) = node_in_expr(condition, file_id, offset) {
                return Some(info);
            }
            if let Some(info) = node_in_block(then_block, file_id, offset) {
                return Some(info);
            }
            if let Some(eb) = else_branch {
                match eb {
                    ElseBranch::Else(block) => {
                        if let Some(info) = node_in_block(block, file_id, offset) {
                            return Some(info);
                        }
                    }
                    ElseBranch::ElseIf(stmt) => {
                        if let Some(info) = node_in_stmt(stmt, file_id, offset) {
                            return Some(info);
                        }
                    }
                }
            }
            None
        }
        Stmt::For {
            iterable,
            body,
            span,
            ..
        } if span.file_id == file_id && span.contains_offset(offset) => {
            if let Some(info) = node_in_expr(iterable, file_id, offset) {
                return Some(info);
            }
            node_in_block(body, file_id, offset)
        }
        Stmt::Return {
            value: Some(expr),
            span,
        } if span.file_id == file_id && span.contains_offset(offset) => {
            node_in_expr(expr, file_id, offset)
        }
        Stmt::Effect { body, span, .. }
            if span.file_id == file_id && span.contains_offset(offset) =>
        {
            node_in_block(body, file_id, offset)
        }
        Stmt::Watch {
            target, body, span, ..
        } if span.file_id == file_id && span.contains_offset(offset) => {
            if let Some(info) = node_in_expr(target, file_id, offset) {
                return Some(info);
            }
            node_in_block(body, file_id, offset)
        }
        Stmt::Block(block)
            if block.span.file_id == file_id && block.span.contains_offset(offset) =>
        {
            node_in_block(block, file_id, offset)
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
        Expr::OptionalChain {
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
        Expr::BinaryOp { left, right, .. }
        | Expr::NullCoalesce { left, right, .. }
        | Expr::Pipe { left, right, .. }
        | Expr::Range {
            start: left,
            end: right,
            ..
        } => {
            if let Some(info) = node_in_expr(left, file_id, offset) {
                return Some(info);
            }
            if let Some(info) = node_in_expr(right, file_id, offset) {
                return Some(info);
            }
            Some(NodeInfo::ExprNode { span })
        }
        Expr::UnaryOp { operand, .. }
        | Expr::Await { expr: operand, .. }
        | Expr::Spread { expr: operand, .. }
        | Expr::Assert { expr: operand, .. } => {
            if let Some(info) = node_in_expr(operand, file_id, offset) {
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
        Expr::IndexAccess { object, index, .. } => {
            if let Some(info) = node_in_expr(object, file_id, offset) {
                return Some(info);
            }
            if let Some(info) = node_in_expr(index, file_id, offset) {
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
        Expr::ObjectLit { fields, .. } => {
            for field in fields {
                if let Some(info) = node_in_expr(&field.value, file_id, offset) {
                    return Some(info);
                }
            }
            Some(NodeInfo::ExprNode { span })
        }
        Expr::ArrowFn { body, .. } => {
            match body {
                ArrowBody::Expr(e) => {
                    if let Some(info) = node_in_expr(e, file_id, offset) {
                        return Some(info);
                    }
                }
                ArrowBody::Block(b) => {
                    if let Some(info) = node_in_block(b, file_id, offset) {
                        return Some(info);
                    }
                }
            }
            Some(NodeInfo::ExprNode { span })
        }
        Expr::Assign { target, value, .. } => {
            if let Some(info) = node_in_expr(target, file_id, offset) {
                return Some(info);
            }
            if let Some(info) = node_in_expr(value, file_id, offset) {
                return Some(info);
            }
            Some(NodeInfo::ExprNode { span })
        }
        Expr::TemplateLit { parts, .. } => {
            for part in parts {
                if let TemplatePart::Expr(e) = part {
                    if let Some(info) = node_in_expr(e, file_id, offset) {
                        return Some(info);
                    }
                }
            }
            Some(NodeInfo::ExprNode { span })
        }
        Expr::EnvAccess { key, span } => Some(NodeInfo::Ident {
            name: key.clone(),
            span: *span,
        }),
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
            directives,
            attributes,
            span,
        } if span.file_id == file_id && span.contains_offset(offset) => {
            // Check directives first (most specific)
            for dir in directives {
                if let Some(info) = node_in_directive(dir, file_id, offset) {
                    return Some(info);
                }
            }
            // Check attribute expressions
            for attr in attributes {
                if let AttrValue::Expr(e) = &attr.value {
                    if let Some(info) = node_in_expr(e, file_id, offset) {
                        return Some(info);
                    }
                }
            }
            // Check children
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
        TemplateNode::SelfClosing {
            tag,
            directives,
            attributes,
            span,
        } if span.file_id == file_id && span.contains_offset(offset) => {
            for dir in directives {
                if let Some(info) = node_in_directive(dir, file_id, offset) {
                    return Some(info);
                }
            }
            for attr in attributes {
                if let AttrValue::Expr(e) = &attr.value {
                    if let Some(info) = node_in_expr(e, file_id, offset) {
                        return Some(info);
                    }
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
            else_branch,
            span,
        } if span.file_id == file_id && span.contains_offset(offset) => {
            if let Some(info) = node_in_expr(condition, file_id, offset) {
                return Some(info);
            }
            for child in body {
                if let Some(info) = node_in_template(child, file_id, offset) {
                    return Some(info);
                }
            }
            if let Some(eb) = else_branch {
                match eb {
                    WhenElse::Else(nodes) => {
                        for child in nodes {
                            if let Some(info) = node_in_template(child, file_id, offset) {
                                return Some(info);
                            }
                        }
                    }
                    WhenElse::ElseWhen(node) => {
                        if let Some(info) = node_in_template(node, file_id, offset) {
                            return Some(info);
                        }
                    }
                }
            }
            None
        }
        TemplateNode::Each {
            iterable,
            body,
            empty,
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
            if let Some(empty_nodes) = empty {
                for child in empty_nodes {
                    if let Some(info) = node_in_template(child, file_id, offset) {
                        return Some(info);
                    }
                }
            }
            None
        }
        TemplateNode::Suspend { body, span, .. }
            if span.file_id == file_id && span.contains_offset(offset) =>
        {
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

fn node_in_directive(dir: &Directive, file_id: u32, offset: u32) -> Option<NodeInfo> {
    match dir {
        Directive::Bind { span, .. } if span.file_id == file_id && span.contains_offset(offset) => {
            Some(NodeInfo::DirectiveRef {
                kind: "bind".to_string(),
                span: *span,
            })
        }
        Directive::On { handler, span, .. }
            if span.file_id == file_id && span.contains_offset(offset) =>
        {
            if let Some(info) = node_in_expr(handler, file_id, offset) {
                return Some(info);
            }
            Some(NodeInfo::DirectiveRef {
                kind: "on".to_string(),
                span: *span,
            })
        }
        Directive::Class {
            condition, span, ..
        } if span.file_id == file_id && span.contains_offset(offset) => {
            if let Some(info) = node_in_expr(condition, file_id, offset) {
                return Some(info);
            }
            Some(NodeInfo::DirectiveRef {
                kind: "class".to_string(),
                span: *span,
            })
        }
        Directive::Ref { span, .. } if span.file_id == file_id && span.contains_offset(offset) => {
            Some(NodeInfo::DirectiveRef {
                kind: "ref".to_string(),
                span: *span,
            })
        }
        Directive::Key { expr, span }
            if span.file_id == file_id && span.contains_offset(offset) =>
        {
            if let Some(info) = node_in_expr(expr, file_id, offset) {
                return Some(info);
            }
            Some(NodeInfo::DirectiveRef {
                kind: "key".to_string(),
                span: *span,
            })
        }
        Directive::FormAction { action, span }
            if span.file_id == file_id && span.contains_offset(offset) =>
        {
            if let Some(info) = node_in_expr(action, file_id, offset) {
                return Some(info);
            }
            Some(NodeInfo::DirectiveRef {
                kind: "form:action".to_string(),
                span: *span,
            })
        }
        Directive::FormGuard { guard, span }
            if span.file_id == file_id && span.contains_offset(offset) =>
        {
            if let Some(info) = node_in_expr(guard, file_id, offset) {
                return Some(info);
            }
            Some(NodeInfo::DirectiveRef {
                kind: "form:guard".to_string(),
                span: *span,
            })
        }
        Directive::FormError { span, .. }
            if span.file_id == file_id && span.contains_offset(offset) =>
        {
            Some(NodeInfo::DirectiveRef {
                kind: "form:error".to_string(),
                span: *span,
            })
        }
        Directive::Transition { config, span, .. }
            if span.file_id == file_id && span.contains_offset(offset) =>
        {
            if let Some(e) = config {
                if let Some(info) = node_in_expr(e, file_id, offset) {
                    return Some(info);
                }
            }
            Some(NodeInfo::DirectiveRef {
                kind: "transition".to_string(),
                span: *span,
            })
        }
        _ => None,
    }
}

fn node_in_type_ann(ty: &TypeAnnotation, file_id: u32, offset: u32) -> Option<NodeInfo> {
    match ty {
        TypeAnnotation::Named { name, span }
            if span.file_id == file_id && span.contains_offset(offset) =>
        {
            Some(NodeInfo::TypeRef {
                name: name.clone(),
                span: *span,
            })
        }
        TypeAnnotation::Array { element, span }
            if span.file_id == file_id && span.contains_offset(offset) =>
        {
            node_in_type_ann(element, file_id, offset)
        }
        TypeAnnotation::Union { types, span }
            if span.file_id == file_id && span.contains_offset(offset) =>
        {
            for t in types {
                if let Some(info) = node_in_type_ann(t, file_id, offset) {
                    return Some(info);
                }
            }
            None
        }
        TypeAnnotation::Optional { inner, span }
            if span.file_id == file_id && span.contains_offset(offset) =>
        {
            node_in_type_ann(inner, file_id, offset)
        }
        TypeAnnotation::Function {
            params, ret, span, ..
        } if span.file_id == file_id && span.contains_offset(offset) => {
            for p in params {
                if let Some(info) = node_in_type_ann(p, file_id, offset) {
                    return Some(info);
                }
            }
            node_in_type_ann(ret, file_id, offset)
        }
        TypeAnnotation::Tuple { elements, span }
            if span.file_id == file_id && span.contains_offset(offset) =>
        {
            for e in elements {
                if let Some(info) = node_in_type_ann(e, file_id, offset) {
                    return Some(info);
                }
            }
            None
        }
        TypeAnnotation::Object { fields, span }
            if span.file_id == file_id && span.contains_offset(offset) =>
        {
            for f in fields {
                if let Some(info) = node_in_type_ann(&f.ty, file_id, offset) {
                    return Some(info);
                }
            }
            None
        }
        _ => None,
    }
}
