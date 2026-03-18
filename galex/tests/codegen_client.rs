//! Integration tests for client-side code generation (Phase 12.1).

use std::collections::HashSet;

use galex::ast::*;
use galex::codegen::emit_client;
use galex::codegen::js_expr::expr_to_js;
use galex::codegen::CodegenContext;
use galex::span::Span;
use galex::types::ty::TypeInterner;

fn s() -> Span {
    Span::dummy()
}

fn make_program(items: Vec<Item>) -> Program {
    Program { items, span: s() }
}

fn sigs(names: &[&str]) -> HashSet<String> {
    names.iter().map(|n| n.to_string()).collect()
}

/// Helper to make an interactive component (has a signal + template with bind)
fn interactive_component(name: &str) -> ComponentDecl {
    ComponentDecl {
        name: name.into(),
        props: vec![],
        body: ComponentBody {
            stmts: vec![Stmt::Signal {
                name: "count".into(),
                ty_ann: None,
                init: Expr::IntLit {
                    value: 0,
                    span: s(),
                },
                span: s(),
            }],
            template: vec![TemplateNode::SelfClosing {
                tag: "input".into(),
                attributes: vec![],
                directives: vec![Directive::Bind {
                    field: "count".into(),
                    span: s(),
                }],
                span: s(),
            }],
            head: None,
            span: s(),
        },
        span: s(),
    }
}

/// Helper to make a static component (no signals, no interactive directives)
fn static_component(name: &str) -> ComponentDecl {
    ComponentDecl {
        name: name.into(),
        props: vec![],
        body: ComponentBody {
            stmts: vec![],
            template: vec![TemplateNode::Text {
                value: "Hello".into(),
                span: s(),
            }],
            head: None,
            span: s(),
        },
        span: s(),
    }
}

// ── Runtime output ─────────────────────────────────────────────────────

#[test]
fn runtime_js_written_to_output() {
    let interner = TypeInterner::new();
    let program = make_program(vec![Item::ComponentDecl(interactive_component("Page"))]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    assert!(
        ctx.files.contains("public/_gale/runtime.js"),
        "should write runtime.js"
    );
}

#[test]
fn runtime_contains_signal_export() {
    let interner = TypeInterner::new();
    let program = make_program(vec![Item::ComponentDecl(interactive_component("Page"))]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    let rt = ctx.files.get("public/_gale/runtime.js").unwrap();
    assert!(
        rt.contains("export function signal"),
        "runtime should export signal: {rt}"
    );
}

#[test]
fn runtime_contains_derive_export() {
    let interner = TypeInterner::new();
    let program = make_program(vec![Item::ComponentDecl(interactive_component("Page"))]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    let rt = ctx.files.get("public/_gale/runtime.js").unwrap();
    assert!(
        rt.contains("export function derive"),
        "should export derive"
    );
}

#[test]
fn runtime_contains_effect_export() {
    let interner = TypeInterner::new();
    let program = make_program(vec![Item::ComponentDecl(interactive_component("Page"))]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    let rt = ctx.files.get("public/_gale/runtime.js").unwrap();
    assert!(
        rt.contains("export function effect"),
        "should export effect"
    );
}

#[test]
fn runtime_contains_hydrate_export() {
    let interner = TypeInterner::new();
    let program = make_program(vec![Item::ComponentDecl(interactive_component("Page"))]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    let rt = ctx.files.get("public/_gale/runtime.js").unwrap();
    assert!(
        rt.contains("export function hydrate"),
        "should export hydrate"
    );
}

#[test]
fn runtime_contains_bind_export() {
    let interner = TypeInterner::new();
    let program = make_program(vec![Item::ComponentDecl(interactive_component("Page"))]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    let rt = ctx.files.get("public/_gale/runtime.js").unwrap();
    assert!(rt.contains("export function bind"), "should export bind");
}

#[test]
fn runtime_contains_action_export() {
    let interner = TypeInterner::new();
    let program = make_program(vec![Item::ComponentDecl(interactive_component("Page"))]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    let rt = ctx.files.get("public/_gale/runtime.js").unwrap();
    assert!(
        rt.contains("export function action"),
        "should export action"
    );
}

#[test]
fn runtime_contains_batch_export() {
    let interner = TypeInterner::new();
    let program = make_program(vec![Item::ComponentDecl(interactive_component("Page"))]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    let rt = ctx.files.get("public/_gale/runtime.js").unwrap();
    assert!(rt.contains("export function batch"), "should export batch");
}

// ── Page script generation ─────────────────────────────────────────────

#[test]
fn page_script_generated_for_interactive_component() {
    let interner = TypeInterner::new();
    let program = make_program(vec![Item::ComponentDecl(interactive_component("Counter"))]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    assert!(
        ctx.files.contains("public/_gale/pages/counter.js"),
        "should generate per-page script"
    );
}

#[test]
fn no_page_script_for_static_component() {
    let interner = TypeInterner::new();
    let program = make_program(vec![Item::ComponentDecl(static_component("About"))]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    assert!(
        !ctx.files.contains("public/_gale/pages/about.js"),
        "static components should not get page scripts"
    );
}

#[test]
fn no_runtime_for_static_only_project() {
    let interner = TypeInterner::new();
    let program = make_program(vec![Item::ComponentDecl(static_component("About"))]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    assert!(
        !ctx.files.contains("public/_gale/runtime.js"),
        "static-only projects should not include runtime"
    );
}

#[test]
fn page_script_imports_from_runtime() {
    let js = emit_client::emit_page_script(&interactive_component("Page"));
    assert!(
        js.contains("from '/_gale/runtime.js'"),
        "should import from runtime: {js}"
    );
}

#[test]
fn page_script_reads_gale_data() {
    let js = emit_client::emit_page_script(&interactive_component("Page"));
    assert!(
        js.contains("_readData()"),
        "should read server data via _readData: {js}"
    );
}

#[test]
fn page_script_creates_signal() {
    let js = emit_client::emit_page_script(&interactive_component("Page"));
    assert!(
        js.contains("const count = signal("),
        "should create signal: {js}"
    );
}

#[test]
fn page_script_creates_derive() {
    let decl = ComponentDecl {
        name: "Page".into(),
        props: vec![],
        body: ComponentBody {
            stmts: vec![
                Stmt::Signal {
                    name: "count".into(),
                    ty_ann: None,
                    init: Expr::IntLit {
                        value: 0,
                        span: s(),
                    },
                    span: s(),
                },
                Stmt::Derive {
                    name: "doubled".into(),
                    init: Expr::BinaryOp {
                        left: Box::new(Expr::Ident {
                            name: "count".into(),
                            span: s(),
                        }),
                        op: BinOp::Mul,
                        right: Box::new(Expr::IntLit {
                            value: 2,
                            span: s(),
                        }),
                        span: s(),
                    },
                    span: s(),
                },
            ],
            template: vec![],
            head: None,
            span: s(),
        },
        span: s(),
    };
    let js = emit_client::emit_page_script(&decl);
    assert!(
        js.contains("const doubled = derive("),
        "should create derive: {js}"
    );
    assert!(
        js.contains("count.get()"),
        "derive body should use signal .get(): {js}"
    );
}

#[test]
fn page_script_creates_effect() {
    let decl = ComponentDecl {
        name: "Page".into(),
        props: vec![],
        body: ComponentBody {
            stmts: vec![Stmt::Effect {
                body: Block {
                    stmts: vec![Stmt::ExprStmt {
                        expr: Expr::IntLit {
                            value: 1,
                            span: s(),
                        },
                        span: s(),
                    }],
                    span: s(),
                },
                cleanup: None,
                span: s(),
            }],
            template: vec![],
            head: None,
            span: s(),
        },
        span: s(),
    };
    let js = emit_client::emit_page_script(&decl);
    assert!(js.contains("effect(()"), "should create effect: {js}");
}

#[test]
fn page_script_bind_instruction() {
    let js = emit_client::emit_page_script(&interactive_component("Page"));
    assert!(
        js.contains("bind(el, count)"),
        "should generate bind instruction: {js}"
    );
}

#[test]
fn page_script_event_instruction() {
    let decl = ComponentDecl {
        name: "Page".into(),
        props: vec![],
        body: ComponentBody {
            stmts: vec![Stmt::Signal {
                name: "x".into(),
                ty_ann: None,
                init: Expr::IntLit {
                    value: 0,
                    span: s(),
                },
                span: s(),
            }],
            template: vec![TemplateNode::Element {
                tag: "button".into(),
                attributes: vec![],
                directives: vec![Directive::On {
                    event: "click".into(),
                    modifiers: vec![],
                    handler: Expr::Ident {
                        name: "handleClick".into(),
                        span: s(),
                    },
                    span: s(),
                }],
                children: vec![],
                span: s(),
            }],
            head: None,
            span: s(),
        },
        span: s(),
    };
    let js = emit_client::emit_page_script(&decl);
    assert!(
        js.contains("addEventListener(\"click\""),
        "should generate event instruction: {js}"
    );
}

#[test]
fn page_script_class_instruction() {
    let decl = ComponentDecl {
        name: "Page".into(),
        props: vec![],
        body: ComponentBody {
            stmts: vec![Stmt::Signal {
                name: "active".into(),
                ty_ann: None,
                init: Expr::BoolLit {
                    value: true,
                    span: s(),
                },
                span: s(),
            }],
            template: vec![TemplateNode::Element {
                tag: "div".into(),
                attributes: vec![],
                directives: vec![Directive::Class {
                    name: "active".into(),
                    condition: Expr::Ident {
                        name: "active".into(),
                        span: s(),
                    },
                    span: s(),
                }],
                children: vec![],
                span: s(),
            }],
            head: None,
            span: s(),
        },
        span: s(),
    };
    let js = emit_client::emit_page_script(&decl);
    assert!(
        js.contains("classList.toggle"),
        "should generate class toggle: {js}"
    );
}

#[test]
fn page_script_ref_instruction() {
    let decl = ComponentDecl {
        name: "Page".into(),
        props: vec![],
        body: ComponentBody {
            stmts: vec![Stmt::RefDecl {
                name: "canvas".into(),
                ty_ann: TypeAnnotation::Named {
                    name: "HTMLCanvasElement".into(),
                    span: s(),
                },
                span: s(),
            }],
            template: vec![TemplateNode::Element {
                tag: "canvas".into(),
                attributes: vec![],
                directives: vec![Directive::Ref {
                    name: "canvas".into(),
                    span: s(),
                }],
                children: vec![],
                span: s(),
            }],
            head: None,
            span: s(),
        },
        span: s(),
    };
    let js = emit_client::emit_page_script(&decl);
    assert!(
        js.contains("canvas = el"),
        "should generate ref assignment: {js}"
    );
}

#[test]
fn page_script_text_expr_instruction() {
    let decl = ComponentDecl {
        name: "Page".into(),
        props: vec![],
        body: ComponentBody {
            stmts: vec![Stmt::Signal {
                name: "name".into(),
                ty_ann: None,
                init: Expr::StringLit {
                    value: "world".into(),
                    span: s(),
                },
                span: s(),
            }],
            template: vec![TemplateNode::ExprInterp {
                expr: Expr::Ident {
                    name: "name".into(),
                    span: s(),
                },
                span: s(),
            }],
            head: None,
            span: s(),
        },
        span: s(),
    };
    let js = emit_client::emit_page_script(&decl);
    assert!(
        js.contains("el.textContent"),
        "should generate text expr update: {js}"
    );
    assert!(
        js.contains("name.get()"),
        "text expr should read signal: {js}"
    );
}

#[test]
fn page_script_only_imports_used_features() {
    // Component with only a signal and no bind/action/transition/etc.
    let decl = ComponentDecl {
        name: "Page".into(),
        props: vec![],
        body: ComponentBody {
            stmts: vec![Stmt::Signal {
                name: "x".into(),
                ty_ann: None,
                init: Expr::IntLit {
                    value: 0,
                    span: s(),
                },
                span: s(),
            }],
            template: vec![TemplateNode::ExprInterp {
                expr: Expr::Ident {
                    name: "x".into(),
                    span: s(),
                },
                span: s(),
            }],
            head: None,
            span: s(),
        },
        span: s(),
    };
    let js = emit_client::emit_page_script(&decl);
    assert!(js.contains("signal"), "should import signal: {js}");
    assert!(js.contains("effect"), "should import effect: {js}");
    assert!(
        !js.contains("bind"),
        "should NOT import bind (not used): {js}"
    );
    assert!(
        !js.contains("action"),
        "should NOT import action (not used): {js}"
    );
    assert!(
        !js.contains("transition"),
        "should NOT import transition (not used): {js}"
    );
}

// ── Script tag injection ───────────────────────────────────────────────

#[test]
fn route_includes_page_script_tag() {
    let interner = TypeInterner::new();
    let program = make_program(vec![Item::ComponentDecl(interactive_component("Counter"))]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    let route = ctx.files.get("src/routes/counter.rs").unwrap();
    assert!(
        route.contains("_gale/pages/counter.js"),
        "route should include page script reference: {route}"
    );
    assert!(
        route.contains("_gale/runtime.js"),
        "route should include runtime script reference: {route}"
    );
    // Paths should be resolved through the asset manifest
    assert!(
        route.contains("asset_manifest::resolve"),
        "route should use asset manifest for path resolution: {route}"
    );
}

#[test]
fn static_route_no_script_tags() {
    let interner = TypeInterner::new();
    let program = make_program(vec![Item::ComponentDecl(static_component("About"))]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    let route = ctx.files.get("src/routes/about.rs").unwrap();
    assert!(
        !route.contains("asset_manifest::resolve"),
        "static route should NOT include script tags: {route}"
    );
}

// ── JS expression converter ────────────────────────────────────────────

#[test]
fn js_expr_ident() {
    let result = expr_to_js(
        &Expr::Ident {
            name: "userName".into(),
            span: s(),
        },
        &sigs(&[]),
    );
    assert_eq!(result, "userName");
}

#[test]
fn js_expr_signal_read() {
    let result = expr_to_js(
        &Expr::Ident {
            name: "count".into(),
            span: s(),
        },
        &sigs(&["count"]),
    );
    assert_eq!(result, "count.get()");
}

#[test]
fn js_expr_binary_eq() {
    let result = expr_to_js(
        &Expr::BinaryOp {
            left: Box::new(Expr::Ident {
                name: "x".into(),
                span: s(),
            }),
            op: BinOp::Eq,
            right: Box::new(Expr::IntLit {
                value: 1,
                span: s(),
            }),
            span: s(),
        },
        &sigs(&[]),
    );
    assert!(
        result.contains("==="),
        "GaleX == should map to JS ===: {result}"
    );
}

#[test]
fn js_expr_template_lit() {
    let result = expr_to_js(
        &Expr::TemplateLit {
            parts: vec![
                TemplatePart::Text("Hello ".into()),
                TemplatePart::Expr(Expr::Ident {
                    name: "name".into(),
                    span: s(),
                }),
            ],
            span: s(),
        },
        &sigs(&[]),
    );
    assert!(
        result.starts_with('`'),
        "should be template literal: {result}"
    );
    assert!(result.contains("${name}"), "should interpolate: {result}");
}

#[test]
fn js_expr_null_coalesce() {
    let result = expr_to_js(
        &Expr::NullCoalesce {
            left: Box::new(Expr::Ident {
                name: "x".into(),
                span: s(),
            }),
            right: Box::new(Expr::IntLit {
                value: 0,
                span: s(),
            }),
            span: s(),
        },
        &sigs(&[]),
    );
    assert!(
        result.contains("??"),
        "GaleX ?? should map to JS ??: {result}"
    );
}

#[test]
fn js_expr_signal_assign() {
    let result = expr_to_js(
        &Expr::Assign {
            target: Box::new(Expr::Ident {
                name: "count".into(),
                span: s(),
            }),
            op: AssignOp::Assign,
            value: Box::new(Expr::IntLit {
                value: 5,
                span: s(),
            }),
            span: s(),
        },
        &sigs(&["count"]),
    );
    assert!(
        result.contains("count.set(5)"),
        "signal assignment should use .set(): {result}"
    );
}

#[test]
fn js_expr_signal_add_assign() {
    let result = expr_to_js(
        &Expr::Assign {
            target: Box::new(Expr::Ident {
                name: "count".into(),
                span: s(),
            }),
            op: AssignOp::AddAssign,
            value: Box::new(Expr::IntLit {
                value: 1,
                span: s(),
            }),
            span: s(),
        },
        &sigs(&["count"]),
    );
    assert!(
        result.contains("count.set(count.get() + 1)"),
        "signal += should use .set(.get() +): {result}"
    );
}

#[test]
fn js_expr_env_access() {
    let result = expr_to_js(
        &Expr::EnvAccess {
            key: "PUBLIC_API".into(),
            span: s(),
        },
        &sigs(&[]),
    );
    assert!(
        result.contains("$env"),
        "env access should read from $env: {result}"
    );
    assert!(
        result.contains("PUBLIC_API"),
        "should include key: {result}"
    );
}
