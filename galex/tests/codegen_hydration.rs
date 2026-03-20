//! Integration tests for hydration code generation (Phase 12.2).
//!
//! Tests construct AST nodes directly and verify the generated JavaScript
//! hydration scripts contain expected patterns for signals, derives,
//! effects, watches, event handlers, bindings, when/each blocks, and
//! correct reactive scope closures.

use galex::ast::*;
use galex::codegen::emit_client;
use galex::codegen::emit_client_runtime;
use galex::codegen::hydration::HydrationCtx;
use galex::codegen::rust_emitter::RustEmitter;
use galex::codegen::ssr;
use galex::span::Span;

fn s() -> Span {
    Span::dummy()
}

// ── Helper: build a component with stmts and template ──────────────────

fn make_component(name: &str, stmts: Vec<Stmt>, template: Vec<TemplateNode>) -> ComponentDecl {
    ComponentDecl {
        name: name.into(),
        props: vec![],
        body: ComponentBody {
            stmts,
            template,
            head: None,
            span: s(),
        },
        span: s(),
    }
}

// ═══════════════════════════════════════════════════════════════════════
// Component detection
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn component_with_signal_is_interactive() {
    let decl = make_component(
        "Counter",
        vec![Stmt::Signal {
            name: "count".into(),
            ty_ann: None,
            init: Expr::IntLit {
                value: 0,
                span: s(),
            },
            span: s(),
        }],
        vec![],
    );
    assert!(emit_client::component_has_client_code(&decl));
}

#[test]
fn component_with_derive_is_interactive() {
    let decl = make_component(
        "Calc",
        vec![Stmt::Derive {
            name: "total".into(),
            init: Expr::IntLit {
                value: 0,
                span: s(),
            },
            span: s(),
        }],
        vec![],
    );
    assert!(emit_client::component_has_client_code(&decl));
}

#[test]
fn component_with_effect_is_interactive() {
    let decl = make_component(
        "Logger",
        vec![Stmt::Effect {
            body: Block {
                stmts: vec![],
                span: s(),
            },
            cleanup: None,
            span: s(),
        }],
        vec![],
    );
    assert!(emit_client::component_has_client_code(&decl));
}

#[test]
fn component_with_watch_is_interactive() {
    let decl = make_component(
        "Watcher",
        vec![Stmt::Watch {
            target: Expr::Ident {
                name: "x".into(),
                span: s(),
            },
            next_name: "n".into(),
            prev_name: "p".into(),
            body: Block {
                stmts: vec![],
                span: s(),
            },
            span: s(),
        }],
        vec![],
    );
    assert!(emit_client::component_has_client_code(&decl));
}

#[test]
fn component_with_on_directive_is_interactive() {
    let decl = make_component(
        "Btn",
        vec![],
        vec![TemplateNode::Element {
            tag: "button".into(),
            attributes: vec![],
            directives: vec![Directive::On {
                event: "click".into(),
                modifiers: vec![],
                handler: Expr::ArrowFn {
                    params: vec![],
                    ret_ty: None,
                    body: ArrowBody::Expr(Box::new(Expr::NullLit { span: s() })),
                    span: s(),
                },
                span: s(),
            }],
            children: vec![],
            span: s(),
        }],
    );
    assert!(emit_client::component_has_client_code(&decl));
}

#[test]
fn component_with_when_block_is_interactive() {
    let decl = make_component(
        "Cond",
        vec![],
        vec![TemplateNode::When {
            condition: Expr::BoolLit {
                value: true,
                span: s(),
            },
            body: vec![TemplateNode::Text {
                value: "yes".into(),
                span: s(),
            }],
            else_branch: None,
            span: s(),
        }],
    );
    assert!(emit_client::component_has_client_code(&decl));
}

#[test]
fn component_with_each_block_is_interactive() {
    let decl = make_component(
        "List",
        vec![],
        vec![TemplateNode::Each {
            binding: "item".into(),
            index: None,
            iterable: Expr::Ident {
                name: "items".into(),
                span: s(),
            },
            body: vec![TemplateNode::Text {
                value: "item".into(),
                span: s(),
            }],
            empty: None,
            span: s(),
        }],
    );
    assert!(emit_client::component_has_client_code(&decl));
}

#[test]
fn static_component_is_not_interactive() {
    let decl = make_component(
        "Static",
        vec![],
        vec![TemplateNode::Text {
            value: "Hello".into(),
            span: s(),
        }],
    );
    assert!(!emit_client::component_has_client_code(&decl));
}

// ═══════════════════════════════════════════════════════════════════════
// Signal generation
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn signal_creates_signal_with_server_data() {
    let decl = make_component(
        "Counter",
        vec![Stmt::Signal {
            name: "count".into(),
            ty_ann: None,
            init: Expr::IntLit {
                value: 0,
                span: s(),
            },
            span: s(),
        }],
        vec![],
    );
    let js = emit_client::emit_page_script(&decl);
    assert!(js.contains("import {"), "should have import");
    assert!(js.contains("signal"), "should import signal");
    assert!(
        js.contains("const count = signal($data.data?.count ?? 0)"),
        "should create signal from server data: {js}"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Derive generation
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn derive_creates_derive() {
    let decl = make_component(
        "Calc",
        vec![
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
        vec![],
    );
    let js = emit_client::emit_page_script(&decl);
    assert!(js.contains("derive"), "should import derive");
    assert!(
        js.contains("const doubled = derive(() => (count.get() * 2))"),
        "should create derive with signal .get(): {js}"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Effect generation
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn effect_emits_effect_call() {
    let decl = make_component(
        "Logger",
        vec![
            Stmt::Signal {
                name: "count".into(),
                ty_ann: None,
                init: Expr::IntLit {
                    value: 0,
                    span: s(),
                },
                span: s(),
            },
            Stmt::Effect {
                body: Block {
                    stmts: vec![Stmt::ExprStmt {
                        expr: Expr::FnCall {
                            callee: Box::new(Expr::MemberAccess {
                                object: Box::new(Expr::Ident {
                                    name: "console".into(),
                                    span: s(),
                                }),
                                field: "log".into(),
                                span: s(),
                            }),
                            args: vec![Expr::Ident {
                                name: "count".into(),
                                span: s(),
                            }],
                            span: s(),
                        },
                        span: s(),
                    }],
                    span: s(),
                },
                cleanup: None,
                span: s(),
            },
        ],
        vec![],
    );
    let js = emit_client::emit_page_script(&decl);
    assert!(js.contains("effect(() => {"), "should emit effect: {js}");
    assert!(
        js.contains("console.log(count.get())"),
        "should use .get() inside effect: {js}"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Watch generation
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn watch_emits_watch_call() {
    let decl = make_component(
        "Watcher",
        vec![
            Stmt::Signal {
                name: "count".into(),
                ty_ann: None,
                init: Expr::IntLit {
                    value: 0,
                    span: s(),
                },
                span: s(),
            },
            Stmt::Watch {
                target: Expr::Ident {
                    name: "count".into(),
                    span: s(),
                },
                next_name: "next".into(),
                prev_name: "prev".into(),
                body: Block {
                    stmts: vec![],
                    span: s(),
                },
                span: s(),
            },
        ],
        vec![],
    );
    let js = emit_client::emit_page_script(&decl);
    assert!(js.contains("watch"), "should import watch");
    assert!(
        js.contains("watch(() => count.get()"),
        "should watch signal: {js}"
    );
    assert!(
        js.contains("(next, prev) =>"),
        "should have next/prev params: {js}"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Event handler generation
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn on_click_handler_attaches_event() {
    let decl = make_component(
        "Btn",
        vec![Stmt::Signal {
            name: "count".into(),
            ty_ann: None,
            init: Expr::IntLit {
                value: 0,
                span: s(),
            },
            span: s(),
        }],
        vec![TemplateNode::Element {
            tag: "button".into(),
            attributes: vec![],
            directives: vec![Directive::On {
                event: "click".into(),
                modifiers: vec![],
                handler: Expr::ArrowFn {
                    params: vec![],
                    ret_ty: None,
                    body: ArrowBody::Expr(Box::new(Expr::Assign {
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
                    })),
                    span: s(),
                },
                span: s(),
            }],
            children: vec![],
            span: s(),
        }],
    );
    let js = emit_client::emit_page_script(&decl);
    assert!(js.contains("hydrate"), "should import hydrate");
    assert!(
        js.contains("addEventListener(\"click\""),
        "should attach click event: {js}"
    );
}

#[test]
fn on_click_prevent_applies_modifier() {
    let decl = make_component(
        "Link",
        vec![],
        vec![TemplateNode::Element {
            tag: "a".into(),
            attributes: vec![],
            directives: vec![Directive::On {
                event: "click".into(),
                modifiers: vec!["prevent".into()],
                handler: Expr::ArrowFn {
                    params: vec![],
                    ret_ty: None,
                    body: ArrowBody::Expr(Box::new(Expr::NullLit { span: s() })),
                    span: s(),
                },
                span: s(),
            }],
            children: vec![],
            span: s(),
        }],
    );
    let js = emit_client::emit_page_script(&decl);
    assert!(
        js.contains("e.preventDefault()"),
        "should apply prevent modifier: {js}"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Bind directive generation
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn bind_directive_sets_up_two_way_binding() {
    let decl = make_component(
        "Form",
        vec![Stmt::Signal {
            name: "name".into(),
            ty_ann: None,
            init: Expr::StringLit {
                value: "".into(),
                span: s(),
            },
            span: s(),
        }],
        vec![TemplateNode::SelfClosing {
            tag: "input".into(),
            attributes: vec![],
            directives: vec![Directive::Bind {
                field: "value".into(),
                expr: Some(Box::new(Expr::Ident {
                    name: "name".into(),
                    span: s(),
                })),
                span: s(),
            }],
            span: s(),
        }],
    );
    let js = emit_client::emit_page_script(&decl);
    assert!(js.contains("bind"), "should import bind");
    assert!(
        js.contains("el => bind(el, name)"),
        "should set up two-way binding: {js}"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// When block generation
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn when_block_emits_replace_region() {
    let decl = make_component(
        "Conditional",
        vec![Stmt::Signal {
            name: "show".into(),
            ty_ann: None,
            init: Expr::BoolLit {
                value: true,
                span: s(),
            },
            span: s(),
        }],
        vec![TemplateNode::When {
            condition: Expr::Ident {
                name: "show".into(),
                span: s(),
            },
            body: vec![TemplateNode::Text {
                value: "Visible".into(),
                span: s(),
            }],
            else_branch: Some(WhenElse::Else(vec![TemplateNode::Text {
                value: "Hidden".into(),
                span: s(),
            }])),
            span: s(),
        }],
    );
    let js = emit_client::emit_page_script(&decl);
    assert!(
        js.contains("replaceRegion"),
        "should import replaceRegion: {js}"
    );
    assert!(
        js.contains("effect(() => replaceRegion("),
        "should set up replaceRegion effect: {js}"
    );
    assert!(
        js.contains("show.get()"),
        "should use signal .get() in condition: {js}"
    );
}

#[test]
fn when_block_ssr_emits_comment_markers() {
    let mut e = RustEmitter::new();
    let mut h = HydrationCtx::new();
    let nodes = vec![TemplateNode::When {
        condition: Expr::BoolLit {
            value: true,
            span: s(),
        },
        body: vec![TemplateNode::Text {
            value: "yes".into(),
            span: s(),
        }],
        else_branch: None,
        span: s(),
    }];
    ssr::emit_template_nodes(&mut e, &nodes, &mut h, None);
    let output = e.finish();
    assert!(
        output.contains("gx-when:"),
        "should emit gx-when comment marker: {output}"
    );
    assert!(
        output.contains("/gx-when:"),
        "should emit closing gx-when marker: {output}"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Each block generation
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn each_block_emits_reconcile_list() {
    let decl = make_component(
        "TodoList",
        vec![Stmt::Signal {
            name: "items".into(),
            ty_ann: None,
            init: Expr::ArrayLit {
                elements: vec![],
                span: s(),
            },
            span: s(),
        }],
        vec![TemplateNode::Each {
            binding: "item".into(),
            index: None,
            iterable: Expr::Ident {
                name: "items".into(),
                span: s(),
            },
            body: vec![TemplateNode::Element {
                tag: "li".into(),
                attributes: vec![],
                directives: vec![],
                children: vec![TemplateNode::ExprInterp {
                    expr: Expr::Ident {
                        name: "item".into(),
                        span: s(),
                    },
                    span: s(),
                }],
                span: s(),
            }],
            empty: None,
            span: s(),
        }],
    );
    let js = emit_client::emit_page_script(&decl);
    assert!(
        js.contains("reconcileList"),
        "should import reconcileList: {js}"
    );
    assert!(
        js.contains("effect(() => reconcileList("),
        "should set up reconcileList effect: {js}"
    );
    assert!(
        js.contains("items.get()"),
        "should use signal .get() for iterable: {js}"
    );
}

#[test]
fn each_block_ssr_emits_comment_markers() {
    let mut e = RustEmitter::new();
    let mut h = HydrationCtx::new();
    let nodes = vec![TemplateNode::Each {
        binding: "item".into(),
        index: None,
        iterable: Expr::Ident {
            name: "items".into(),
            span: s(),
        },
        body: vec![TemplateNode::Text {
            value: "x".into(),
            span: s(),
        }],
        empty: None,
        span: s(),
    }];
    ssr::emit_template_nodes(&mut e, &nodes, &mut h, None);
    let output = e.finish();
    assert!(
        output.contains("gx-each:"),
        "should emit gx-each comment marker: {output}"
    );
    assert!(
        output.contains("/gx-each:"),
        "should emit closing gx-each marker: {output}"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Class toggle generation
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn class_toggle_creates_effect() {
    let decl = make_component(
        "Toggle",
        vec![Stmt::Signal {
            name: "active".into(),
            ty_ann: None,
            init: Expr::BoolLit {
                value: false,
                span: s(),
            },
            span: s(),
        }],
        vec![TemplateNode::Element {
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
    );
    let js = emit_client::emit_page_script(&decl);
    assert!(
        js.contains("classList.toggle(\"active\""),
        "should toggle active class: {js}"
    );
    assert!(
        js.contains("active.get()"),
        "should read signal in class toggle: {js}"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Ref directive generation
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn ref_directive_assigns_element() {
    let decl = make_component(
        "Input",
        vec![Stmt::RefDecl {
            name: "inputEl".into(),
            ty_ann: TypeAnnotation::Named {
                name: "HTMLInputElement".into(),
                span: s(),
            },
            span: s(),
        }],
        vec![TemplateNode::SelfClosing {
            tag: "input".into(),
            attributes: vec![],
            directives: vec![Directive::Ref {
                name: "inputEl".into(),
                span: s(),
            }],
            span: s(),
        }],
    );
    let js = emit_client::emit_page_script(&decl);
    assert!(
        js.contains("let inputEl = null"),
        "should declare ref variable: {js}"
    );
    assert!(
        js.contains("inputEl = el"),
        "should assign element to ref: {js}"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Hydration data attributes
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn hydration_uses_data_gx_id() {
    let mut e = RustEmitter::new();
    let mut h = HydrationCtx::new();
    let nodes = vec![TemplateNode::Element {
        tag: "button".into(),
        attributes: vec![],
        directives: vec![Directive::On {
            event: "click".into(),
            modifiers: vec![],
            handler: Expr::NullLit { span: s() },
            span: s(),
        }],
        children: vec![],
        span: s(),
    }];
    ssr::emit_template_nodes(&mut e, &nodes, &mut h, None);
    let output = e.finish();
    assert!(
        output.contains("data-gx-id"),
        "should use data-gx-id attribute: {output}"
    );
    assert!(
        !output.contains("data-gale-id"),
        "should NOT use old data-gale-id: {output}"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Hydration data script
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn hydration_script_emitted_with_markers() {
    let mut h = HydrationCtx::new();
    h.mark_event("click", &[]);
    h.mark_bind("value");
    h.add_server_data("count");

    let mut e = RustEmitter::new();
    h.emit_script(&mut e);
    let output = e.finish();

    assert!(
        output.contains("gale-data"),
        "should emit gale-data script tag: {output}"
    );
    assert!(
        output.contains("markers"),
        "should have markers key: {output}"
    );
    assert!(
        output.contains("server_data"),
        "should have server data: {output}"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// Runtime exports
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn runtime_exports_all_hydration_primitives() {
    let js = emit_client_runtime::generate_runtime_js();

    // Core reactive
    assert!(js.contains("export function signal("));
    assert!(js.contains("export function derive("));
    assert!(js.contains("export function effect("));
    assert!(js.contains("export function watch("));
    assert!(js.contains("export function batch("));

    // DOM utilities
    assert!(js.contains("export function $(id)"));
    assert!(js.contains("export function hydrate("));
    assert!(js.contains("export function bind("));
    assert!(js.contains("export function _readData()"));

    // Template dynamics
    assert!(js.contains("export function replaceRegion("));
    assert!(js.contains("export function reconcileList("));
}

// ═══════════════════════════════════════════════════════════════════════
// Full component end-to-end
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn full_component_generates_complete_hydration_script() {
    let decl = make_component(
        "Counter",
        vec![
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
            Stmt::Effect {
                body: Block {
                    stmts: vec![Stmt::ExprStmt {
                        expr: Expr::FnCall {
                            callee: Box::new(Expr::MemberAccess {
                                object: Box::new(Expr::Ident {
                                    name: "console".into(),
                                    span: s(),
                                }),
                                field: "log".into(),
                                span: s(),
                            }),
                            args: vec![Expr::Ident {
                                name: "doubled".into(),
                                span: s(),
                            }],
                            span: s(),
                        },
                        span: s(),
                    }],
                    span: s(),
                },
                cleanup: None,
                span: s(),
            },
            Stmt::Watch {
                target: Expr::Ident {
                    name: "count".into(),
                    span: s(),
                },
                next_name: "next".into(),
                prev_name: "prev".into(),
                body: Block {
                    stmts: vec![],
                    span: s(),
                },
                span: s(),
            },
        ],
        vec![TemplateNode::Element {
            tag: "button".into(),
            attributes: vec![],
            directives: vec![Directive::On {
                event: "click".into(),
                modifiers: vec![],
                handler: Expr::ArrowFn {
                    params: vec![],
                    ret_ty: None,
                    body: ArrowBody::Expr(Box::new(Expr::Assign {
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
                    })),
                    span: s(),
                },
                span: s(),
            }],
            children: vec![TemplateNode::Text {
                value: "Click".into(),
                span: s(),
            }],
            span: s(),
        }],
    );

    let js = emit_client::emit_page_script(&decl);

    // Verify structure
    assert!(js.contains("import {"), "should start with import");
    assert!(
        js.contains("from '/_gale/runtime.js'"),
        "should import from runtime"
    );
    assert!(
        js.contains("const $data = _readData()"),
        "should read server data"
    );
    assert!(js.contains("const count = signal("), "should create signal");
    assert!(
        js.contains("const doubled = derive("),
        "should create derive"
    );
    assert!(js.contains("effect(() =>"), "should run effects");
    assert!(js.contains("watch(() =>"), "should set up watch");
    assert!(js.contains("hydrate("), "should call hydrate");
    assert!(js.contains("addEventListener"), "should attach events");

    // Verify reactive scope closures — signals use .get() in expressions
    assert!(js.contains("count.get()"), "signal reads use .get()");
    assert!(js.contains("doubled.get()"), "derive reads use .get()");
}
