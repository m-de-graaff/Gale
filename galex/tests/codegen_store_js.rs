//! Integration tests for store JS singleton code generation (12.7).
//!
//! Tests construct AST nodes directly and verify the generated JavaScript
//! contains expected patterns for singleton store modules.

use galex::ast::*;
use galex::codegen::emit_store_js::{self, StoreJsMeta};
use galex::codegen::js_emitter::JsEmitter;
use galex::codegen::CodegenContext;
use galex::span::Span;
use galex::types::ty::TypeInterner;

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

fn str_lit(val: &str) -> Expr {
    Expr::StringLit {
        value: val.into(),
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

fn make_program(items: Vec<Item>) -> Program {
    Program { items, span: s() }
}

fn emit_store(decl: &StoreDecl) -> (String, StoreJsMeta) {
    let mut e = JsEmitter::new();
    let meta = emit_store_js::emit_store_js_file(&mut e, decl);
    (e.finish(), meta)
}

// ── Full store integration ─────────────────────────────────────────────

#[test]
fn full_counter_store() {
    let decl = make_store(
        "Counter",
        vec![
            signal_member("count", int_lit(0)),
            derive_member("doubled", bin_op(ident("count"), BinOp::Mul, int_lit(2))),
            method_member(
                "increment",
                vec![],
                vec![Stmt::ExprStmt {
                    expr: Expr::Assign {
                        target: Box::new(ident("count")),
                        op: AssignOp::AddAssign,
                        value: Box::new(int_lit(1)),
                        span: s(),
                    },
                    span: s(),
                }],
            ),
            method_member(
                "reset",
                vec![],
                vec![Stmt::ExprStmt {
                    expr: Expr::Assign {
                        target: Box::new(ident("count")),
                        op: AssignOp::Assign,
                        value: Box::new(int_lit(0)),
                        span: s(),
                    },
                    span: s(),
                }],
            ),
        ],
    );
    let (out, meta) = emit_store(&decl);

    // Header
    assert!(out.contains("Store: `Counter`."), "header: {out}");

    // Imports
    assert!(
        out.contains("import { signal, derive } from '/_gale/runtime.js'"),
        "imports: {out}"
    );

    // Signal
    assert!(
        out.contains("const count = signal(0);"),
        "signal init: {out}"
    );

    // Derive
    assert!(
        out.contains("const doubled = derive(() => (count.get() * 2));"),
        "derive body: {out}"
    );

    // Methods
    assert!(
        out.contains("function increment()"),
        "increment method: {out}"
    );
    assert!(
        out.contains("count.set(count.get() + 1)"),
        "increment body: {out}"
    );
    assert!(out.contains("function reset()"), "reset method: {out}");
    assert!(out.contains("count.set(0)"), "reset body: {out}");

    // Export
    assert!(out.contains("export const Counter ="), "export: {out}");
    assert!(out.contains("get count()"), "getter: {out}");
    assert!(out.contains("set count(v)"), "setter: {out}");
    assert!(out.contains("get doubled()"), "derive getter: {out}");
    assert!(!out.contains("set doubled("), "no derive setter: {out}");
    assert!(out.contains("increment,"), "method ref: {out}");
    assert!(out.contains("reset,"), "method ref: {out}");

    // Meta
    assert_eq!(meta.store_name, "Counter");
    assert_eq!(meta.module_name, "counter");
    assert_eq!(meta.signal_names, vec!["count"]);
    assert_eq!(meta.derive_names, vec!["doubled"]);
    assert_eq!(meta.method_names, vec!["increment", "reset"]);
}

// ── CodegenContext integration ──────────────────────────────────────────

#[test]
fn codegen_emits_store_js_file() {
    let interner = TypeInterner::new();
    let program = make_program(vec![Item::StoreDecl(make_store(
        "AppState",
        vec![signal_member("count", int_lit(0))],
    ))]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    assert!(
        ctx.files.contains("public/_gale/stores/app_state.js"),
        "should emit store JS file"
    );
    let js = ctx.files.get("public/_gale/stores/app_state.js").unwrap();
    assert!(js.contains("export const AppState"), "store export: {js}");
}

#[test]
fn codegen_store_triggers_runtime_emission() {
    let interner = TypeInterner::new();
    let program = make_program(vec![Item::StoreDecl(make_store(
        "S",
        vec![signal_member("x", int_lit(0))],
    ))]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    // Store sets has_client_code, so the runtime should be emitted
    assert!(
        ctx.files.contains("public/_gale/runtime.js"),
        "should emit runtime.js when stores exist"
    );
}

#[test]
fn codegen_multiple_stores() {
    let interner = TypeInterner::new();
    let program = make_program(vec![
        Item::StoreDecl(make_store("A", vec![signal_member("x", int_lit(1))])),
        Item::StoreDecl(make_store("B", vec![signal_member("y", int_lit(2))])),
    ]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    assert!(ctx.files.contains("public/_gale/stores/a.js"));
    assert!(ctx.files.contains("public/_gale/stores/b.js"));
}

#[test]
fn codegen_no_store_files_without_stores() {
    let interner = TypeInterner::new();
    let program = make_program(vec![Item::ActionDecl(ActionDecl {
        name: "doSomething".into(),
        params: vec![],
        ret_ty: None,
        body: Block {
            stmts: vec![],
            span: s(),
        },
        span: s(),
    })]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    // No store files should exist
    let store_files: Vec<_> = ctx
        .files
        .iter()
        .filter(|(p, _)| p.to_string_lossy().contains("stores"))
        .collect();
    assert!(store_files.is_empty(), "no store files should exist");
}

// ── Singleton guarantee ────────────────────────────────────────────────

#[test]
fn store_signals_are_module_level() {
    let decl = make_store(
        "S",
        vec![
            signal_member("count", int_lit(0)),
            signal_member("name", str_lit("")),
        ],
    );
    let (out, _) = emit_store(&decl);

    // Signals should be at module level (not inside a function)
    // This means they appear before the export and without indentation
    let lines: Vec<&str> = out.lines().collect();
    let count_line = lines
        .iter()
        .find(|l| l.contains("const count = signal("))
        .unwrap();
    let name_line = lines
        .iter()
        .find(|l| l.contains("const name = signal("))
        .unwrap();

    // Not indented = module level
    assert!(
        !count_line.starts_with(' '),
        "count should be module-level: {count_line}"
    );
    assert!(
        !name_line.starts_with(' '),
        "name should be module-level: {name_line}"
    );
}

// ── Method body conversion ─────────────────────────────────────────────

#[test]
fn store_method_signal_mutation() {
    let decl = make_store(
        "S",
        vec![
            signal_member("count", int_lit(0)),
            method_member(
                "add",
                vec!["n"],
                vec![Stmt::ExprStmt {
                    expr: Expr::Assign {
                        target: Box::new(ident("count")),
                        op: AssignOp::AddAssign,
                        value: Box::new(ident("n")),
                        span: s(),
                    },
                    span: s(),
                }],
            ),
        ],
    );
    let (out, _) = emit_store(&decl);

    assert!(out.contains("function add(n)"), "method params: {out}");
    assert!(
        out.contains("count.set(count.get() + n)"),
        "signal +=: {out}"
    );
}

#[test]
fn store_method_reads_derive() {
    let decl = make_store(
        "S",
        vec![
            signal_member("x", int_lit(0)),
            derive_member("y", bin_op(ident("x"), BinOp::Add, int_lit(1))),
            method_member(
                "getY",
                vec![],
                vec![Stmt::Return {
                    value: Some(ident("y")),
                    span: s(),
                }],
            ),
        ],
    );
    let (out, _) = emit_store(&decl);

    // Method should read derive via .get()
    assert!(out.contains("return y.get();"), "derive read: {out}");
}

#[test]
fn store_empty() {
    let decl = make_store("Empty", vec![]);
    let (out, meta) = emit_store(&decl);

    assert!(out.contains("export const Empty ="), "export: {out}");
    assert!(meta.signal_names.is_empty());
    assert!(meta.derive_names.is_empty());
    assert!(meta.method_names.is_empty());
}
