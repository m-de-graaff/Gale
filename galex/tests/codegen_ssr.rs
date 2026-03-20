//! Integration tests for SSR page generation (Phase 11.2).
//!
//! Tests construct AST nodes directly and verify the generated Rust
//! code contains expected patterns.

use galex::ast::*;
use galex::codegen::hydration::HydrationCtx;
use galex::codegen::route;
use galex::codegen::rust_emitter::RustEmitter;
use galex::codegen::ssr;
use galex::codegen::CodegenContext;
use galex::span::Span;
use galex::types::ty::TypeInterner;

fn s() -> Span {
    Span::dummy()
}

fn make_program(items: Vec<Item>) -> Program {
    Program { items, span: s() }
}

// ── SSR template rendering ─────────────────────────────────────────────

#[test]
fn ssr_static_text() {
    let mut e = RustEmitter::new();
    let mut h = HydrationCtx::new();
    let nodes = vec![TemplateNode::Text {
        value: "Hello, world!".into(),
        span: s(),
    }];
    ssr::emit_template_nodes(&mut e, &nodes, &mut h, None);
    let output = e.finish();
    assert!(
        output.contains("push_str"),
        "should use push_str for text: {output}"
    );
    assert!(
        output.contains("Hello, world!"),
        "should contain the text: {output}"
    );
}

#[test]
fn ssr_element_with_attrs() {
    let mut e = RustEmitter::new();
    let mut h = HydrationCtx::new();
    let nodes = vec![TemplateNode::Element {
        tag: "div".into(),
        attributes: vec![Attribute {
            name: "class".into(),
            value: AttrValue::String("container".into()),
            span: s(),
        }],
        directives: vec![],
        children: vec![TemplateNode::Text {
            value: "content".into(),
            span: s(),
        }],
        span: s(),
    }];
    ssr::emit_template_nodes(&mut e, &nodes, &mut h, None);
    let output = e.finish();
    assert!(output.contains("<div"), "should open div tag: {output}");
    assert!(
        output.contains("container"),
        "should have class value: {output}"
    );
    assert!(
        output.contains("class="),
        "should have class attr: {output}"
    );
    assert!(output.contains("</div>"), "should close div tag: {output}");
}

#[test]
fn ssr_self_closing() {
    let mut e = RustEmitter::new();
    let mut h = HydrationCtx::new();
    let nodes = vec![TemplateNode::SelfClosing {
        tag: "img".into(),
        attributes: vec![Attribute {
            name: "src".into(),
            value: AttrValue::String("/logo.png".into()),
            span: s(),
        }],
        directives: vec![],
        span: s(),
    }];
    ssr::emit_template_nodes(&mut e, &nodes, &mut h, None);
    let output = e.finish();
    assert!(output.contains("<img"), "should have img tag: {output}");
    assert!(
        output.contains("src=\\\"/logo.png\\\""),
        "should have src attr: {output}"
    );
    assert!(output.contains("/>"), "should self-close: {output}");
}

#[test]
fn ssr_expr_interpolation() {
    let mut e = RustEmitter::new();
    let mut h = HydrationCtx::new();
    let nodes = vec![TemplateNode::ExprInterp {
        expr: Expr::Ident {
            name: "userName".into(),
            span: s(),
        },
        span: s(),
    }];
    ssr::emit_template_nodes(&mut e, &nodes, &mut h, None);
    let output = e.finish();
    assert!(
        output.contains("escape_html"),
        "should HTML-escape expr output: {output}"
    );
    assert!(
        output.contains("user_name"),
        "should snake_case the ident: {output}"
    );
}

#[test]
fn ssr_when_block() {
    let mut e = RustEmitter::new();
    let mut h = HydrationCtx::new();
    let nodes = vec![TemplateNode::When {
        condition: Expr::BoolLit {
            value: true,
            span: s(),
        },
        body: vec![TemplateNode::Text {
            value: "visible".into(),
            span: s(),
        }],
        else_branch: None,
        span: s(),
    }];
    ssr::emit_template_nodes(&mut e, &nodes, &mut h, None);
    let output = e.finish();
    assert!(output.contains("if true"), "should generate if: {output}");
    assert!(
        output.contains("visible"),
        "should contain body text: {output}"
    );
}

#[test]
fn ssr_when_else() {
    let mut e = RustEmitter::new();
    let mut h = HydrationCtx::new();
    let nodes = vec![TemplateNode::When {
        condition: Expr::Ident {
            name: "isAdmin".into(),
            span: s(),
        },
        body: vec![TemplateNode::Text {
            value: "admin".into(),
            span: s(),
        }],
        else_branch: Some(WhenElse::Else(vec![TemplateNode::Text {
            value: "user".into(),
            span: s(),
        }])),
        span: s(),
    }];
    ssr::emit_template_nodes(&mut e, &nodes, &mut h, None);
    let output = e.finish();
    assert!(
        output.contains("if is_admin"),
        "should generate if with condition: {output}"
    );
    assert!(output.contains("else"), "should have else branch: {output}");
}

#[test]
fn ssr_each_block() {
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
            value: "row".into(),
            span: s(),
        }],
        empty: None,
        span: s(),
    }];
    ssr::emit_template_nodes(&mut e, &nodes, &mut h, None);
    let output = e.finish();
    assert!(
        output.contains("for item in items.iter()"),
        "should generate for loop: {output}"
    );
}

#[test]
fn ssr_each_with_index() {
    let mut e = RustEmitter::new();
    let mut h = HydrationCtx::new();
    let nodes = vec![TemplateNode::Each {
        binding: "item".into(),
        index: Some("idx".into()),
        iterable: Expr::Ident {
            name: "items".into(),
            span: s(),
        },
        body: vec![TemplateNode::Text {
            value: "row".into(),
            span: s(),
        }],
        empty: None,
        span: s(),
    }];
    ssr::emit_template_nodes(&mut e, &nodes, &mut h, None);
    let output = e.finish();
    assert!(
        output.contains("enumerate"),
        "should use enumerate for index: {output}"
    );
}

#[test]
fn ssr_each_with_empty() {
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
            value: "row".into(),
            span: s(),
        }],
        empty: Some(vec![TemplateNode::Text {
            value: "No items".into(),
            span: s(),
        }]),
        span: s(),
    }];
    ssr::emit_template_nodes(&mut e, &nodes, &mut h, None);
    let output = e.finish();
    assert!(
        output.contains("is_empty"),
        "should check for empty: {output}"
    );
    assert!(
        output.contains("No items"),
        "should have empty fallback: {output}"
    );
}

#[test]
fn ssr_slot_renders_param() {
    let mut e = RustEmitter::new();
    let mut h = HydrationCtx::new();
    let nodes = vec![TemplateNode::Slot {
        name: None,
        default: None,
        span: s(),
    }];
    ssr::emit_template_nodes(&mut e, &nodes, &mut h, Some("body_html"));
    let output = e.finish();
    assert!(
        output.contains("body_html"),
        "should inject slot parameter: {output}"
    );
}

#[test]
fn ssr_nested_elements() {
    let mut e = RustEmitter::new();
    let mut h = HydrationCtx::new();
    let nodes = vec![TemplateNode::Element {
        tag: "ul".into(),
        attributes: vec![],
        directives: vec![],
        children: vec![TemplateNode::Element {
            tag: "li".into(),
            attributes: vec![],
            directives: vec![],
            children: vec![TemplateNode::Text {
                value: "item".into(),
                span: s(),
            }],
            span: s(),
        }],
        span: s(),
    }];
    ssr::emit_template_nodes(&mut e, &nodes, &mut h, None);
    let output = e.finish();
    assert!(output.contains("<ul"), "should have ul: {output}");
    assert!(output.contains("<li"), "should have li: {output}");
    assert!(output.contains("</li>"), "should close li: {output}");
    assert!(output.contains("</ul>"), "should close ul: {output}");
}

// ── Route path convention ──────────────────────────────────────────────

#[test]
fn route_homepage_path() {
    assert_eq!(route::component_name_to_path("HomePage"), "/");
}

#[test]
fn route_index_path() {
    assert_eq!(route::component_name_to_path("Index"), "/");
}

#[test]
fn route_kebab_case_path() {
    assert_eq!(
        route::component_name_to_path("UserProfile"),
        "/user-profile"
    );
}

#[test]
fn route_simple_name() {
    assert_eq!(route::component_name_to_path("About"), "/about");
}

#[test]
fn route_dynamic_param() {
    assert_eq!(
        route::component_name_to_path("UserById[id]"),
        "/user-by-id/:id"
    );
}

#[test]
fn route_multiple_params() {
    assert_eq!(
        route::component_name_to_path("Post[year][slug]"),
        "/post/:year/:slug"
    );
}

// ── Route handler generation ───────────────────────────────────────────

#[test]
fn route_handler_generated() {
    let output = route::emit_route_module(&ComponentDecl {
        name: "About".into(),
        props: vec![],
        body: ComponentBody {
            stmts: vec![],
            template: vec![TemplateNode::Text {
                value: "About us".into(),
                span: s(),
            }],
            head: None,
            span: s(),
        },
        span: s(),
    });
    assert!(
        output.contains("pub async fn handler"),
        "should have handler fn: {output}"
    );
    assert!(
        output.contains("Html(crate::layout::render"),
        "should call layout::render: {output}"
    );
    assert!(
        output.contains("render_body"),
        "should have render_body fn: {output}"
    );
    assert!(
        output.contains("render_head"),
        "should have render_head fn: {output}"
    );
}

#[test]
fn route_handler_with_server_data() {
    let output = route::emit_route_module(&ComponentDecl {
        name: "UserList".into(),
        props: vec![],
        body: ComponentBody {
            stmts: vec![Stmt::Let {
                name: "users".into(),
                ty_ann: None,
                init: Expr::ArrayLit {
                    elements: vec![],
                    span: s(),
                },
                span: s(),
            }],
            template: vec![TemplateNode::Text {
                value: "list".into(),
                span: s(),
            }],
            head: None,
            span: s(),
        },
        span: s(),
    });
    assert!(
        output.contains("let users"),
        "should load server data: {output}"
    );
    assert!(
        output.contains("render_body(&users"),
        "should pass data to render_body: {output}"
    );
}

// ── Layout generation ──────────────────────────────────────────────────

#[test]
fn layout_default_shell() {
    let interner = TypeInterner::new();
    let program = make_program(vec![Item::ComponentDecl(ComponentDecl {
        name: "HomePage".into(),
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
    })]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    assert!(
        ctx.files.contains("src/layout.rs"),
        "should generate default layout"
    );
    let layout = ctx.files.get("src/layout.rs").unwrap();
    assert!(
        layout.contains("<!DOCTYPE html>"),
        "default layout should have doctype: {layout}"
    );
    assert!(
        layout.contains("body_html"),
        "default layout should inject body: {layout}"
    );
}

#[test]
fn layout_custom_with_slot() {
    let interner = TypeInterner::new();
    let program = make_program(vec![
        Item::LayoutDecl(LayoutDecl {
            name: "Shell".into(),
            props: vec![],
            body: ComponentBody {
                stmts: vec![],
                template: vec![TemplateNode::Element {
                    tag: "html".into(),
                    attributes: vec![],
                    directives: vec![],
                    children: vec![TemplateNode::Element {
                        tag: "body".into(),
                        attributes: vec![],
                        directives: vec![],
                        children: vec![TemplateNode::Slot {
                            name: None,
                            default: None,
                            span: s(),
                        }],
                        span: s(),
                    }],
                    span: s(),
                }],
                head: None,
                span: s(),
            },
            span: s(),
        }),
        Item::ComponentDecl(ComponentDecl {
            name: "HomePage".into(),
            props: vec![],
            body: ComponentBody {
                stmts: vec![],
                template: vec![],
                head: None,
                span: s(),
            },
            span: s(),
        }),
    ]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    assert!(
        ctx.files.contains("src/layout.rs"),
        "should generate custom layout"
    );
    let layout = ctx.files.get("src/layout.rs").unwrap();
    assert!(
        layout.contains("body_html"),
        "custom layout should inject body at slot: {layout}"
    );
}

// ── Head metadata ──────────────────────────────────────────────────────

#[test]
fn head_title_meta() {
    let output = route::emit_route_module(&ComponentDecl {
        name: "About".into(),
        props: vec![],
        body: ComponentBody {
            stmts: vec![],
            template: vec![],
            head: Some(HeadBlock {
                fields: vec![HeadField {
                    key: "title".into(),
                    value: Expr::StringLit {
                        value: "About Us".into(),
                        span: s(),
                    },
                    span: s(),
                }],
                span: s(),
            }),
            span: s(),
        },
        span: s(),
    });
    assert!(
        output.contains("<title>"),
        "should generate title tag: {output}"
    );
}

#[test]
fn head_description_meta() {
    let output = route::emit_route_module(&ComponentDecl {
        name: "About".into(),
        props: vec![],
        body: ComponentBody {
            stmts: vec![],
            template: vec![],
            head: Some(HeadBlock {
                fields: vec![HeadField {
                    key: "description".into(),
                    value: Expr::StringLit {
                        value: "Welcome".into(),
                        span: s(),
                    },
                    span: s(),
                }],
                span: s(),
            }),
            span: s(),
        },
        span: s(),
    });
    assert!(
        output.contains("description"),
        "should generate description meta: {output}"
    );
}

// ── Hydration markers ──────────────────────────────────────────────────

#[test]
fn hydration_marker_on_bind() {
    let mut e = RustEmitter::new();
    let mut h = HydrationCtx::new();
    let nodes = vec![TemplateNode::SelfClosing {
        tag: "input".into(),
        attributes: vec![],
        directives: vec![Directive::Bind {
            field: "name".into(),
            span: s(),
        }],
        span: s(),
    }];
    ssr::emit_template_nodes(&mut e, &nodes, &mut h, None);
    let output = e.finish();
    assert!(
        output.contains("data-gx-id"),
        "bind: directive should add hydration marker: {output}"
    );
    assert!(h.has_markers(), "should record hydration marker");
}

#[test]
fn hydration_marker_on_event() {
    let mut e = RustEmitter::new();
    let mut h = HydrationCtx::new();
    let nodes = vec![TemplateNode::Element {
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
    }];
    ssr::emit_template_nodes(&mut e, &nodes, &mut h, None);
    let output = e.finish();
    assert!(
        output.contains("data-gx-id"),
        "on: directive should add hydration marker: {output}"
    );
}

#[test]
fn hydration_script_block() {
    let mut e = RustEmitter::new();
    let mut h = HydrationCtx::new();
    h.mark_bind("name");
    h.emit_script(&mut e);
    let output = e.finish();
    assert!(
        output.contains("gale-data"),
        "should emit gale-data script: {output}"
    );
    assert!(
        output.contains("markers"),
        "should contain markers: {output}"
    );
}

// ── class: directive SSR ───────────────────────────────────────────────

#[test]
fn class_directive_ssr() {
    let mut e = RustEmitter::new();
    let mut h = HydrationCtx::new();
    let nodes = vec![TemplateNode::Element {
        tag: "div".into(),
        attributes: vec![],
        directives: vec![Directive::Class {
            name: "active".into(),
            condition: Expr::BoolLit {
                value: true,
                span: s(),
            },
            span: s(),
        }],
        children: vec![],
        span: s(),
    }];
    ssr::emit_template_nodes(&mut e, &nodes, &mut h, None);
    let output = e.finish();
    assert!(
        output.contains("classes"),
        "should build class list: {output}"
    );
    assert!(
        output.contains("active"),
        "should include class name: {output}"
    );
}

// ── form: directives SSR ───────────────────────────────────────────────

#[test]
fn form_action_ssr() {
    let mut e = RustEmitter::new();
    let mut h = HydrationCtx::new();
    let nodes = vec![TemplateNode::Element {
        tag: "form".into(),
        attributes: vec![],
        directives: vec![Directive::FormAction {
            action: Expr::Ident {
                name: "createUser".into(),
                span: s(),
            },
            span: s(),
        }],
        children: vec![],
        span: s(),
    }];
    ssr::emit_template_nodes(&mut e, &nodes, &mut h, None);
    let output = e.finish();
    assert!(
        output.contains("/api/__gx/actions/createUser"),
        "should generate action URL: {output}"
    );
    assert!(
        output.contains("method=\\\"post\\\""),
        "should set POST method: {output}"
    );
}

// ── build_router with routes ───────────────────────────────────────────

#[test]
fn build_router_has_routes() {
    let interner = TypeInterner::new();
    let program = make_program(vec![
        Item::ComponentDecl(ComponentDecl {
            name: "HomePage".into(),
            props: vec![],
            body: ComponentBody {
                stmts: vec![],
                template: vec![],
                head: None,
                span: s(),
            },
            span: s(),
        }),
        Item::ComponentDecl(ComponentDecl {
            name: "About".into(),
            props: vec![],
            body: ComponentBody {
                stmts: vec![],
                template: vec![],
                head: None,
                span: s(),
            },
            span: s(),
        }),
    ]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    let main = ctx.files.get("src/main.rs").unwrap();
    assert!(main.contains(".route(\"/\""), "should have / route: {main}");
    assert!(
        main.contains(".route(\"/about\""),
        "should have /about route: {main}"
    );
    assert!(
        main.contains("routes::home_page::handler"),
        "should reference handler: {main}"
    );
}

// ── Bool attribute rendering ───────────────────────────────────────────

#[test]
fn bool_attribute_ssr() {
    let mut e = RustEmitter::new();
    let mut h = HydrationCtx::new();
    let nodes = vec![TemplateNode::SelfClosing {
        tag: "input".into(),
        attributes: vec![Attribute {
            name: "disabled".into(),
            value: AttrValue::Bool,
            span: s(),
        }],
        directives: vec![],
        span: s(),
    }];
    ssr::emit_template_nodes(&mut e, &nodes, &mut h, None);
    let output = e.finish();
    assert!(
        output.contains("disabled"),
        "should have disabled attribute: {output}"
    );
}

// ── SSR runtime generation ─────────────────────────────────────────────

#[test]
fn gale_ssr_runtime_generated() {
    let interner = TypeInterner::new();
    let program = make_program(vec![Item::ComponentDecl(ComponentDecl {
        name: "Page".into(),
        props: vec![],
        body: ComponentBody {
            stmts: vec![],
            template: vec![],
            head: None,
            span: s(),
        },
        span: s(),
    })]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    assert!(
        ctx.files.contains("src/gale_ssr.rs"),
        "should generate SSR runtime"
    );
    let runtime = ctx.files.get("src/gale_ssr.rs").unwrap();
    assert!(
        runtime.contains("escape_html"),
        "runtime should have escape_html: {runtime}"
    );
}

// ── Expression rendering ───────────────────────────────────────────────

#[test]
fn expr_string_literal() {
    let result = galex::codegen::expr::expr_to_rust(&Expr::StringLit {
        value: "hello".into(),
        span: s(),
    });
    assert!(result.contains("hello"), "should contain string: {result}");
}

#[test]
fn expr_ident_snake_case() {
    let result = galex::codegen::expr::expr_to_rust(&Expr::Ident {
        name: "userName".into(),
        span: s(),
    });
    assert_eq!(result, "user_name");
}

#[test]
fn expr_member_access() {
    let result = galex::codegen::expr::expr_to_rust(&Expr::MemberAccess {
        object: Box::new(Expr::Ident {
            name: "user".into(),
            span: s(),
        }),
        field: "firstName".into(),
        span: s(),
    });
    assert_eq!(result, "user.first_name");
}

#[test]
fn expr_binary_op() {
    let result = galex::codegen::expr::expr_to_rust(&Expr::BinaryOp {
        left: Box::new(Expr::IntLit {
            value: 1,
            span: s(),
        }),
        op: BinOp::Add,
        right: Box::new(Expr::IntLit {
            value: 2,
            span: s(),
        }),
        span: s(),
    });
    assert!(result.contains("+"), "should have + operator: {result}");
}

#[test]
fn expr_env_access() {
    let result = galex::codegen::expr::expr_to_rust(&Expr::EnvAccess {
        key: "DATABASE_URL".into(),
        span: s(),
    });
    assert!(
        result.contains("std::env::var"),
        "should use env::var: {result}"
    );
    assert!(result.contains("DATABASE_URL"), "should have key: {result}");
}
