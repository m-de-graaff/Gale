//! Go-to-definition, find references, and rename.

use lsp_types::{Location, Position, Range, TextEdit, Url, WorkspaceEdit};

use super::document::DocumentManager;
use super::position::{self, span_to_lsp_range, NodeInfo};
use crate::ast::*;
use crate::span::Span;
use smol_str::SmolStr;

// ── Go-to-definition ───────────────────────────────────────────────────

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

    // Compute range — use full range for same-file, best-effort for cross-file
    let range = if def_span.file_id == file_id {
        span_to_lsp_range(&def_span, source)
    } else {
        match docs.get_source_by_file_id(def_span.file_id) {
            Some(other_source) => span_to_lsp_range(&def_span, other_source),
            None => {
                let start = Position {
                    line: def_span.line.saturating_sub(1),
                    character: def_span.col.saturating_sub(1),
                };
                Range { start, end: start }
            }
        }
    };

    Some(Location {
        uri: def_uri,
        range,
    })
}

// ── Find references ────────────────────────────────────────────────────

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

    let mut locations = Vec::new();
    collect_references(&program.items, &target_name, docs, &mut locations);
    locations
}

// ── Rename ─────────────────────────────────────────────────────────────

/// Rename the symbol at the given offset.
pub fn rename_symbol(
    docs: &DocumentManager,
    file_id: u32,
    offset: u32,
    new_name: &str,
) -> Option<WorkspaceEdit> {
    // Validate new name: must be a valid identifier
    if new_name.is_empty()
        || !new_name
            .chars()
            .next()
            .map_or(false, |c| c.is_alphabetic() || c == '_')
        || !new_name.chars().all(|c| c.is_alphanumeric() || c == '_')
    {
        return None;
    }

    // Reject renaming to keywords
    if is_keyword(new_name) {
        return None;
    }

    let refs = find_references(docs, file_id, offset);
    if refs.is_empty() {
        return None;
    }

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

fn is_keyword(name: &str) -> bool {
    matches!(
        name,
        "let"
            | "mut"
            | "signal"
            | "derive"
            | "frozen"
            | "ref"
            | "fn"
            | "return"
            | "if"
            | "else"
            | "for"
            | "await"
            | "guard"
            | "action"
            | "query"
            | "store"
            | "channel"
            | "effect"
            | "watch"
            | "when"
            | "each"
            | "suspend"
            | "server"
            | "client"
            | "shared"
            | "use"
            | "out"
            | "type"
            | "enum"
            | "test"
            | "env"
            | "middleware"
            | "true"
            | "false"
            | "null"
            | "void"
            | "never"
    )
}

// ── Reference collection — comprehensive AST walk ──────────────────────

fn collect_references(
    items: &[Item],
    target: &SmolStr,
    docs: &DocumentManager,
    locations: &mut Vec<Location>,
) {
    for item in items {
        match item {
            Item::ComponentDecl(c) => {
                check_name(&c.name, &c.span, target, docs, locations);
                collect_refs_in_params(&c.props, target, docs, locations);
                collect_refs_in_component_body(&c.body, target, docs, locations);
            }
            Item::LayoutDecl(l) => {
                check_name(&l.name, &l.span, target, docs, locations);
                collect_refs_in_params(&l.props, target, docs, locations);
                collect_refs_in_component_body(&l.body, target, docs, locations);
            }
            Item::FnDecl(f) => {
                check_name(&f.name, &f.span, target, docs, locations);
                collect_refs_in_params(&f.params, target, docs, locations);
                collect_refs_in_type_ann_opt(&f.ret_ty, target, docs, locations);
                collect_refs_in_block(&f.body, target, docs, locations);
            }
            Item::ActionDecl(a) => {
                check_name(&a.name, &a.span, target, docs, locations);
                collect_refs_in_params(&a.params, target, docs, locations);
                collect_refs_in_type_ann_opt(&a.ret_ty, target, docs, locations);
                collect_refs_in_block(&a.body, target, docs, locations);
            }
            Item::GuardDecl(g) => {
                check_name(&g.name, &g.span, target, docs, locations);
                for field in &g.fields {
                    check_name(&field.name, &field.span, target, docs, locations);
                    collect_refs_in_type_ann(&field.ty, target, docs, locations);
                    for v in &field.validators {
                        for arg in &v.args {
                            collect_refs_in_expr(arg, target, docs, locations);
                        }
                    }
                }
            }
            Item::StoreDecl(s) => {
                check_name(&s.name, &s.span, target, docs, locations);
                for member in &s.members {
                    match member {
                        StoreMember::Signal(stmt) | StoreMember::Derive(stmt) => {
                            collect_refs_in_stmt(stmt, target, docs, locations);
                        }
                        StoreMember::Method(f) => {
                            check_name(&f.name, &f.span, target, docs, locations);
                            collect_refs_in_params(&f.params, target, docs, locations);
                            collect_refs_in_block(&f.body, target, docs, locations);
                        }
                    }
                }
            }
            Item::QueryDecl(q) => {
                check_name(&q.name, &q.span, target, docs, locations);
                collect_refs_in_expr(&q.url_pattern, target, docs, locations);
                collect_refs_in_type_ann_opt(&q.ret_ty, target, docs, locations);
            }
            Item::ChannelDecl(ch) => {
                check_name(&ch.name, &ch.span, target, docs, locations);
                collect_refs_in_params(&ch.params, target, docs, locations);
                collect_refs_in_type_ann(&ch.msg_ty, target, docs, locations);
                for handler in &ch.handlers {
                    collect_refs_in_params(&handler.params, target, docs, locations);
                    collect_refs_in_block(&handler.body, target, docs, locations);
                }
            }
            Item::TypeAlias(t) => {
                check_name(&t.name, &t.span, target, docs, locations);
                collect_refs_in_type_ann(&t.ty, target, docs, locations);
            }
            Item::EnumDecl(e) => {
                check_name(&e.name, &e.span, target, docs, locations);
                for variant in &e.variants {
                    if variant == target {
                        add_span_location(e.span, docs, locations);
                    }
                }
            }
            Item::TestDecl(t) => {
                collect_refs_in_block(&t.body, target, docs, locations);
            }
            Item::ApiDecl(api) => {
                check_name(&api.name, &api.span, target, docs, locations);
                for handler in &api.handlers {
                    collect_refs_in_params(&handler.params, target, docs, locations);
                    collect_refs_in_type_ann_opt(&handler.ret_ty, target, docs, locations);
                    collect_refs_in_block(&handler.body, target, docs, locations);
                }
            }
            Item::MiddlewareDecl(m) => {
                check_name(&m.name, &m.span, target, docs, locations);
                collect_refs_in_params(&m.params, target, docs, locations);
                collect_refs_in_block(&m.body, target, docs, locations);
            }
            Item::EnvDecl(env) => {
                for var in &env.vars {
                    collect_refs_in_type_ann(&var.ty, target, docs, locations);
                    for v in &var.validators {
                        for arg in &v.args {
                            collect_refs_in_expr(arg, target, docs, locations);
                        }
                    }
                    if let Some(ref default) = var.default {
                        collect_refs_in_expr(default, target, docs, locations);
                    }
                }
            }
            Item::Out(out) => {
                collect_references(&[*out.inner.clone()], target, docs, locations);
            }
            Item::ServerBlock(b) | Item::ClientBlock(b) | Item::SharedBlock(b) => {
                collect_references(&b.items, target, docs, locations);
            }
            Item::Stmt(stmt) => {
                collect_refs_in_stmt(stmt, target, docs, locations);
            }
            Item::Use(_) => {
                // Import references could be tracked, but names are strings
            }
        }
    }
}

fn collect_refs_in_component_body(
    body: &ComponentBody,
    target: &SmolStr,
    docs: &DocumentManager,
    locations: &mut Vec<Location>,
) {
    for stmt in &body.stmts {
        collect_refs_in_stmt(stmt, target, docs, locations);
    }
    collect_refs_in_template(&body.template, target, docs, locations);
    if let Some(ref head) = body.head {
        for field in &head.fields {
            collect_refs_in_expr(&field.value, target, docs, locations);
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
        Stmt::Let {
            name, init, span, ..
        }
        | Stmt::Mut {
            name, init, span, ..
        }
        | Stmt::Signal {
            name, init, span, ..
        } => {
            if name == target {
                add_span_location(*span, docs, locations);
            }
            collect_refs_in_expr(init, target, docs, locations);
        }
        Stmt::Derive {
            name, init, span, ..
        }
        | Stmt::Frozen {
            name, init, span, ..
        } => {
            if name == target {
                add_span_location(*span, docs, locations);
            }
            collect_refs_in_expr(init, target, docs, locations);
        }
        Stmt::RefDecl { name, span, .. } => {
            if name == target {
                add_span_location(*span, docs, locations);
            }
        }
        Stmt::FnDecl(f) => {
            check_name(&f.name, &f.span, target, docs, locations);
            collect_refs_in_params(&f.params, target, docs, locations);
            collect_refs_in_type_ann_opt(&f.ret_ty, target, docs, locations);
            collect_refs_in_block(&f.body, target, docs, locations);
        }
        Stmt::If {
            condition,
            then_block,
            else_branch,
            ..
        } => {
            collect_refs_in_expr(condition, target, docs, locations);
            collect_refs_in_block(then_block, target, docs, locations);
            if let Some(eb) = else_branch {
                match eb {
                    ElseBranch::Else(block) => {
                        collect_refs_in_block(block, target, docs, locations);
                    }
                    ElseBranch::ElseIf(stmt) => {
                        collect_refs_in_stmt(stmt, target, docs, locations);
                    }
                }
            }
        }
        Stmt::For {
            binding,
            index,
            iterable,
            body,
            span,
        } => {
            if binding == target {
                add_span_location(*span, docs, locations);
            }
            if let Some(idx) = index {
                if idx == target {
                    add_span_location(*span, docs, locations);
                }
            }
            collect_refs_in_expr(iterable, target, docs, locations);
            collect_refs_in_block(body, target, docs, locations);
        }
        Stmt::Return { value, .. } => {
            if let Some(e) = value {
                collect_refs_in_expr(e, target, docs, locations);
            }
        }
        Stmt::Effect { body, cleanup, .. } => {
            collect_refs_in_block(body, target, docs, locations);
            if let Some(cleanup_block) = cleanup {
                collect_refs_in_block(cleanup_block, target, docs, locations);
            }
        }
        Stmt::Watch {
            target: watched,
            next_name,
            prev_name,
            body,
            span,
        } => {
            collect_refs_in_expr(watched, target, docs, locations);
            if next_name == target || prev_name == target {
                add_span_location(*span, docs, locations);
            }
            collect_refs_in_block(body, target, docs, locations);
        }
        Stmt::ExprStmt { expr, .. } => {
            collect_refs_in_expr(expr, target, docs, locations);
        }
        Stmt::Block(block) => {
            collect_refs_in_block(block, target, docs, locations);
        }
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
        Expr::UnaryOp { operand, .. } => {
            collect_refs_in_expr(operand, target, docs, locations);
        }
        Expr::FnCall { callee, args, .. } => {
            collect_refs_in_expr(callee, target, docs, locations);
            for arg in args {
                collect_refs_in_expr(arg, target, docs, locations);
            }
        }
        Expr::MemberAccess { object, .. } => {
            collect_refs_in_expr(object, target, docs, locations);
        }
        Expr::OptionalChain { object, .. } => {
            collect_refs_in_expr(object, target, docs, locations);
        }
        Expr::IndexAccess { object, index, .. } => {
            collect_refs_in_expr(object, target, docs, locations);
            collect_refs_in_expr(index, target, docs, locations);
        }
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
        Expr::NullCoalesce { left, right, .. } => {
            collect_refs_in_expr(left, target, docs, locations);
            collect_refs_in_expr(right, target, docs, locations);
        }
        Expr::ArrayLit { elements, .. } => {
            for el in elements {
                collect_refs_in_expr(el, target, docs, locations);
            }
        }
        Expr::ObjectLit { fields, .. } => {
            for field in fields {
                collect_refs_in_expr(&field.value, target, docs, locations);
            }
        }
        Expr::ArrowFn { params, body, .. } => {
            collect_refs_in_params(params, target, docs, locations);
            match body {
                ArrowBody::Expr(e) => collect_refs_in_expr(e, target, docs, locations),
                ArrowBody::Block(b) => collect_refs_in_block(b, target, docs, locations),
            }
        }
        Expr::Spread { expr, .. } => {
            collect_refs_in_expr(expr, target, docs, locations);
        }
        Expr::Range { start, end, .. } => {
            collect_refs_in_expr(start, target, docs, locations);
            collect_refs_in_expr(end, target, docs, locations);
        }
        Expr::Pipe { left, right, .. } => {
            collect_refs_in_expr(left, target, docs, locations);
            collect_refs_in_expr(right, target, docs, locations);
        }
        Expr::Await { expr, .. } => {
            collect_refs_in_expr(expr, target, docs, locations);
        }
        Expr::Assign {
            target: t, value, ..
        } => {
            collect_refs_in_expr(t, target, docs, locations);
            collect_refs_in_expr(value, target, docs, locations);
        }
        Expr::Assert { expr, .. } => {
            collect_refs_in_expr(expr, target, docs, locations);
        }
        Expr::TemplateLit { parts, .. } => {
            for part in parts {
                if let TemplatePart::Expr(e) = part {
                    collect_refs_in_expr(e, target, docs, locations);
                }
            }
        }
        Expr::EnvAccess { key, span } if key == target => {
            add_span_location(*span, docs, locations);
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
                collect_refs_in_expr(expr, target, docs, locations);
            }
            TemplateNode::Element {
                children,
                directives,
                attributes,
                ..
            } => {
                collect_refs_in_attrs(attributes, target, docs, locations);
                collect_refs_in_directives(directives, target, docs, locations);
                collect_refs_in_template(children, target, docs, locations);
            }
            TemplateNode::SelfClosing {
                directives,
                attributes,
                ..
            } => {
                collect_refs_in_attrs(attributes, target, docs, locations);
                collect_refs_in_directives(directives, target, docs, locations);
            }
            TemplateNode::When {
                condition,
                body,
                else_branch,
                ..
            } => {
                collect_refs_in_expr(condition, target, docs, locations);
                collect_refs_in_template(body, target, docs, locations);
                if let Some(eb) = else_branch {
                    match eb {
                        WhenElse::Else(nodes) => {
                            collect_refs_in_template(nodes, target, docs, locations);
                        }
                        WhenElse::ElseWhen(node) => {
                            collect_refs_in_template(&[*node.clone()], target, docs, locations);
                        }
                    }
                }
            }
            TemplateNode::Each {
                iterable,
                body,
                empty,
                binding,
                index,
                span,
                ..
            } => {
                collect_refs_in_expr(iterable, target, docs, locations);
                if binding == target {
                    add_span_location(*span, docs, locations);
                }
                if let Some(idx) = index {
                    if idx == target {
                        add_span_location(*span, docs, locations);
                    }
                }
                collect_refs_in_template(body, target, docs, locations);
                if let Some(empty_nodes) = empty {
                    collect_refs_in_template(empty_nodes, target, docs, locations);
                }
            }
            TemplateNode::Suspend { body, .. } => {
                collect_refs_in_template(body, target, docs, locations);
            }
            _ => {}
        }
    }
}

fn collect_refs_in_attrs(
    attrs: &[Attribute],
    target: &SmolStr,
    docs: &DocumentManager,
    locations: &mut Vec<Location>,
) {
    for attr in attrs {
        if let AttrValue::Expr(e) = &attr.value {
            collect_refs_in_expr(e, target, docs, locations);
        }
    }
}

fn collect_refs_in_directives(
    directives: &[Directive],
    target: &SmolStr,
    docs: &DocumentManager,
    locations: &mut Vec<Location>,
) {
    for dir in directives {
        match dir {
            Directive::Bind { field, span } => {
                if field == target {
                    add_span_location(*span, docs, locations);
                }
            }
            Directive::On { handler, .. } => {
                collect_refs_in_expr(handler, target, docs, locations);
            }
            Directive::Class { condition, .. } => {
                collect_refs_in_expr(condition, target, docs, locations);
            }
            Directive::Ref { name, span } => {
                if name == target {
                    add_span_location(*span, docs, locations);
                }
            }
            Directive::Transition { config, .. } => {
                if let Some(e) = config {
                    collect_refs_in_expr(e, target, docs, locations);
                }
            }
            Directive::Key { expr, .. } => {
                collect_refs_in_expr(expr, target, docs, locations);
            }
            Directive::FormAction { action, .. } => {
                collect_refs_in_expr(action, target, docs, locations);
            }
            Directive::FormGuard { guard, .. } => {
                collect_refs_in_expr(guard, target, docs, locations);
            }
            _ => {}
        }
    }
}

fn collect_refs_in_params(
    params: &[Param],
    target: &SmolStr,
    docs: &DocumentManager,
    locations: &mut Vec<Location>,
) {
    for param in params {
        if &param.name == target {
            add_span_location(param.span, docs, locations);
        }
        collect_refs_in_type_ann_opt(&param.ty_ann, target, docs, locations);
        if let Some(ref default) = param.default {
            collect_refs_in_expr(default, target, docs, locations);
        }
    }
}

fn collect_refs_in_type_ann(
    ty: &TypeAnnotation,
    target: &SmolStr,
    docs: &DocumentManager,
    locations: &mut Vec<Location>,
) {
    match ty {
        TypeAnnotation::Named { name, span } => {
            if name == target {
                add_span_location(*span, docs, locations);
            }
        }
        TypeAnnotation::Array { element, .. } => {
            collect_refs_in_type_ann(element, target, docs, locations);
        }
        TypeAnnotation::Union { types, .. } => {
            for t in types {
                collect_refs_in_type_ann(t, target, docs, locations);
            }
        }
        TypeAnnotation::Optional { inner, .. } => {
            collect_refs_in_type_ann(inner, target, docs, locations);
        }
        TypeAnnotation::Function { params, ret, .. } => {
            for p in params {
                collect_refs_in_type_ann(p, target, docs, locations);
            }
            collect_refs_in_type_ann(ret, target, docs, locations);
        }
        TypeAnnotation::Tuple { elements, .. } => {
            for e in elements {
                collect_refs_in_type_ann(e, target, docs, locations);
            }
        }
        TypeAnnotation::Object { fields, .. } => {
            for f in fields {
                collect_refs_in_type_ann(&f.ty, target, docs, locations);
            }
        }
        TypeAnnotation::StringLiteral { .. } => {}
    }
}

fn collect_refs_in_type_ann_opt(
    ty: &Option<TypeAnnotation>,
    target: &SmolStr,
    docs: &DocumentManager,
    locations: &mut Vec<Location>,
) {
    if let Some(t) = ty {
        collect_refs_in_type_ann(t, target, docs, locations);
    }
}

// ── Helpers ────────────────────────────────────────────────────────────

fn check_name(
    name: &SmolStr,
    span: &Span,
    target: &SmolStr,
    docs: &DocumentManager,
    locations: &mut Vec<Location>,
) {
    if name == target {
        add_span_location(*span, docs, locations);
    }
}

fn add_span_location(span: Span, docs: &DocumentManager, locations: &mut Vec<Location>) {
    if let Some(path) = docs.file_table.get_path(span.file_id) {
        if let Ok(uri) = Url::from_file_path(path) {
            let range = match docs.get_source_by_file_id(span.file_id) {
                Some(source) => span_to_lsp_range(&span, source),
                None => {
                    let start = Position {
                        line: span.line.saturating_sub(1),
                        character: span.col.saturating_sub(1),
                    };
                    Range { start, end: start }
                }
            };
            locations.push(Location { uri, range });
        }
    }
}
