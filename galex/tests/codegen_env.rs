//! Integration tests for env generation (Phase 11.8).

use galex::ast::*;
use galex::codegen::emit_env;
use galex::codegen::rust_emitter::RustEmitter;
use galex::codegen::CodegenContext;
use galex::span::Span;
use galex::types::ty::TypeInterner;

fn s() -> Span {
    Span::dummy()
}

fn make_program(items: Vec<Item>) -> Program {
    Program { items, span: s() }
}

fn str_ty() -> TypeAnnotation {
    TypeAnnotation::Named {
        name: "string".into(),
        span: s(),
    }
}

fn int_ty() -> TypeAnnotation {
    TypeAnnotation::Named {
        name: "int".into(),
        span: s(),
    }
}

fn bool_ty() -> TypeAnnotation {
    TypeAnnotation::Named {
        name: "bool".into(),
        span: s(),
    }
}

fn float_ty() -> TypeAnnotation {
    TypeAnnotation::Named {
        name: "float".into(),
        span: s(),
    }
}

fn make_env(vars: Vec<EnvVarDef>) -> EnvDecl {
    EnvDecl { vars, span: s() }
}

fn emit_env(decl: &EnvDecl) -> String {
    let mut e = RustEmitter::new();
    emit_env::emit_env_file(&mut e, decl);
    e.finish()
}

// ── File generation ────────────────────────────────────────────────────

#[test]
fn env_decl_generates_file() {
    let interner = TypeInterner::new();
    let program = make_program(vec![Item::EnvDecl(make_env(vec![EnvVarDef {
        key: "DATABASE_URL".into(),
        ty: str_ty(),
        validators: vec![],
        default: None,
        span: s(),
    }]))]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    assert!(
        ctx.files.contains("src/env_config.rs"),
        "should generate src/env_config.rs"
    );
}

// ── Struct fields ──────────────────────────────────────────────────────

#[test]
fn env_struct_has_string_field() {
    let out = emit_env(&make_env(vec![EnvVarDef {
        key: "DATABASE_URL".into(),
        ty: str_ty(),
        validators: vec![],
        default: None,
        span: s(),
    }]));
    assert!(
        out.contains("pub database_url: String,"),
        "should have String field: {out}"
    );
}

#[test]
fn env_struct_has_int_field() {
    let out = emit_env(&make_env(vec![EnvVarDef {
        key: "PORT".into(),
        ty: int_ty(),
        validators: vec![],
        default: None,
        span: s(),
    }]));
    assert!(
        out.contains("pub port: i64,"),
        "should have i64 field: {out}"
    );
}

#[test]
fn env_struct_has_bool_field() {
    let out = emit_env(&make_env(vec![EnvVarDef {
        key: "DEBUG".into(),
        ty: bool_ty(),
        validators: vec![],
        default: None,
        span: s(),
    }]));
    assert!(
        out.contains("pub debug: bool,"),
        "should have bool field: {out}"
    );
}

#[test]
fn env_struct_has_float_field() {
    let out = emit_env(&make_env(vec![EnvVarDef {
        key: "RATIO".into(),
        ty: float_ty(),
        validators: vec![],
        default: None,
        span: s(),
    }]));
    assert!(
        out.contains("pub ratio: f64,"),
        "should have f64 field: {out}"
    );
}

// ── Load parsing ───────────────────────────────────────────────────────

#[test]
fn env_load_parses_int() {
    let out = emit_env(&make_env(vec![EnvVarDef {
        key: "PORT".into(),
        ty: int_ty(),
        validators: vec![],
        default: None,
        span: s(),
    }]));
    assert!(out.contains("parse::<i64>()"), "should parse i64: {out}");
}

#[test]
fn env_load_parses_bool() {
    let out = emit_env(&make_env(vec![EnvVarDef {
        key: "DEBUG".into(),
        ty: bool_ty(),
        validators: vec![],
        default: None,
        span: s(),
    }]));
    assert!(out.contains("parse::<bool>()"), "should parse bool: {out}");
}

// ── Validators ─────────────────────────────────────────────────────────

#[test]
fn env_validate_non_empty() {
    let out = emit_env(&make_env(vec![EnvVarDef {
        key: "DATABASE_URL".into(),
        ty: str_ty(),
        validators: vec![ValidatorCall {
            name: "nonEmpty".into(),
            args: vec![],
            span: s(),
        }],
        default: None,
        span: s(),
    }]));
    assert!(out.contains("is_empty()"), "should check for empty: {out}");
    assert!(
        out.contains("must not be empty"),
        "should have error message: {out}"
    );
}

#[test]
fn env_validate_min() {
    let out = emit_env(&make_env(vec![EnvVarDef {
        key: "PORT".into(),
        ty: int_ty(),
        validators: vec![ValidatorCall {
            name: "min".into(),
            args: vec![Expr::IntLit {
                value: 1,
                span: s(),
            }],
            span: s(),
        }],
        default: None,
        span: s(),
    }]));
    assert!(out.contains("< 1"), "should have min check: {out}");
}

#[test]
fn env_validate_max() {
    let out = emit_env(&make_env(vec![EnvVarDef {
        key: "PORT".into(),
        ty: int_ty(),
        validators: vec![ValidatorCall {
            name: "max".into(),
            args: vec![Expr::IntLit {
                value: 65535,
                span: s(),
            }],
            span: s(),
        }],
        default: None,
        span: s(),
    }]));
    assert!(out.contains("> 65535"), "should have max check: {out}");
}

// ── Default values ─────────────────────────────────────────────────────

#[test]
fn env_default_value() {
    let out = emit_env(&make_env(vec![EnvVarDef {
        key: "HOST".into(),
        ty: str_ty(),
        validators: vec![],
        default: Some(Expr::StringLit {
            value: "localhost".into(),
            span: s(),
        }),
        span: s(),
    }]));
    assert!(
        out.contains("unwrap_or"),
        "should use unwrap_or with default: {out}"
    );
    assert!(
        out.contains("localhost"),
        "should contain default value: {out}"
    );
}

// ── Static singleton ───────────────────────────────────────────────────

#[test]
fn env_static_lazy_lock() {
    let out = emit_env(&make_env(vec![EnvVarDef {
        key: "X".into(),
        ty: str_ty(),
        validators: vec![],
        default: None,
        span: s(),
    }]));
    assert!(out.contains("LazyLock<Env>"), "should use LazyLock: {out}");
}

#[test]
fn env_dotenvy_in_static() {
    let out = emit_env(&make_env(vec![EnvVarDef {
        key: "X".into(),
        ty: str_ty(),
        validators: vec![],
        default: None,
        span: s(),
    }]));
    assert!(
        out.contains("dotenvy::dotenv()"),
        "should load .env file: {out}"
    );
}

#[test]
fn env_fail_fast_exit() {
    let out = emit_env(&make_env(vec![EnvVarDef {
        key: "X".into(),
        ty: str_ty(),
        validators: vec![],
        default: None,
        span: s(),
    }]));
    assert!(
        out.contains("std::process::exit(1)"),
        "should exit on validation failure: {out}"
    );
}

// ── Public vars JSON ───────────────────────────────────────────────────

#[test]
fn env_public_vars_json() {
    let out = emit_env(&make_env(vec![
        EnvVarDef {
            key: "DATABASE_URL".into(),
            ty: str_ty(),
            validators: vec![],
            default: None,
            span: s(),
        },
        EnvVarDef {
            key: "PUBLIC_API_URL".into(),
            ty: str_ty(),
            validators: vec![],
            default: None,
            span: s(),
        },
    ]));
    assert!(
        out.contains("PUBLIC_API_URL"),
        "public_vars_json should include PUBLIC_ var: {out}"
    );
    assert!(
        out.contains("serde_json::json!"),
        "should use serde_json: {out}"
    );
}

#[test]
fn env_public_vars_json_excludes_server() {
    let out = emit_env(&make_env(vec![EnvVarDef {
        key: "SECRET_KEY".into(),
        ty: str_ty(),
        validators: vec![],
        default: None,
        span: s(),
    }]));
    // No PUBLIC_ vars → empty JSON
    assert!(
        !out.contains("SECRET_KEY") || !out.contains("serde_json::json"),
        "public_vars_json should NOT include server vars"
    );
}

// ── main.rs integration ────────────────────────────────────────────────

#[test]
fn env_main_declares_mod() {
    let interner = TypeInterner::new();
    let program = make_program(vec![Item::EnvDecl(make_env(vec![EnvVarDef {
        key: "X".into(),
        ty: str_ty(),
        validators: vec![],
        default: None,
        span: s(),
    }]))]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    let main = ctx.files.get("src/main.rs").unwrap();
    assert!(
        main.contains("mod env_config;"),
        "main.rs should declare env_config: {main}"
    );
}

#[test]
fn env_main_triggers_init() {
    let interner = TypeInterner::new();
    let program = make_program(vec![Item::EnvDecl(make_env(vec![EnvVarDef {
        key: "X".into(),
        ty: str_ty(),
        validators: vec![],
        default: None,
        span: s(),
    }]))]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    let main = ctx.files.get("src/main.rs").unwrap();
    assert!(
        main.contains("env_config::ENV"),
        "main.rs should trigger ENV init: {main}"
    );
}

#[test]
fn env_cargo_has_dotenvy() {
    let interner = TypeInterner::new();
    let program = make_program(vec![Item::EnvDecl(make_env(vec![EnvVarDef {
        key: "X".into(),
        ty: str_ty(),
        validators: vec![],
        default: None,
        span: s(),
    }]))]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    let cargo = ctx.files.get("Cargo.toml").unwrap();
    assert!(
        cargo.contains("dotenvy"),
        "Cargo.toml should have dotenvy: {cargo}"
    );
}

// ── SSR embedding ──────────────────────────────────────────────────────

#[test]
fn env_ssr_embeds_public_vars() {
    let interner = TypeInterner::new();
    let program = make_program(vec![
        Item::EnvDecl(make_env(vec![EnvVarDef {
            key: "PUBLIC_API_URL".into(),
            ty: str_ty(),
            validators: vec![],
            default: None,
            span: s(),
        }])),
        Item::ComponentDecl(ComponentDecl {
            name: "HomePage".into(),
            props: vec![],
            body: ComponentBody {
                stmts: vec![],
                template: vec![TemplateNode::Text {
                    value: "hi".into(),
                    span: s(),
                }],
                head: None,
                span: s(),
            },
            span: s(),
        }),
    ]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    let route = ctx.files.get("src/routes/home_page.rs").unwrap();
    assert!(
        route.contains("gale-env"),
        "SSR should embed public env vars: {route}"
    );
    assert!(
        route.contains("public_vars_json"),
        "SSR should call public_vars_json: {route}"
    );
}

#[test]
fn env_ssr_no_script_without_public_vars() {
    let interner = TypeInterner::new();
    let program = make_program(vec![
        Item::EnvDecl(make_env(vec![EnvVarDef {
            key: "SECRET_KEY".into(),
            ty: str_ty(),
            validators: vec![],
            default: None,
            span: s(),
        }])),
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

    let route = ctx.files.get("src/routes/home_page.rs").unwrap();
    assert!(
        !route.contains("gale-env"),
        "SSR should NOT embed env script when no PUBLIC_ vars: {route}"
    );
}

// ── Typed accessor ─────────────────────────────────────────────────────

#[test]
fn env_typed_accessor() {
    let interner = TypeInterner::new();
    let program = make_program(vec![
        Item::EnvDecl(make_env(vec![EnvVarDef {
            key: "PORT".into(),
            ty: int_ty(),
            validators: vec![],
            default: None,
            span: s(),
        }])),
        Item::ComponentDecl(ComponentDecl {
            name: "Page".into(),
            props: vec![],
            body: ComponentBody {
                stmts: vec![],
                template: vec![TemplateNode::ExprInterp {
                    expr: Expr::EnvAccess {
                        key: "PORT".into(),
                        span: s(),
                    },
                    span: s(),
                }],
                head: None,
                span: s(),
            },
            span: s(),
        }),
    ]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    let route = ctx.files.get("src/routes/page.rs").unwrap();
    assert!(
        route.contains("crate::env_config::ENV.port"),
        "env.PORT should use typed accessor: {route}"
    );
    assert!(
        !route.contains("std::env::var"),
        "should NOT use std::env::var for declared keys: {route}"
    );
}

#[test]
fn env_undeclared_fallback() {
    let interner = TypeInterner::new();
    let program = make_program(vec![
        Item::EnvDecl(make_env(vec![EnvVarDef {
            key: "PORT".into(),
            ty: int_ty(),
            validators: vec![],
            default: None,
            span: s(),
        }])),
        Item::ComponentDecl(ComponentDecl {
            name: "Page".into(),
            props: vec![],
            body: ComponentBody {
                stmts: vec![],
                template: vec![TemplateNode::ExprInterp {
                    expr: Expr::EnvAccess {
                        key: "UNDECLARED_KEY".into(),
                        span: s(),
                    },
                    span: s(),
                }],
                head: None,
                span: s(),
            },
            span: s(),
        }),
    ]);
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    let route = ctx.files.get("src/routes/page.rs").unwrap();
    assert!(
        route.contains("std::env::var(\"UNDECLARED_KEY\")"),
        "undeclared env key should fall back to std::env::var: {route}"
    );
}
