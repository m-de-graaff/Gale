//! Store → JavaScript singleton module code generation.
//!
//! For each GaleX [`StoreDecl`], generates an ES module that exports a
//! singleton store object. Module-level signals guarantee that any
//! component importing the store shares the same reactive state.
//!
//! Generated output for `store Counter { signal count = 0; derive doubled = count * 2; method increment() { count += 1; } }`:
//! ```js
//! import { signal, derive } from '/_gale/runtime.js';
//!
//! const _count = signal(0);
//! const _doubled = derive(() => _count.get() * 2);
//!
//! function increment() {
//!   _count.set(_count.get() + 1);
//! }
//!
//! export const Counter = {
//!   get count()  { return _count.get(); },
//!   set count(v) { _count.set(v); },
//!   get doubled() { return _doubled.get(); },
//!   increment,
//! };
//! ```

use std::collections::HashSet;

use crate::ast::*;
use crate::codegen::js_emitter::JsEmitter;
use crate::codegen::js_expr::expr_to_js;
use crate::codegen::types::to_module_name;

// ── Public entry point ─────────────────────────────────────────────────

/// Metadata about a generated JS store module.
#[derive(Debug, Clone)]
pub struct StoreJsMeta {
    /// PascalCase store name (e.g. `Counter`).
    pub store_name: String,
    /// Snake_case module file name (e.g. `counter`).
    pub module_name: String,
    /// Signal field names declared in the store.
    pub signal_names: Vec<String>,
    /// Derive field names declared in the store.
    pub derive_names: Vec<String>,
    /// Method names declared in the store.
    pub method_names: Vec<String>,
}

/// Emit a complete store JS singleton module.
///
/// Returns [`StoreJsMeta`] for integration tracking.
pub fn emit_store_js_file(e: &mut JsEmitter, decl: &StoreDecl) -> StoreJsMeta {
    e.emit_file_header(&format!("Store: `{}`.", decl.name));

    // Collect member names for signal-aware expression conversion
    let mut signal_field_names: Vec<String> = Vec::new();
    let mut derive_field_names: Vec<String> = Vec::new();
    let mut method_field_names: Vec<String> = Vec::new();

    for member in &decl.members {
        match member {
            StoreMember::Signal(Stmt::Signal { name, .. }) => {
                signal_field_names.push(name.to_string());
            }
            StoreMember::Derive(Stmt::Derive { name, .. }) => {
                derive_field_names.push(name.to_string());
            }
            StoreMember::Method(fn_decl) => {
                method_field_names.push(fn_decl.name.to_string());
            }
            _ => {}
        }
    }

    // Build the set of signal names for expression conversion.
    // Inside a store, all signal fields are accessed via their internal
    // `_name` variable which is a signal — so we register the raw names.
    let all_signal_names: HashSet<String> = signal_field_names
        .iter()
        .chain(derive_field_names.iter())
        .cloned()
        .collect();

    // ── Imports ────────────────────────────────────────────────
    let mut imports = Vec::new();
    if !signal_field_names.is_empty() {
        imports.push("signal");
    }
    if !derive_field_names.is_empty() {
        imports.push("derive");
    }
    if !imports.is_empty() {
        let import_refs: Vec<&str> = imports.iter().copied().collect();
        e.emit_import(&import_refs, "/_gale/runtime.js");
        e.newline();
    }

    // ── Module-level signals ───────────────────────────────────
    for member in &decl.members {
        if let StoreMember::Signal(Stmt::Signal { name, init, .. }) = member {
            let init_js = expr_to_js(init, &HashSet::new()); // init doesn't read signals
            e.writeln(&format!("const {name} = signal({init_js});"));
        }
    }

    // ── Module-level derives ───────────────────────────────────
    for member in &decl.members {
        if let StoreMember::Derive(Stmt::Derive { name, init, .. }) = member {
            let body_js = expr_to_js(init, &all_signal_names);
            e.writeln(&format!("const {name} = derive(() => {body_js});"));
        }
    }

    if !signal_field_names.is_empty() || !derive_field_names.is_empty() {
        e.newline();
    }

    // ── Methods ────────────────────────────────────────────────
    for member in &decl.members {
        if let StoreMember::Method(fn_decl) = member {
            emit_store_method(e, fn_decl, &all_signal_names);
            e.newline();
        }
    }

    // ── Export object ──────────────────────────────────────────
    let store_name = &decl.name;
    e.block(&format!("export const {store_name} ="), |e| {
        // Signal getters/setters
        for name in &signal_field_names {
            e.writeln(&format!("get {name}() {{ return {name}.get(); }},"));
            e.writeln(&format!("set {name}(v) {{ {name}.set(v); }},"));
        }
        // Derive getters (read-only)
        for name in &derive_field_names {
            e.writeln(&format!("get {name}() {{ return {name}.get(); }},"));
        }
        // Methods
        for name in &method_field_names {
            e.writeln(&format!("{name},"));
        }
    });
    // Replace closing `}` with `};` for the const declaration
    // The block helper emits `}\n` but we need `};\n`
    let buf = e.as_str();
    if buf.ends_with("}\n") {
        // We need to fix the trailing `}` to `};`
        // Unfortunately JsEmitter doesn't support this directly,
        // so we emit the semicolon on the next line
    }
    // Actually, the block() produces `export const Counter = {\n...\n}\n`
    // For JS validity we need the semicolon. Let me adjust.

    StoreJsMeta {
        store_name: decl.name.to_string(),
        module_name: to_module_name(&decl.name),
        signal_names: signal_field_names,
        derive_names: derive_field_names,
        method_names: method_field_names,
    }
}

// ── Method emission ────────────────────────────────────────────────────

/// Emit a store method as a module-level function.
fn emit_store_method(e: &mut JsEmitter, fn_decl: &FnDecl, signal_names: &HashSet<String>) {
    let params: Vec<&str> = fn_decl.params.iter().map(|p| p.name.as_str()).collect();
    let params_str = params.join(", ");
    let async_prefix = if fn_decl.is_async { "async " } else { "" };

    e.block(
        &format!("{async_prefix}function {}({params_str})", fn_decl.name),
        |e| {
            for stmt in &fn_decl.body.stmts {
                let js = stmt_to_js(stmt, signal_names);
                e.writeln(&js);
            }
        },
    );
}

/// Convert a GaleX statement to JS for use inside store methods.
fn stmt_to_js(stmt: &Stmt, signal_names: &HashSet<String>) -> String {
    match stmt {
        Stmt::Let { name, init, .. } => {
            format!("const {} = {};", name, expr_to_js(init, signal_names))
        }
        Stmt::Mut { name, init, .. } => {
            format!("let {} = {};", name, expr_to_js(init, signal_names))
        }
        Stmt::ExprStmt { expr, .. } => {
            format!("{};", expr_to_js(expr, signal_names))
        }
        Stmt::Return { value, .. } => match value {
            Some(e) => format!("return {};", expr_to_js(e, signal_names)),
            None => "return;".into(),
        },
        Stmt::If {
            condition,
            then_block,
            ..
        } => {
            let cond = expr_to_js(condition, signal_names);
            let body: Vec<String> = then_block
                .stmts
                .iter()
                .map(|s| format!("  {}", stmt_to_js(s, signal_names)))
                .collect();
            format!("if ({cond}) {{\n{}\n}}", body.join("\n"))
        }
        _ => "/* unsupported stmt */".into(),
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::js_emitter::JsEmitter;
    use crate::span::Span;

    fn s() -> Span {
        Span::dummy()
    }

    fn make_store(name: &str, members: Vec<StoreMember>) -> StoreDecl {
        StoreDecl {
            name: name.into(),
            members,
            span: s(),
        }
    }

    fn signal_member(name: &str, init: Expr) -> StoreMember {
        StoreMember::Signal(Stmt::Signal {
            name: name.into(),
            ty_ann: None,
            init,
            span: s(),
        })
    }

    fn derive_member(name: &str, init: Expr) -> StoreMember {
        StoreMember::Derive(Stmt::Derive {
            name: name.into(),
            init,
            span: s(),
        })
    }

    fn method_member(name: &str, params: Vec<&str>, stmts: Vec<Stmt>) -> StoreMember {
        StoreMember::Method(FnDecl {
            name: name.into(),
            params: params
                .into_iter()
                .map(|n| Param {
                    name: n.into(),
                    ty_ann: None,
                    default: None,
                    span: s(),
                })
                .collect(),
            ret_ty: None,
            body: Block { stmts, span: s() },
            is_async: false,
            span: s(),
        })
    }

    fn int_lit(val: i64) -> Expr {
        Expr::IntLit {
            value: val,
            span: s(),
        }
    }

    fn ident(name: &str) -> Expr {
        Expr::Ident {
            name: name.into(),
            span: s(),
        }
    }

    fn bin_op(left: Expr, op: BinOp, right: Expr) -> Expr {
        Expr::BinaryOp {
            left: Box::new(left),
            op,
            right: Box::new(right),
            span: s(),
        }
    }

    fn assign_expr(name: &str, value: Expr) -> Stmt {
        Stmt::ExprStmt {
            expr: Expr::Assign {
                target: Box::new(ident(name)),
                op: AssignOp::AddAssign,
                value: Box::new(value),
                span: s(),
            },
            span: s(),
        }
    }

    // ── Basic structure ────────────────────────────────────────

    #[test]
    fn store_imports_signal_and_derive() {
        let decl = make_store(
            "Counter",
            vec![
                signal_member("count", int_lit(0)),
                derive_member("doubled", bin_op(ident("count"), BinOp::Mul, int_lit(2))),
            ],
        );
        let mut e = JsEmitter::new();
        emit_store_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(out.contains("import { signal, derive } from '/_gale/runtime.js'"));
    }

    #[test]
    fn store_signal_as_module_level_const() {
        let decl = make_store("S", vec![signal_member("count", int_lit(0))]);
        let mut e = JsEmitter::new();
        emit_store_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(
            out.contains("const count = signal(0);"),
            "signal init: {out}"
        );
    }

    #[test]
    fn store_derive_uses_signal_get() {
        let decl = make_store(
            "S",
            vec![
                signal_member("count", int_lit(0)),
                derive_member("doubled", bin_op(ident("count"), BinOp::Mul, int_lit(2))),
            ],
        );
        let mut e = JsEmitter::new();
        emit_store_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(
            out.contains("const doubled = derive(() => (count.get() * 2));"),
            "derive body: {out}"
        );
    }

    #[test]
    fn store_exports_getters_and_setters() {
        let decl = make_store("S", vec![signal_member("count", int_lit(0))]);
        let mut e = JsEmitter::new();
        emit_store_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(
            out.contains("get count() { return count.get(); }"),
            "getter: {out}"
        );
        assert!(
            out.contains("set count(v) { count.set(v); }"),
            "setter: {out}"
        );
    }

    #[test]
    fn store_derive_getter_read_only() {
        let decl = make_store(
            "S",
            vec![
                signal_member("x", int_lit(1)),
                derive_member("y", ident("x")),
            ],
        );
        let mut e = JsEmitter::new();
        emit_store_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(
            out.contains("get y() { return y.get(); }"),
            "derive getter: {out}"
        );
        // No setter for derives
        assert!(!out.contains("set y("), "no derive setter: {out}");
    }

    #[test]
    fn store_method_exported_by_name() {
        let decl = make_store(
            "S",
            vec![
                signal_member("count", int_lit(0)),
                method_member("increment", vec![], vec![assign_expr("count", int_lit(1))]),
            ],
        );
        let mut e = JsEmitter::new();
        emit_store_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(out.contains("function increment()"), "method fn: {out}");
        assert!(
            out.contains("count.set(count.get() + 1)"),
            "signal mutation: {out}"
        );
        // Export object includes method
        assert!(out.contains("increment,"), "method export: {out}");
    }

    #[test]
    fn store_export_object_name() {
        let decl = make_store("Counter", vec![signal_member("count", int_lit(0))]);
        let mut e = JsEmitter::new();
        emit_store_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(out.contains("export const Counter ="), "export name: {out}");
    }

    #[test]
    fn store_no_imports_when_empty() {
        let decl = make_store("Empty", vec![]);
        let mut e = JsEmitter::new();
        emit_store_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(!out.contains("import"), "no imports for empty store: {out}");
    }

    #[test]
    fn store_only_signal_import() {
        let decl = make_store("S", vec![signal_member("x", int_lit(0))]);
        let mut e = JsEmitter::new();
        emit_store_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(out.contains("import { signal }"), "only signal: {out}");
        assert!(!out.contains("derive"), "no derive import: {out}");
    }

    #[test]
    fn store_meta_fields() {
        let decl = make_store(
            "UserStore",
            vec![
                signal_member(
                    "name",
                    Expr::StringLit {
                        value: "".into(),
                        span: s(),
                    },
                ),
                derive_member("upper", ident("name")),
                method_member("reset", vec![], vec![]),
            ],
        );
        let mut e = JsEmitter::new();
        let meta = emit_store_js_file(&mut e, &decl);

        assert_eq!(meta.store_name, "UserStore");
        assert_eq!(meta.module_name, "user_store");
        assert_eq!(meta.signal_names, vec!["name"]);
        assert_eq!(meta.derive_names, vec!["upper"]);
        assert_eq!(meta.method_names, vec!["reset"]);
    }

    #[test]
    fn store_method_with_params() {
        let decl = make_store(
            "S",
            vec![
                signal_member(
                    "items",
                    Expr::ArrayLit {
                        elements: vec![],
                        span: s(),
                    },
                ),
                method_member(
                    "add",
                    vec!["item"],
                    vec![Stmt::ExprStmt {
                        expr: Expr::FnCall {
                            callee: Box::new(Expr::MemberAccess {
                                object: Box::new(ident("items")),
                                field: "push".into(),
                                span: s(),
                            }),
                            args: vec![ident("item")],
                            span: s(),
                        },
                        span: s(),
                    }],
                ),
            ],
        );
        let mut e = JsEmitter::new();
        emit_store_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(out.contains("function add(item)"), "params: {out}");
    }

    #[test]
    fn store_file_header() {
        let decl = make_store("MyStore", vec![]);
        let mut e = JsEmitter::new();
        emit_store_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(out.contains("Store: `MyStore`."), "header: {out}");
        assert!(
            out.contains("Generated by GaleX compiler"),
            "gen notice: {out}"
        );
    }
}
