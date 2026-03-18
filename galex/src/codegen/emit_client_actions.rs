//! Client-side action stub generator — `public/_gale/actions.js`.
//!
//! For each [`ActionDecl`] in the program, generates an exported async
//! function that:
//!
//! 1. Validates arguments client-side (using existing guard JS validators)
//! 2. Sanitizes transforms (trim, precision) if the guard has them
//! 3. Serializes arguments to JSON
//! 4. POSTs to `/api/__gx/actions/{actionName}`
//! 5. Deserializes the response
//! 6. Returns the typed result
//!
//! Each action also gets a `.withMutate(queryName, optimisticUpdater)`
//! helper for optimistic updates via the query cache.
//!
//! Integrates with the existing guard JS system in `emit_guard_js`:
//! - Guard validators live at `static/js/guards/{name}.js`
//! - Functions are `validate{Guard}(data)` → `{ ok, data?, errors? }`
//! - Optional `sanitize{Guard}(data)` → sanitized data copy

use std::collections::HashSet;

use crate::ast::*;
use crate::codegen::emit_guard_js::GuardJsMeta;
use crate::codegen::js_emitter::JsEmitter;
use crate::codegen::types::to_snake_case;

// ── Public entry point ─────────────────────────────────────────────────

/// Generate the complete `public/_gale/actions.js` file.
///
/// `actions` — all ActionDecl nodes from the program.
/// `known_guards` — guard names (to detect guard-typed params).
/// `guard_meta` — metadata for existing JS guard modules (for import paths).
pub fn generate_client_actions_js(
    actions: &[ActionDecl],
    known_guards: &HashSet<String>,
    guard_meta: &[GuardJsMeta],
) -> String {
    let mut e = JsEmitter::new();
    e.emit_file_header("GaleX action stubs — client-side RPC bridge.");
    e.newline();

    // ── Imports ────────────────────────────────────────────────
    // Runtime (error classes, fetch wrapper, query cache)
    e.emit_import(
        &["__gx_fetch", "GaleValidationError", "queryCache"],
        "/_gale/runtime.js",
    );

    // Guard imports — collect unique guards referenced by action params
    let mut imported_guards: Vec<&GuardJsMeta> = Vec::new();
    for action in actions {
        if let Some((_, guard_name)) = find_guard_param(&action.params, known_guards) {
            if !imported_guards.iter().any(|m| m.guard_name == guard_name) {
                if let Some(meta) = guard_meta.iter().find(|m| m.guard_name == guard_name) {
                    imported_guards.push(meta);
                }
            }
        }
    }

    for meta in &imported_guards {
        let mut import_names: Vec<&str> = vec![&meta.validate_fn];
        if let Some(ref san_fn) = meta.sanitize_fn {
            import_names.push(san_fn);
        }
        e.emit_import(
            &import_names,
            &format!("/js/guards/{}.js", meta.module_name),
        );
    }

    // ── Action stubs ──────────────────────────────────────────
    for action in actions {
        e.newline();
        emit_action_stub(&mut e, action, known_guards, guard_meta);
    }

    e.finish()
}

// ── Per-action emission ────────────────────────────────────────────────

/// Emit a single action stub function + `.withMutate()` helper.
fn emit_action_stub(
    e: &mut JsEmitter,
    decl: &ActionDecl,
    known_guards: &HashSet<String>,
    guard_meta: &[GuardJsMeta],
) {
    let action_name = &decl.name;
    let guard_param = find_guard_param(&decl.params, known_guards);

    // Resolve guard metadata if this action has a guard param
    let meta = guard_param
        .as_ref()
        .and_then(|(_, name)| guard_meta.iter().find(|m| m.guard_name == *name));

    e.emit_comment(&format!("POST /api/__gx/actions/{action_name}"));

    match (&guard_param, decl.params.len()) {
        // ── Guard-param action (single guard-typed param) ──────
        (Some(_), _) => {
            let param_name = to_snake_case(&decl.params[0].name);
            e.emit_export_fn(&format!("async {action_name}"), &[&param_name], |e| {
                if let Some(meta) = meta {
                    // Sanitize first (if transforms exist)
                    if let Some(ref san_fn) = meta.sanitize_fn {
                        e.writeln(&format!("const sanitized = {san_fn}({param_name});"));
                        // Validate sanitized data
                        e.writeln(&format!("const result = {}(sanitized);", meta.validate_fn));
                    } else {
                        e.writeln(&format!(
                            "const result = {}({param_name});",
                            meta.validate_fn
                        ));
                    }

                    e.emit_if("!result.ok", |e| {
                        e.writeln(&format!(
                            "throw new GaleValidationError('{action_name}', result.errors);"
                        ));
                    });

                    // POST with sanitized data (or original if no sanitize)
                    if meta.sanitize_fn.is_some() {
                        e.writeln(&format!("return __gx_fetch('{action_name}', sanitized);"));
                    } else {
                        e.writeln(&format!(
                            "return __gx_fetch('{action_name}', {param_name});"
                        ));
                    }
                } else {
                    // Guard meta not found — skip validation, just POST
                    e.writeln(&format!(
                        "return __gx_fetch('{action_name}', {param_name});"
                    ));
                }
            });
        }

        // ── Multi-param action (plain params) ──────────────────
        (None, n) if n > 1 => {
            let param_names: Vec<String> =
                decl.params.iter().map(|p| to_snake_case(&p.name)).collect();
            let param_refs: Vec<&str> = param_names.iter().map(|s| s.as_str()).collect();
            let obj_fields = param_names.join(", ");
            e.emit_export_fn(&format!("async {action_name}"), &param_refs, |e| {
                e.writeln(&format!(
                    "return __gx_fetch('{action_name}', {{ {obj_fields} }});"
                ));
            });
        }

        // ── Single plain param ─────────────────────────────────
        (None, 1) => {
            let param_name = to_snake_case(&decl.params[0].name);
            e.emit_export_fn(&format!("async {action_name}"), &[&param_name], |e| {
                e.writeln(&format!(
                    "return __gx_fetch('{action_name}', {param_name});"
                ));
            });
        }

        // ── No-param action ────────────────────────────────────
        (None, 0) => {
            e.emit_export_fn(&format!("async {action_name}"), &[], |e| {
                e.writeln(&format!("return __gx_fetch('{action_name}');"));
            });
        }

        _ => unreachable!(),
    }

    // ── .withMutate() helper ───────────────────────────────────
    emit_with_mutate(e, action_name);
}

/// Emit the `.withMutate(queryName, optimisticUpdater)` helper on an action.
///
/// Returns a wrapper function that:
/// 1. Applies the optimistic update via `queryCache.mutate()`
/// 2. Calls the original action
/// 3. On success: invalidates the query (re-fetches authoritative data)
/// 4. On error: rolls back the optimistic update and re-throws
fn emit_with_mutate(e: &mut JsEmitter, action_name: &str) {
    e.newline();
    e.emit_comment("Optimistic update wrapper — applies mutation before the action,");
    e.emit_comment("invalidates on success, rolls back on failure.");
    e.writeln(&format!(
        "{action_name}.withMutate = function(queryName, optimisticUpdater) {{"
    ));
    e.indent();
    e.block("return async function(...args)", |e| {
        e.emit_if("optimisticUpdater !== undefined", |e| {
            e.writeln("queryCache.mutate(queryName, optimisticUpdater);");
        });
        e.block("try", |e| {
            e.writeln(&format!("const result = await {action_name}(...args);"));
            e.writeln("queryCache.invalidate(queryName);");
            e.writeln("return result;");
        });
        e.block("catch (err)", |e| {
            e.writeln("queryCache.rollback(queryName);");
            e.writeln("throw err;");
        });
    });
    e.dedent();
    e.writeln("};");
}

// ── Helpers ────────────────────────────────────────────────────────────

/// Find the first param whose type annotation references a known guard.
///
/// Returns `(param_index, guard_name)`.
fn find_guard_param(params: &[Param], known_guards: &HashSet<String>) -> Option<(usize, String)> {
    for (i, p) in params.iter().enumerate() {
        if let Some(TypeAnnotation::Named { name, .. }) = &p.ty_ann {
            if known_guards.contains(name.as_str()) {
                return Some((i, name.to_string()));
            }
        }
    }
    None
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::span::Span;

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

    fn make_meta(guard_name: &str, has_sanitize: bool) -> GuardJsMeta {
        GuardJsMeta {
            guard_name: guard_name.into(),
            module_name: crate::codegen::types::to_module_name(guard_name),
            validate_fn: format!("validate{guard_name}"),
            sanitize_fn: if has_sanitize {
                Some(format!("sanitize{guard_name}"))
            } else {
                None
            },
            fields: vec![],
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

    // ── Basic action patterns ──────────────────────────────────

    #[test]
    fn action_no_params() {
        let action = make_action("clearAll", vec![]);
        let out = generate_client_actions_js(&[action], &guards(&[]), &[]);
        assert!(out.contains("export function async clearAll()"));
        assert!(out.contains("__gx_fetch('clearAll')"));
    }

    #[test]
    fn action_single_plain_param() {
        let action = make_action("deleteUser", vec![param("userId", "int")]);
        let out = generate_client_actions_js(&[action], &guards(&[]), &[]);
        assert!(out.contains("export function async deleteUser(user_id)"));
        assert!(out.contains("__gx_fetch('deleteUser', user_id)"));
    }

    #[test]
    fn action_multi_plain_params() {
        let action = make_action(
            "addItem",
            vec![param("name", "string"), param("count", "int")],
        );
        let out = generate_client_actions_js(&[action], &guards(&[]), &[]);
        assert!(out.contains("export function async addItem(name, count)"));
        assert!(out.contains("__gx_fetch('addItem', { name, count })"));
    }

    // ── Guard param integration ────────────────────────────────

    #[test]
    fn action_with_guard_param_validates() {
        let action = make_action("createUser", vec![param("data", "UserForm")]);
        let meta = make_meta("UserForm", false);
        let out = generate_client_actions_js(&[action], &guards(&["UserForm"]), &[meta]);

        assert!(out.contains("import { validateUserForm } from '/js/guards/user_form.js'"));
        assert!(out.contains("validateUserForm(data)"));
        assert!(out.contains("GaleValidationError('createUser', result.errors)"));
        assert!(out.contains("__gx_fetch('createUser', data)"));
    }

    #[test]
    fn action_with_guard_sanitize_calls_sanitize_first() {
        let action = make_action("signup", vec![param("form", "SignUpForm")]);
        let meta = make_meta("SignUpForm", true);
        let out = generate_client_actions_js(&[action], &guards(&["SignUpForm"]), &[meta]);

        assert!(out.contains("import { validateSignUpForm, sanitizeSignUpForm }"));
        assert!(out.contains("sanitizeSignUpForm(form)"));
        assert!(out.contains("validateSignUpForm(sanitized)"));
        assert!(out.contains("__gx_fetch('signup', sanitized)"));
    }

    #[test]
    fn no_guard_import_when_not_needed() {
        let action = make_action("ping", vec![]);
        let out = generate_client_actions_js(&[action], &guards(&[]), &[]);
        assert!(!out.contains("/js/guards/"));
    }

    // ── .withMutate() helper ───────────────────────────────────

    #[test]
    fn action_has_with_mutate_helper() {
        let action = make_action("save", vec![]);
        let out = generate_client_actions_js(&[action], &guards(&[]), &[]);

        assert!(out.contains("save.withMutate = function(queryName, optimisticUpdater)"));
        assert!(out.contains("queryCache.mutate(queryName, optimisticUpdater)"));
        assert!(out.contains("await save(...args)"));
        assert!(out.contains("queryCache.invalidate(queryName)"));
        assert!(out.contains("queryCache.rollback(queryName)"));
    }

    // ── Multiple actions ───────────────────────────────────────

    #[test]
    fn multiple_actions_single_file() {
        let a = make_action("create", vec![]);
        let b = make_action("remove", vec![]);
        let out = generate_client_actions_js(&[a, b], &guards(&[]), &[]);
        assert!(out.contains("async create()"));
        assert!(out.contains("async remove()"));
    }

    // ── Import structure ───────────────────────────────────────

    #[test]
    fn runtime_imports_always_present() {
        let action = make_action("ping", vec![]);
        let out = generate_client_actions_js(&[action], &guards(&[]), &[]);
        assert!(out.contains(
            "import { __gx_fetch, GaleValidationError, queryCache } from '/_gale/runtime.js'"
        ));
    }

    #[test]
    fn deduplicates_guard_imports() {
        let a = make_action("actionA", vec![param("d", "MyGuard")]);
        let b = make_action("actionB", vec![param("d", "MyGuard")]);
        let meta = make_meta("MyGuard", false);
        let out = generate_client_actions_js(&[a, b], &guards(&["MyGuard"]), &[meta]);

        // Should appear exactly once
        let count = out.matches("from '/js/guards/my_guard.js'").count();
        assert_eq!(count, 1, "guard import should be deduplicated");
    }

    // ── Output format ──────────────────────────────────────────

    #[test]
    fn output_is_valid_esm() {
        let action = make_action("test", vec![]);
        let out = generate_client_actions_js(&[action], &guards(&[]), &[]);
        assert!(out.contains("export "));
        assert!(out.contains("import "));
        assert!(!out.contains("module.exports"));
        assert!(!out.contains("require("));
    }

    #[test]
    fn has_route_comment() {
        let action = make_action("login", vec![]);
        let out = generate_client_actions_js(&[action], &guards(&[]), &[]);
        assert!(out.contains("// POST /api/__gx/actions/login"));
    }
}
