//! Integration tests for client-side action stub code generation (Phase 12.4).
//!
//! Tests construct AST nodes directly and verify the generated JavaScript
//! contains expected patterns for action stubs, error handling, guard
//! integration, and query cache optimistic updates.

use galex::ast::*;
use galex::codegen::emit_client_actions;
use galex::codegen::emit_client_runtime;
use galex::codegen::emit_guard_js::{self, GuardJsMeta};
use galex::codegen::js_emitter::JsEmitter;
use galex::codegen::CodegenContext;
use galex::span::Span;
use galex::types::ty::TypeInterner;
use std::collections::HashSet;

fn s() -> Span {
    Span::dummy()
}

fn guards(names: &[&str]) -> HashSet<String> {
    names.iter().map(|n| n.to_string()).collect()
}

fn make_action(name: &str, params: Vec<Param>) -> ActionDecl {
    ActionDecl {
        name: name.into(),
        params,
        ret_ty: None,
        body: Block {
            stmts: vec![],
            span: s(),
        },
        span: s(),
    }
}

fn param(name: &str, ty: &str) -> Param {
    Param {
        name: name.into(),
        ty_ann: Some(TypeAnnotation::Named {
            name: ty.into(),
            span: s(),
        }),
        default: None,
        span: s(),
    }
}

fn make_guard_decl(name: &str, fields: Vec<GuardFieldDecl>) -> GuardDecl {
    GuardDecl {
        name: name.into(),
        fields,
        span: s(),
    }
}

fn make_field(name: &str, ty: &str, validators: Vec<(&str, Vec<Expr>)>) -> GuardFieldDecl {
    GuardFieldDecl {
        name: name.into(),
        ty: TypeAnnotation::Named {
            name: ty.into(),
            span: s(),
        },
        validators: validators
            .into_iter()
            .map(|(vname, args)| ValidatorCall {
                name: vname.into(),
                args,
                span: s(),
            })
            .collect(),
        span: s(),
    }
}

fn int_lit(val: i64) -> Expr {
    Expr::IntLit {
        value: val,
        span: s(),
    }
}

/// Emit JS guard and return its metadata.
fn emit_guard_meta(decl: &GuardDecl) -> GuardJsMeta {
    let mut e = JsEmitter::new();
    emit_guard_js::emit_guard_js_file(&mut e, decl)
}

// ── Runtime tests ──────────────────────────────────────────────────────

#[test]
fn runtime_contains_all_error_classes() {
    let js = emit_client_runtime::generate_runtime_js();
    assert!(js.contains("export class GaleValidationError extends Error"));
    assert!(js.contains("export class GaleServerError extends Error"));
    assert!(js.contains("export class GaleNetworkError extends Error"));
}

#[test]
fn runtime_fetch_wrapper_posts_to_action_endpoint() {
    let js = emit_client_runtime::generate_runtime_js();
    assert!(js.contains("export async function __gx_fetch(actionName, body)"));
    assert!(js.contains("/api/__gx/actions/"));
    assert!(js.contains("method: 'POST'"));
    assert!(js.contains("'Content-Type': 'application/json'"));
}

#[test]
fn runtime_fetch_classifies_validation_errors() {
    let js = emit_client_runtime::generate_runtime_js();
    assert!(js.contains("response.status === 400"));
    assert!(js.contains("responseBody.error === 'validation_failed'"));
    assert!(js.contains("GaleValidationError(actionName, responseBody.details)"));
}

#[test]
fn runtime_fetch_classifies_server_errors() {
    let js = emit_client_runtime::generate_runtime_js();
    assert!(js.contains("GaleServerError(actionName, response.status, responseBody)"));
}

#[test]
fn runtime_fetch_classifies_network_errors() {
    let js = emit_client_runtime::generate_runtime_js();
    assert!(js.contains("GaleNetworkError(actionName, err)"));
}

#[test]
fn runtime_query_cache_supports_mutate_rollback_invalidate() {
    let js = emit_client_runtime::generate_runtime_js();
    assert!(js.contains("GaleQueryCache.prototype.mutate"));
    assert!(js.contains("GaleQueryCache.prototype.rollback"));
    assert!(js.contains("GaleQueryCache.prototype.invalidate"));
}

#[test]
fn runtime_query_cache_singleton_exported() {
    let js = emit_client_runtime::generate_runtime_js();
    assert!(js.contains("export var queryCache = new GaleQueryCache()"));
}

#[test]
fn runtime_mutate_stores_and_restores_rollback() {
    let js = emit_client_runtime::generate_runtime_js();
    // mutate stores rollback
    assert!(js.contains("val._rollback = prev"));
    // rollback restores
    assert!(js.contains("val.data = val._rollback"));
    assert!(js.contains("delete val._rollback"));
}

// ── Action stub: no params ─────────────────────────────────────────────

#[test]
fn action_no_params_generates_stub() {
    let action = make_action("clearAll", vec![]);
    let out = emit_client_actions::generate_client_actions_js(&[action], &guards(&[]), &[]);

    assert!(out.contains("async clearAll()"));
    assert!(out.contains("__gx_fetch('clearAll')"));
}

// ── Action stub: single plain param ────────────────────────────────────

#[test]
fn action_single_plain_param() {
    let action = make_action("deleteUser", vec![param("userId", "int")]);
    let out = emit_client_actions::generate_client_actions_js(&[action], &guards(&[]), &[]);

    assert!(out.contains("async deleteUser(user_id)"));
    assert!(out.contains("__gx_fetch('deleteUser', user_id)"));
}

// ── Action stub: multi plain params ────────────────────────────────────

#[test]
fn action_multi_plain_params() {
    let action = make_action(
        "addItem",
        vec![param("name", "string"), param("count", "int")],
    );
    let out = emit_client_actions::generate_client_actions_js(&[action], &guards(&[]), &[]);

    assert!(out.contains("async addItem(name, count)"));
    assert!(out.contains("{ name, count }"));
}

// ── Action stub: guard param with validation ───────────────────────────

#[test]
fn action_guard_param_imports_and_validates() {
    let guard_decl = make_guard_decl(
        "UserForm",
        vec![make_field("email", "string", vec![("email", vec![])])],
    );
    let meta = emit_guard_meta(&guard_decl);

    let action = make_action("createUser", vec![param("data", "UserForm")]);
    let out =
        emit_client_actions::generate_client_actions_js(&[action], &guards(&["UserForm"]), &[meta]);

    // Import
    assert!(out.contains("import { validateUserForm } from '/js/guards/user_form.js'"));
    // Validation call
    assert!(out.contains("validateUserForm(data)"));
    // Error throw
    assert!(out.contains("GaleValidationError('createUser', result.errors)"));
    // POST
    assert!(out.contains("__gx_fetch('createUser', data)"));
}

#[test]
fn action_guard_with_sanitize_calls_sanitize_before_validate() {
    let guard_decl = make_guard_decl(
        "SignUpForm",
        vec![make_field(
            "name",
            "string",
            vec![("trim", vec![]), ("minLen", vec![int_lit(2)])],
        )],
    );
    let meta = emit_guard_meta(&guard_decl);
    assert!(meta.sanitize_fn.is_some(), "guard should have sanitize fn");

    let action = make_action("signup", vec![param("form", "SignUpForm")]);
    let out = emit_client_actions::generate_client_actions_js(
        &[action],
        &guards(&["SignUpForm"]),
        &[meta],
    );

    // Import includes both validate and sanitize
    assert!(out.contains("sanitizeSignUpForm"));
    assert!(out.contains("validateSignUpForm"));
    // Sanitize called first
    assert!(out.contains("sanitizeSignUpForm(form)"));
    // Validate on sanitized data
    assert!(out.contains("validateSignUpForm(sanitized)"));
    // POST with sanitized data
    assert!(out.contains("__gx_fetch('signup', sanitized)"));
}

// ── .withMutate() helper ───────────────────────────────────────────────

#[test]
fn action_has_with_mutate_helper() {
    let action = make_action("save", vec![]);
    let out = emit_client_actions::generate_client_actions_js(&[action], &guards(&[]), &[]);

    assert!(out.contains("save.withMutate = function(queryName, optimisticUpdater)"));
    assert!(out.contains("queryCache.mutate(queryName, optimisticUpdater)"));
    assert!(out.contains("await save(...args)"));
    assert!(out.contains("queryCache.invalidate(queryName)"));
    assert!(out.contains("queryCache.rollback(queryName)"));
}

#[test]
fn with_mutate_rolls_back_on_error() {
    let action = make_action("fail", vec![]);
    let out = emit_client_actions::generate_client_actions_js(&[action], &guards(&[]), &[]);

    // catch block should rollback
    assert!(out.contains("catch (err)"));
    assert!(out.contains("queryCache.rollback(queryName)"));
    assert!(out.contains("throw err"));
}

// ── Multiple actions ───────────────────────────────────────────────────

#[test]
fn multiple_actions_in_single_file() {
    let a = make_action("create", vec![]);
    let b = make_action("remove", vec![]);
    let out = emit_client_actions::generate_client_actions_js(&[a, b], &guards(&[]), &[]);

    assert!(out.contains("async create()"));
    assert!(out.contains("async remove()"));
    assert!(out.contains("create.withMutate"));
    assert!(out.contains("remove.withMutate"));
}

// ── Import structure ───────────────────────────────────────────────────

#[test]
fn runtime_imports_always_present() {
    let action = make_action("ping", vec![]);
    let out = emit_client_actions::generate_client_actions_js(&[action], &guards(&[]), &[]);
    assert!(out.contains(
        "import { __gx_fetch, GaleValidationError, queryCache } from '/_gale/runtime.js'"
    ));
}

#[test]
fn no_guard_import_when_no_guard_params() {
    let action = make_action("ping", vec![]);
    let out = emit_client_actions::generate_client_actions_js(&[action], &guards(&[]), &[]);
    assert!(!out.contains("/js/guards/"));
}

#[test]
fn guard_import_deduplicated_across_actions() {
    let guard_decl = make_guard_decl("MyGuard", vec![]);
    let meta = emit_guard_meta(&guard_decl);

    let a = make_action("actionA", vec![param("d", "MyGuard")]);
    let b = make_action("actionB", vec![param("d", "MyGuard")]);
    let out =
        emit_client_actions::generate_client_actions_js(&[a, b], &guards(&["MyGuard"]), &[meta]);

    let count = out.matches("from '/js/guards/my_guard.js'").count();
    assert_eq!(count, 1, "guard import should be deduplicated");
}

// ── ESM format ─────────────────────────────────────────────────────────

#[test]
fn output_is_valid_esm() {
    let action = make_action("test", vec![]);
    let out = emit_client_actions::generate_client_actions_js(&[action], &guards(&[]), &[]);
    assert!(out.contains("export "));
    assert!(out.contains("import "));
    assert!(!out.contains("module.exports"));
    assert!(!out.contains("require("));
}

// ── End-to-end through CodegenContext ───────────────────────────────────

#[test]
fn codegen_context_emits_client_js_for_actions() {
    let interner = TypeInterner::new();
    let program = Program {
        items: vec![Item::ActionDecl(ActionDecl {
            name: "createUser".into(),
            params: vec![],
            ret_ty: None,
            body: Block {
                stmts: vec![],
                span: s(),
            },
            span: s(),
        })],
        span: s(),
    };
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    // Should have the consolidated runtime
    assert!(
        ctx.files.contains("public/_gale/runtime.js"),
        "should emit runtime.js"
    );
    let runtime = ctx.files.get("public/_gale/runtime.js").unwrap();
    assert!(runtime.contains("GaleValidationError"));
    assert!(runtime.contains("__gx_fetch"));
    assert!(runtime.contains("GaleQueryCache"));

    // Should have the actions
    assert!(
        ctx.files.contains("public/_gale/actions.js"),
        "should emit actions.js"
    );
    let actions = ctx.files.get("public/_gale/actions.js").unwrap();
    assert!(actions.contains("async createUser()"));
    assert!(actions.contains("createUser.withMutate"));
}

#[test]
fn codegen_context_no_client_js_without_actions() {
    let interner = TypeInterner::new();
    let program = Program {
        items: vec![Item::GuardDecl(GuardDecl {
            name: "MyGuard".into(),
            fields: vec![],
            span: s(),
        })],
        span: s(),
    };
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    assert!(
        !ctx.files.contains("public/_gale/actions.js"),
        "no actions.js without actions"
    );
}

#[test]
fn codegen_context_action_with_guard_param() {
    let interner = TypeInterner::new();
    let program = Program {
        items: vec![
            Item::GuardDecl(GuardDecl {
                name: "UserForm".into(),
                fields: vec![GuardFieldDecl {
                    name: "email".into(),
                    ty: TypeAnnotation::Named {
                        name: "string".into(),
                        span: s(),
                    },
                    validators: vec![ValidatorCall {
                        name: "email".into(),
                        args: vec![],
                        span: s(),
                    }],
                    span: s(),
                }],
                span: s(),
            }),
            Item::ActionDecl(ActionDecl {
                name: "createUser".into(),
                params: vec![Param {
                    name: "data".into(),
                    ty_ann: Some(TypeAnnotation::Named {
                        name: "UserForm".into(),
                        span: s(),
                    }),
                    default: None,
                    span: s(),
                }],
                ret_ty: None,
                body: Block {
                    stmts: vec![],
                    span: s(),
                },
                span: s(),
            }),
        ],
        span: s(),
    };
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    // Should have the guard JS validator
    assert!(ctx.files.contains("static/js/guards/user_form.js"));

    // Action stubs should import the guard validator
    let actions = ctx.files.get("public/_gale/actions.js").unwrap();
    assert!(
        actions.contains("validateUserForm"),
        "should reference guard validator"
    );
    assert!(
        actions.contains("/js/guards/user_form.js"),
        "should import from guards dir"
    );
}

#[test]
fn codegen_context_multiple_actions() {
    let interner = TypeInterner::new();
    let program = Program {
        items: vec![
            Item::ActionDecl(ActionDecl {
                name: "create".into(),
                params: vec![],
                ret_ty: None,
                body: Block {
                    stmts: vec![],
                    span: s(),
                },
                span: s(),
            }),
            Item::ActionDecl(ActionDecl {
                name: "remove".into(),
                params: vec![],
                ret_ty: None,
                body: Block {
                    stmts: vec![],
                    span: s(),
                },
                span: s(),
            }),
        ],
        span: s(),
    };
    let mut ctx = CodegenContext::new(&interner, "test_app");
    ctx.emit_program(&program);

    let actions = ctx.files.get("public/_gale/actions.js").unwrap();
    assert!(actions.contains("async create()"));
    assert!(actions.contains("async remove()"));
}
