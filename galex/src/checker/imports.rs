//! Module and import validation (GX0800–GX0809).
//!
//! Validates imports at the program level:
//! - Duplicate imports (GX0809)
//! - Unused imports (GX0805)
//! - Circular imports (GX0802)
//! - Module not found (GX0800)
//! - Named export not found (GX0801)
//! - Ambiguous import (GX0803)
//! - Symbol not exported (GX0804)
//! - Server module in client (GX0806)
//! - Package not installed (GX0807)
//! - Package version conflict (GX0808)

use crate::ast::*;
use crate::errors::{codes, Diagnostic};
use crate::span::Span;
use std::collections::{HashMap, HashSet};

/// Validate all imports in a program.
///
/// Checks for duplicate imports, unused imports, and structural issues.
/// Module resolution (GX0800) and cross-boundary checks (GX0806) are
/// stubbed — they require the module graph which is built later.
pub fn validate_imports(program: &Program) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // Collect all imported names and their source spans
    let mut imported_names: HashMap<String, Vec<Span>> = HashMap::new();
    // Collect all import paths for circular detection
    let mut import_paths: HashMap<String, Vec<Span>> = HashMap::new();

    for item in &program.items {
        match item {
            Item::Use(decl) => {
                collect_import_names(decl, &mut imported_names, &mut diagnostics);
                import_paths
                    .entry(decl.path.to_string())
                    .or_default()
                    .push(decl.span);
            }
            // Check imports inside boundary blocks too
            Item::ServerBlock(block) | Item::ClientBlock(block) | Item::SharedBlock(block) => {
                for inner in &block.items {
                    if let Item::Use(decl) = inner {
                        collect_import_names(decl, &mut imported_names, &mut diagnostics);
                        import_paths
                            .entry(decl.path.to_string())
                            .or_default()
                            .push(decl.span);
                    }
                }
            }
            _ => {}
        }
    }

    // GX0805: Unused import detection
    // Collect all referenced names in the program
    let referenced = collect_referenced_names(program);
    for (name, spans) in &imported_names {
        if !referenced.contains(name.as_str()) {
            // Report on the first import of this name
            if let Some(&span) = spans.first() {
                diagnostics.push(
                    Diagnostic::with_message(
                        &codes::GX0805,
                        format!("Unused import `{}`", name),
                        span,
                    )
                    .with_hint("remove this import or use the imported symbol"),
                );
            }
        }
    }

    diagnostics
}

/// Collect imported names from a `use` declaration, detecting duplicates (GX0809).
fn collect_import_names(
    decl: &UseDecl,
    imported_names: &mut HashMap<String, Vec<Span>>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let names: Vec<String> = match &decl.imports {
        ImportKind::Default(name) => vec![name.to_string()],
        ImportKind::Named(names) => names.iter().map(|n| n.to_string()).collect(),
        ImportKind::Star => vec![], // Can't track individual names for star imports
    };

    for name in names {
        let entry = imported_names.entry(name.clone()).or_default();
        if !entry.is_empty() {
            // GX0809: Duplicate import
            diagnostics.push(
                Diagnostic::with_message(
                    &codes::GX0809,
                    format!("Duplicate import of `{}`", name),
                    decl.span,
                )
                .with_hint("this symbol is already imported above"),
            );
        }
        entry.push(decl.span);
    }
}

/// Collect all names referenced in the program body (excluding imports).
///
/// This is a best-effort scan — it looks at identifiers in expressions,
/// statements, and templates to determine which imported names are used.
fn collect_referenced_names(program: &Program) -> HashSet<String> {
    let mut names = HashSet::new();
    for item in &program.items {
        collect_names_from_item(item, &mut names);
    }
    names
}

/// Recursively collect referenced names from an item.
fn collect_names_from_item(item: &Item, names: &mut HashSet<String>) {
    match item {
        Item::Use(_) => {} // Skip imports themselves
        Item::FnDecl(decl) => {
            collect_names_from_block(&decl.body, names);
        }
        Item::GuardDecl(decl) => {
            for field in &decl.fields {
                if let TypeAnnotation::Named { name, .. } = &field.ty {
                    names.insert(name.to_string());
                }
            }
        }
        Item::StoreDecl(decl) => {
            for member in &decl.members {
                match member {
                    StoreMember::Signal(stmt) | StoreMember::Derive(stmt) => {
                        collect_names_from_stmt(stmt, names);
                    }
                    StoreMember::Method(fn_decl) => {
                        collect_names_from_block(&fn_decl.body, names);
                    }
                }
            }
        }
        Item::ActionDecl(decl) => {
            collect_names_from_block(&decl.body, names);
        }
        Item::ComponentDecl(decl) => {
            for stmt in &decl.body.stmts {
                collect_names_from_stmt(stmt, names);
            }
            collect_names_from_template(&decl.body.template, names);
        }
        Item::LayoutDecl(decl) => {
            for stmt in &decl.body.stmts {
                collect_names_from_stmt(stmt, names);
            }
            collect_names_from_template(&decl.body.template, names);
        }
        Item::ServerBlock(block) | Item::ClientBlock(block) | Item::SharedBlock(block) => {
            for inner in &block.items {
                collect_names_from_item(inner, names);
            }
        }
        Item::Out(out) => {
            collect_names_from_item(&out.inner, names);
        }
        Item::Stmt(stmt) => {
            collect_names_from_stmt(stmt, names);
        }
        _ => {}
    }
}

/// Collect referenced names from a block.
fn collect_names_from_block(block: &Block, names: &mut HashSet<String>) {
    for stmt in &block.stmts {
        collect_names_from_stmt(stmt, names);
    }
}

/// Collect referenced names from a statement.
fn collect_names_from_stmt(stmt: &Stmt, names: &mut HashSet<String>) {
    match stmt {
        Stmt::Let { init, .. }
        | Stmt::Mut { init, .. }
        | Stmt::Signal { init, .. }
        | Stmt::Derive { init, .. }
        | Stmt::Frozen { init, .. } => {
            collect_names_from_expr(init, names);
        }
        Stmt::ExprStmt { expr, .. } => {
            collect_names_from_expr(expr, names);
        }
        Stmt::Return { value, .. } => {
            if let Some(expr) = value {
                collect_names_from_expr(expr, names);
            }
        }
        Stmt::If {
            condition,
            then_block,
            ..
        } => {
            collect_names_from_expr(condition, names);
            collect_names_from_block(then_block, names);
        }
        Stmt::For { iterable, body, .. } => {
            collect_names_from_expr(iterable, names);
            collect_names_from_block(body, names);
        }
        Stmt::FnDecl(decl) => {
            collect_names_from_block(&decl.body, names);
        }
        Stmt::Block(block) => {
            collect_names_from_block(block, names);
        }
        _ => {}
    }
}

/// Collect referenced names from an expression.
fn collect_names_from_expr(expr: &Expr, names: &mut HashSet<String>) {
    match expr {
        Expr::Ident { name, .. } => {
            names.insert(name.to_string());
        }
        Expr::FnCall { callee, args, .. } => {
            collect_names_from_expr(callee, names);
            for arg in args {
                collect_names_from_expr(arg, names);
            }
        }
        Expr::MemberAccess { object, .. } | Expr::OptionalChain { object, .. } => {
            collect_names_from_expr(object, names);
        }
        Expr::BinaryOp { left, right, .. } => {
            collect_names_from_expr(left, names);
            collect_names_from_expr(right, names);
        }
        Expr::UnaryOp { operand, .. } => {
            collect_names_from_expr(operand, names);
        }
        Expr::Assign { target, value, .. } => {
            collect_names_from_expr(target, names);
            collect_names_from_expr(value, names);
        }
        Expr::ArrayLit { elements, .. } => {
            for elem in elements {
                collect_names_from_expr(elem, names);
            }
        }
        Expr::ObjectLit { fields, .. } => {
            for field in fields {
                collect_names_from_expr(&field.value, names);
            }
        }
        Expr::Await { expr: inner, .. } | Expr::Spread { expr: inner, .. } => {
            collect_names_from_expr(inner, names);
        }
        Expr::Ternary {
            condition,
            then_expr,
            else_expr,
            ..
        } => {
            collect_names_from_expr(condition, names);
            collect_names_from_expr(then_expr, names);
            collect_names_from_expr(else_expr, names);
        }
        Expr::IndexAccess { object, index, .. } => {
            collect_names_from_expr(object, names);
            collect_names_from_expr(index, names);
        }
        Expr::TemplateLit { parts, .. } => {
            for part in parts {
                if let TemplatePart::Expr(e) = part {
                    collect_names_from_expr(e, names);
                }
            }
        }
        Expr::Pipe { left, right, .. } | Expr::NullCoalesce { left, right, .. } => {
            collect_names_from_expr(left, names);
            collect_names_from_expr(right, names);
        }
        _ => {}
    }
}

/// Collect referenced names from template nodes.
fn collect_names_from_template(nodes: &[TemplateNode], names: &mut HashSet<String>) {
    for node in nodes {
        match node {
            TemplateNode::Element {
                tag,
                children,
                attributes,
                ..
            } => {
                // Component references (capitalized tags)
                if tag.chars().next().map_or(false, |c| c.is_uppercase()) {
                    names.insert(tag.to_string());
                }
                for attr in attributes {
                    if let AttrValue::Expr(expr) = &attr.value {
                        collect_names_from_expr(expr, names);
                    }
                }
                collect_names_from_template(children, names);
            }
            TemplateNode::SelfClosing {
                tag, attributes, ..
            } => {
                if tag.chars().next().map_or(false, |c| c.is_uppercase()) {
                    names.insert(tag.to_string());
                }
                for attr in attributes {
                    if let AttrValue::Expr(expr) = &attr.value {
                        collect_names_from_expr(expr, names);
                    }
                }
            }
            TemplateNode::ExprInterp { expr, .. } => {
                collect_names_from_expr(expr, names);
            }
            TemplateNode::When {
                condition,
                body,
                else_branch,
                ..
            } => {
                collect_names_from_expr(condition, names);
                collect_names_from_template(body, names);
                if let Some(WhenElse::Else(nodes)) = else_branch {
                    collect_names_from_template(nodes, names);
                }
            }
            TemplateNode::Each { iterable, body, .. } => {
                collect_names_from_expr(iterable, names);
                collect_names_from_template(body, names);
            }
            TemplateNode::Suspend { body, .. } => {
                collect_names_from_template(body, names);
            }
            _ => {}
        }
    }
}

/// Validate that a module path is resolvable (GX0800).
///
/// This is called during module resolution, not during the initial import
/// scan. Returns a diagnostic if the path cannot be resolved.
pub fn validate_module_exists(path: &str, span: Span) -> Option<Diagnostic> {
    // Module resolution is a later phase — stub for now.
    // In a real implementation this would check the filesystem and gale_modules/.
    if path.is_empty() {
        Some(
            Diagnostic::with_message(
                &codes::GX0800,
                format!("Module not found: `{}`", path),
                span,
            )
            .with_hint("check the import path is correct"),
        )
    } else {
        None
    }
}

/// Check that a named export exists in a module (GX0801).
pub fn validate_named_export(
    module_path: &str,
    name: &str,
    _exports: &HashSet<String>,
    span: Span,
) -> Option<Diagnostic> {
    if !_exports.contains(name) {
        Some(
            Diagnostic::with_message(
                &codes::GX0801,
                format!(
                    "Named export `{}` not found in module `{}`",
                    name, module_path
                ),
                span,
            )
            .with_hint("check the module's `out` exports"),
        )
    } else {
        None
    }
}

/// Detect circular imports (GX0802).
///
/// Takes a list of module paths in the current import chain.
/// If the target path is already in the chain, a cycle exists.
pub fn check_circular_import(
    target_path: &str,
    import_chain: &[String],
    span: Span,
) -> Option<Diagnostic> {
    if import_chain.contains(&target_path.to_string()) {
        let cycle = import_chain.join(" -> ");
        Some(
            Diagnostic::with_message(
                &codes::GX0802,
                format!("Circular import detected: {} -> {}", cycle, target_path),
                span,
            )
            .with_hint("break the cycle by extracting shared code into a separate module"),
        )
    } else {
        None
    }
}

/// Check for ambiguous imports (GX0803).
///
/// If a name is exported by multiple imported modules, it's ambiguous.
pub fn check_ambiguous_import(name: &str, sources: &[String], span: Span) -> Option<Diagnostic> {
    if sources.len() > 1 {
        Some(
            Diagnostic::with_message(
                &codes::GX0803,
                format!(
                    "Ambiguous import `{}` — exported by: {}",
                    name,
                    sources.join(", ")
                ),
                span,
            )
            .with_hint("use a named import with an explicit path to disambiguate"),
        )
    } else {
        None
    }
}

/// Check that a symbol has the `out` keyword (GX0804).
pub fn check_symbol_exported(
    name: &str,
    module_path: &str,
    is_exported: bool,
    span: Span,
) -> Option<Diagnostic> {
    if !is_exported {
        Some(
            Diagnostic::with_message(
                &codes::GX0804,
                format!(
                    "Cannot import `{}` — symbol exists in `{}` but is not exported",
                    name, module_path
                ),
                span,
            )
            .with_hint("add `out` keyword to the declaration in the source module"),
        )
    } else {
        None
    }
}

/// Check that a server-only module is not imported in client context (GX0806).
pub fn check_server_module_in_client(
    module_path: &str,
    is_server_only: bool,
    in_client_scope: bool,
    span: Span,
) -> Option<Diagnostic> {
    if is_server_only && in_client_scope {
        Some(
            Diagnostic::with_message(
                &codes::GX0806,
                format!(
                    "Cannot import server-only module `{}` in client block",
                    module_path
                ),
                span,
            )
            .with_hint("move this import to a server { } block"),
        )
    } else {
        None
    }
}

/// Check that a package is installed (GX0807).
pub fn check_package_installed(
    package_name: &str,
    is_installed: bool,
    span: Span,
) -> Option<Diagnostic> {
    if !is_installed {
        Some(
            Diagnostic::with_message(
                &codes::GX0807,
                format!("Package `{}` is not installed", package_name),
                span,
            )
            .with_hint("run `gale add` to install the package"),
        )
    } else {
        None
    }
}

/// Check for package version conflicts (GX0808).
pub fn check_version_conflict(
    package_name: &str,
    versions: &[String],
    span: Span,
) -> Option<Diagnostic> {
    if versions.len() > 1 {
        Some(
            Diagnostic::with_message(
                &codes::GX0808,
                format!(
                    "Package version conflict for `{}`: {}",
                    package_name,
                    versions.join(" vs ")
                ),
                span,
            )
            .with_hint("resolve version conflicts in gale.toml"),
        )
    } else {
        None
    }
}
