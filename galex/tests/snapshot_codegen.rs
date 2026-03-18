//! Snapshot tests for GaleX code generation.
//!
//! Uses the `insta` crate to capture complete generated Rust files and
//! compare against stored snapshots. Run `cargo insta review` to accept
//! changes after legitimate codegen modifications.

use galex::ast::*;
use galex::codegen::CodegenContext;
use galex::span::Span;
use galex::types::ty::TypeInterner;

fn s() -> Span {
    Span::dummy()
}

fn make_program(items: Vec<Item>) -> Program {
    Program { items, span: s() }
}

// ── Helper: build common AST fragments ─────────────────────────────────

fn simple_guard(name: &str, fields: Vec<GuardFieldDecl>) -> Item {
    Item::GuardDecl(GuardDecl {
        name: name.into(),
        fields,
        span: s(),
    })
}

fn simple_action(name: &str, params: Vec<Param>, stmts: Vec<Stmt>) -> Item {
    Item::ActionDecl(ActionDecl {
        name: name.into(),
        params,
        ret_ty: None,
        body: Block { stmts, span: s() },
        span: s(),
    })
}

fn simple_component(name: &str, stmts: Vec<Stmt>, template: Vec<TemplateNode>) -> Item {
    Item::ComponentDecl(ComponentDecl {
        name: name.into(),
        props: vec![],
        body: ComponentBody {
            stmts,
            template,
            head: None,
            span: s(),
        },
        span: s(),
    })
}

fn text_node(text: &str) -> TemplateNode {
    TemplateNode::Text {
        value: text.into(),
        span: s(),
    }
}

fn string_field(name: &str, validators: Vec<ValidatorCall>) -> GuardFieldDecl {
    GuardFieldDecl {
        name: name.into(),
        ty: TypeAnnotation::Named {
            name: "string".into(),
            span: s(),
        },
        validators,
        span: s(),
    }
}

fn int_field(name: &str, validators: Vec<ValidatorCall>) -> GuardFieldDecl {
    GuardFieldDecl {
        name: name.into(),
        ty: TypeAnnotation::Named {
            name: "int".into(),
            span: s(),
        },
        validators,
        span: s(),
    }
}

fn validator(name: &str, args: Vec<Expr>) -> ValidatorCall {
    ValidatorCall {
        name: name.into(),
        args,
        span: s(),
    }
}

fn int_lit(v: i64) -> Expr {
    Expr::IntLit {
        value: v,
        span: s(),
    }
}

fn string_param(name: &str) -> Param {
    Param {
        name: name.into(),
        ty_ann: Some(TypeAnnotation::Named {
            name: "string".into(),
            span: s(),
        }),
        default: None,
        span: s(),
    }
}

fn guard_param(name: &str, guard_name: &str) -> Param {
    Param {
        name: name.into(),
        ty_ann: Some(TypeAnnotation::Named {
            name: guard_name.into(),
            span: s(),
        }),
        default: None,
        span: s(),
    }
}

// ══════════════════════════════════════════════════════════════════════
// Snapshot: main.rs with all feature types
// ══════════════════════════════════════════════════════════════════════

#[test]
fn snap_main_rs_full_project() {
    let interner = TypeInterner::new();
    let program = make_program(vec![
        // Guard
        simple_guard(
            "UserForm",
            vec![
                string_field("email", vec![validator("email", vec![])]),
                string_field("name", vec![validator("minLen", vec![int_lit(1)])]),
            ],
        ),
        // Action
        simple_action(
            "createUser",
            vec![guard_param("data", "UserForm")],
            vec![Stmt::Return {
                value: Some(Expr::ObjectLit {
                    fields: vec![ObjectFieldExpr {
                        key: "ok".into(),
                        value: Expr::BoolLit {
                            value: true,
                            span: s(),
                        },
                        span: s(),
                    }],
                    span: s(),
                }),
                span: s(),
            }],
        ),
        // Components (routes)
        simple_component("HomePage", vec![], vec![text_node("Welcome")]),
        simple_component("About", vec![], vec![text_node("About us")]),
        // Channel
        Item::ChannelDecl(ChannelDecl {
            name: "Chat".into(),
            params: vec![],
            direction: ChannelDirection::Bidirectional,
            msg_ty: TypeAnnotation::Named {
                name: "string".into(),
                span: s(),
            },
            handlers: vec![ChannelHandler {
                event: "receive".into(),
                params: vec![Param {
                    name: "msg".into(),
                    ty_ann: None,
                    default: None,
                    span: s(),
                }],
                body: Block {
                    stmts: vec![],
                    span: s(),
                },
                span: s(),
            }],
            span: s(),
        }),
        // API
        Item::ApiDecl(ApiDecl {
            name: "Users".into(),
            handlers: vec![
                ApiHandler {
                    method: HttpMethod::Get,
                    path_params: vec![],
                    params: vec![],
                    ret_ty: None,
                    body: Block {
                        stmts: vec![],
                        span: s(),
                    },
                    span: s(),
                },
                ApiHandler {
                    method: HttpMethod::Post,
                    path_params: vec![],
                    params: vec![string_param("name")],
                    ret_ty: None,
                    body: Block {
                        stmts: vec![],
                        span: s(),
                    },
                    span: s(),
                },
            ],
            span: s(),
        }),
        // Enum
        Item::EnumDecl(EnumDecl {
            name: "Status".into(),
            variants: vec!["Active".into(), "Inactive".into()],
            span: s(),
        }),
    ]);

    let mut ctx = CodegenContext::new(&interner, "full_app");
    ctx.emit_program(&program);

    let main = ctx.files.get("src/main.rs").unwrap();
    insta::assert_snapshot!("main_rs_full_project", main);
}

// ══════════════════════════════════════════════════════════════════════
// Snapshot: Cargo.toml variants
// ══════════════════════════════════════════════════════════════════════

#[test]
fn snap_cargo_toml_with_regex() {
    let interner = TypeInterner::new();
    let program = make_program(vec![simple_guard(
        "EmailForm",
        vec![string_field("email", vec![validator("email", vec![])])],
    )]);
    let mut ctx = CodegenContext::new(&interner, "regex_app");
    ctx.emit_program(&program);

    let toml = ctx.files.get("Cargo.toml").unwrap();
    insta::assert_snapshot!("cargo_toml_with_regex", toml);
}

#[test]
fn snap_cargo_toml_without_regex() {
    let interner = TypeInterner::new();
    let program = make_program(vec![simple_guard(
        "AgeForm",
        vec![int_field("age", vec![validator("min", vec![int_lit(0)])])],
    )]);
    let mut ctx = CodegenContext::new(&interner, "no_regex_app");
    ctx.emit_program(&program);

    let toml = ctx.files.get("Cargo.toml").unwrap();
    insta::assert_snapshot!("cargo_toml_without_regex", toml);
}

// ══════════════════════════════════════════════════════════════════════
// Snapshot: Action handlers
// ══════════════════════════════════════════════════════════════════════

#[test]
fn snap_action_with_guard() {
    let interner = TypeInterner::new();
    let program = make_program(vec![
        simple_guard(
            "UserForm",
            vec![
                string_field("email", vec![validator("email", vec![])]),
                string_field(
                    "name",
                    vec![
                        validator("trim", vec![]),
                        validator("minLen", vec![int_lit(1)]),
                    ],
                ),
            ],
        ),
        simple_action("createUser", vec![guard_param("data", "UserForm")], vec![]),
    ]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    let action = ctx.files.get("src/actions/create_user.rs").unwrap();
    insta::assert_snapshot!("action_with_guard", action);
}

#[test]
fn snap_action_plain_params() {
    let interner = TypeInterner::new();
    let program = make_program(vec![simple_action(
        "addItem",
        vec![
            string_param("name"),
            Param {
                name: "count".into(),
                ty_ann: Some(TypeAnnotation::Named {
                    name: "int".into(),
                    span: s(),
                }),
                default: None,
                span: s(),
            },
        ],
        vec![],
    )]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    let action = ctx.files.get("src/actions/add_item.rs").unwrap();
    insta::assert_snapshot!("action_plain_params", action);
}

// ══════════════════════════════════════════════════════════════════════
// Snapshot: Guard validators
// ══════════════════════════════════════════════════════════════════════

#[test]
fn snap_guard_with_validators() {
    let interner = TypeInterner::new();
    let program = make_program(vec![simple_guard(
        "UserForm",
        vec![
            string_field(
                "email",
                vec![validator("trim", vec![]), validator("email", vec![])],
            ),
            string_field(
                "name",
                vec![
                    validator("minLen", vec![int_lit(2)]),
                    validator("maxLen", vec![int_lit(100)]),
                ],
            ),
            int_field(
                "age",
                vec![validator("range", vec![int_lit(0), int_lit(150)])],
            ),
        ],
    )]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    let guard = ctx.files.get("src/guards/user_form.rs").unwrap();
    insta::assert_snapshot!("guard_with_validators", guard);
}

#[test]
fn snap_guard_simple() {
    let interner = TypeInterner::new();
    let program = make_program(vec![simple_guard(
        "LoginForm",
        vec![
            string_field("email", vec![]),
            string_field("password", vec![]),
        ],
    )]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    let guard = ctx.files.get("src/guards/login_form.rs").unwrap();
    insta::assert_snapshot!("guard_simple", guard);
}

// ══════════════════════════════════════════════════════════════════════
// Snapshot: Route handlers
// ══════════════════════════════════════════════════════════════════════

#[test]
fn snap_route_static() {
    let interner = TypeInterner::new();
    let program = make_program(vec![simple_component(
        "About",
        vec![],
        vec![
            TemplateNode::Element {
                tag: "h1".into(),
                attributes: vec![],
                directives: vec![],
                children: vec![text_node("About Us")],
                span: s(),
            },
            TemplateNode::Element {
                tag: "p".into(),
                attributes: vec![],
                directives: vec![],
                children: vec![text_node("Welcome to the about page.")],
                span: s(),
            },
        ],
    )]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    let route = ctx.files.get("src/routes/about.rs").unwrap();
    insta::assert_snapshot!("route_static", route);
}

#[test]
fn snap_route_dynamic() {
    let interner = TypeInterner::new();
    let program = make_program(vec![simple_component(
        "BlogPost[slug]",
        vec![],
        vec![text_node("Post content")],
    )]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    let route = ctx.files.get("src/routes/blog_post.rs").unwrap();
    insta::assert_snapshot!("route_dynamic", route);
}

// ══════════════════════════════════════════════════════════════════════
// Snapshot: Channel handlers
// ══════════════════════════════════════════════════════════════════════

#[test]
fn snap_channel_bidirectional() {
    let interner = TypeInterner::new();
    let program = make_program(vec![Item::ChannelDecl(ChannelDecl {
        name: "Chat".into(),
        params: vec![],
        direction: ChannelDirection::Bidirectional,
        msg_ty: TypeAnnotation::Named {
            name: "string".into(),
            span: s(),
        },
        handlers: vec![
            ChannelHandler {
                event: "connect".into(),
                params: vec![],
                body: Block {
                    stmts: vec![],
                    span: s(),
                },
                span: s(),
            },
            ChannelHandler {
                event: "receive".into(),
                params: vec![Param {
                    name: "msg".into(),
                    ty_ann: None,
                    default: None,
                    span: s(),
                }],
                body: Block {
                    stmts: vec![],
                    span: s(),
                },
                span: s(),
            },
            ChannelHandler {
                event: "disconnect".into(),
                params: vec![],
                body: Block {
                    stmts: vec![],
                    span: s(),
                },
                span: s(),
            },
        ],
        span: s(),
    })]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    let channel = ctx.files.get("src/channels/chat.rs").unwrap();
    insta::assert_snapshot!("channel_bidirectional", channel);
}

#[test]
fn snap_channel_server_to_client() {
    let interner = TypeInterner::new();
    let program = make_program(vec![Item::ChannelDecl(ChannelDecl {
        name: "Ticker".into(),
        params: vec![],
        direction: ChannelDirection::ServerToClient,
        msg_ty: TypeAnnotation::Named {
            name: "string".into(),
            span: s(),
        },
        handlers: vec![],
        span: s(),
    })]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    let channel = ctx.files.get("src/channels/ticker.rs").unwrap();
    insta::assert_snapshot!("channel_server_to_client", channel);
}

// ══════════════════════════════════════════════════════════════════════
// Snapshot: API resource
// ══════════════════════════════════════════════════════════════════════

#[test]
fn snap_api_resource() {
    let interner = TypeInterner::new();
    let program = make_program(vec![Item::ApiDecl(ApiDecl {
        name: "Users".into(),
        handlers: vec![
            ApiHandler {
                method: HttpMethod::Get,
                path_params: vec![],
                params: vec![],
                ret_ty: None,
                body: Block {
                    stmts: vec![],
                    span: s(),
                },
                span: s(),
            },
            ApiHandler {
                method: HttpMethod::Post,
                path_params: vec![],
                params: vec![string_param("name")],
                ret_ty: None,
                body: Block {
                    stmts: vec![],
                    span: s(),
                },
                span: s(),
            },
            ApiHandler {
                method: HttpMethod::Delete,
                path_params: vec!["id".into()],
                params: vec![],
                ret_ty: None,
                body: Block {
                    stmts: vec![],
                    span: s(),
                },
                span: s(),
            },
        ],
        span: s(),
    })]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    let api = ctx.files.get("src/api/users.rs").unwrap();
    insta::assert_snapshot!("api_resource", api);
}

// ══════════════════════════════════════════════════════════════════════
// Snapshot: Layout
// ══════════════════════════════════════════════════════════════════════

#[test]
fn snap_layout_default() {
    let interner = TypeInterner::new();
    // A component triggers default layout generation
    let program = make_program(vec![simple_component(
        "Index",
        vec![],
        vec![text_node("Hello")],
    )]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    let layout = ctx.files.get("src/layout.rs").unwrap();
    insta::assert_snapshot!("layout_default", layout);
}

// ══════════════════════════════════════════════════════════════════════
// Snapshot: Shared validation
// ══════════════════════════════════════════════════════════════════════

#[test]
fn snap_validation_module() {
    let interner = TypeInterner::new();
    let program = make_program(vec![simple_guard(
        "Minimal",
        vec![string_field("name", vec![])],
    )]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    let validation = ctx.files.get("src/shared/validation.rs").unwrap();
    insta::assert_snapshot!("validation_module", validation);
}

// ══════════════════════════════════════════════════════════════════════
// Snapshot: SSR runtime
// ══════════════════════════════════════════════════════════════════════

#[test]
fn snap_ssr_runtime() {
    let interner = TypeInterner::new();
    let program = make_program(vec![simple_component(
        "Page",
        vec![],
        vec![text_node("hi")],
    )]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    let ssr = ctx.files.get("src/gale_ssr.rs").unwrap();
    insta::assert_snapshot!("ssr_runtime", ssr);
}
