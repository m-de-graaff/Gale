//! REST API resource handler generator.
//!
//! For each GaleX [`ApiDecl`], generates a Rust file containing Axum
//! handlers for each HTTP method. Guards are used to validate query
//! parameters (GET) and request bodies (POST/PUT/PATCH).
//!
//! Convention-based status codes:
//! - GET → 200 (or 404 if Optional return & null)
//! - POST → 201
//! - PUT/PATCH → 200
//! - DELETE → 204
//! - Validation failure → 422

use std::collections::HashSet;

use crate::ast::*;
use crate::codegen::emit_stmt::{annotation_to_rust, emit_block_body};
use crate::codegen::rust_emitter::RustEmitter;
use crate::codegen::types::{collect_shared_type_refs, to_module_name, to_snake_case};

// ── Public API ─────────────────────────────────────────────────────────

/// Emit a complete API resource Rust file.
///
/// `known_guards` is the set of guard names declared in the program —
/// used to detect when a handler param is guard-typed (triggers validation).
pub fn emit_api_file(
    e: &mut RustEmitter,
    decl: &ApiDecl,
    known_guards: &HashSet<String>,
    known_shared_types: &HashSet<String>,
) {
    e.emit_file_header(&format!("API resource: `{}`.", decl.name));
    e.newline();

    // Imports
    e.emit_use("axum::extract::Json");
    e.emit_use("axum::http::StatusCode");
    let needs_query = decl
        .handlers
        .iter()
        .any(|h| h.method == HttpMethod::Get && !h.params.is_empty());
    let needs_path = decl.handlers.iter().any(|h| !h.path_params.is_empty());
    if needs_query {
        e.emit_use("axum::extract::Query");
    }
    if needs_path {
        e.emit_use("axum::extract::Path");
    }

    // Guard imports
    let mut imported_guards = HashSet::new();
    for handler in &decl.handlers {
        if let Some(guard_name) = find_guard_param(&handler.params, known_guards) {
            if imported_guards.insert(guard_name.clone()) {
                let guard_mod = to_module_name(&guard_name);
                e.emit_use(&format!("crate::guards::{guard_mod}::{guard_name}"));
            }
        }
    }

    // Shared type imports (enums, type aliases referenced in handler params/return types)
    if !known_shared_types.is_empty() {
        let mut annotations: Vec<&TypeAnnotation> = Vec::new();
        for handler in &decl.handlers {
            for p in &handler.params {
                if let Some(ann) = &p.ty_ann {
                    annotations.push(ann);
                }
            }
            if let Some(ann) = &handler.ret_ty {
                annotations.push(ann);
            }
        }
        for name in collect_shared_type_refs(&annotations, known_shared_types) {
            let mod_name = to_module_name(&name);
            e.emit_use(&format!("crate::shared::{mod_name}::{name}"));
        }
    }
    e.newline();

    // Input structs for non-guard, multi-param handlers
    for handler in &decl.handlers {
        if needs_input_struct(handler, known_guards) {
            emit_input_struct(e, &decl.name, handler);
            e.newline();
        }
    }

    // Handler functions
    for handler in &decl.handlers {
        emit_handler_fn(e, &decl.name, handler, known_guards);
        e.newline();
    }
}

/// Compute the route paths and method mappings for an API resource.
///
/// Groups handlers by their effective path. Returns `Vec<(path, Vec<(method_str, fn_name)>)>`.
pub fn api_route_groups(
    resource_name: &str,
    handlers: &[ApiHandler],
) -> Vec<(String, Vec<(String, String)>)> {
    let base_path = format!("/api/{}", pascal_to_kebab(resource_name));

    // Group handlers by path
    let mut groups: Vec<(String, Vec<(String, String)>)> = Vec::new();

    for handler in handlers {
        let path = if handler.path_params.is_empty() {
            base_path.clone()
        } else {
            let param_suffix: Vec<String> = handler
                .path_params
                .iter()
                .map(|p| format!("/:{p}"))
                .collect();
            format!("{}{}", base_path, param_suffix.join(""))
        };

        let method_str = match handler.method {
            HttpMethod::Get => "get",
            HttpMethod::Post => "post",
            HttpMethod::Put => "put",
            HttpMethod::Patch => "patch",
            HttpMethod::Delete => "delete",
        };

        let fn_name = handler_fn_name(handler);

        // Find or create group for this path
        if let Some(group) = groups.iter_mut().find(|(p, _)| p == &path) {
            group.1.push((method_str.to_string(), fn_name));
        } else {
            groups.push((path, vec![(method_str.to_string(), fn_name)]));
        }
    }

    groups
}

// ── Handler naming ─────────────────────────────────────────────────────

/// Derive the Rust function name for a handler.
fn handler_fn_name(handler: &ApiHandler) -> String {
    if handler.path_params.is_empty() {
        match handler.method {
            HttpMethod::Get => "list".into(),
            HttpMethod::Post => "create".into(),
            HttpMethod::Put => "replace".into(),
            HttpMethod::Patch => "patch".into(),
            HttpMethod::Delete => "delete_all".into(),
        }
    } else {
        let params_suffix = handler
            .path_params
            .iter()
            .map(|p| to_snake_case(p))
            .collect::<Vec<_>>()
            .join("_");
        match handler.method {
            HttpMethod::Get => format!("get_by_{params_suffix}"),
            HttpMethod::Post => format!("create_by_{params_suffix}"),
            HttpMethod::Put => format!("update_by_{params_suffix}"),
            HttpMethod::Patch => format!("patch_by_{params_suffix}"),
            HttpMethod::Delete => format!("delete_by_{params_suffix}"),
        }
    }
}

// ── Input structs ──────────────────────────────────────────────────────

/// Check if a handler needs a generated input struct.
fn needs_input_struct(handler: &ApiHandler, known_guards: &HashSet<String>) -> bool {
    if handler.params.is_empty() {
        return false;
    }
    // If single param is a guard, no input struct needed
    if handler.params.len() == 1 {
        if let Some(TypeAnnotation::Named { name, .. }) = &handler.params[0].ty_ann {
            if known_guards.contains(name.as_str()) {
                return false;
            }
        }
    }
    true
}

/// Emit an input struct for a handler with multiple non-guard params.
fn emit_input_struct(e: &mut RustEmitter, resource_name: &str, handler: &ApiHandler) {
    let fn_name = handler_fn_name(handler);
    let struct_name = format!(
        "{}{}Input",
        pascal_case(resource_name),
        pascal_case(&fn_name)
    );

    e.emit_attribute("derive(Debug, serde::Deserialize)");
    e.block(&format!("pub struct {struct_name}"), |e| {
        for p in &handler.params {
            let field_name = to_snake_case(&p.name);
            let ty = if let Some(ann) = &p.ty_ann {
                annotation_to_rust(ann)
            } else {
                "serde_json::Value".into()
            };
            e.writeln(&format!("pub {field_name}: {ty},"));
        }
    });
}

// ── Handler emission ───────────────────────────────────────────────────

/// Emit a single handler function for an API method.
fn emit_handler_fn(
    e: &mut RustEmitter,
    resource_name: &str,
    handler: &ApiHandler,
    known_guards: &HashSet<String>,
) {
    let fn_name = handler_fn_name(handler);
    let base_path = format!("/api/{}", pascal_to_kebab(resource_name));
    let full_path = if handler.path_params.is_empty() {
        base_path
    } else {
        let params: Vec<String> = handler
            .path_params
            .iter()
            .map(|p| format!("/:{p}"))
            .collect();
        format!("{}{}", base_path, params.join(""))
    };

    // Doc comment
    e.emit_doc_comment(&format!("{} {}", handler.method, full_path));

    // Determine guard param
    let guard_name = find_guard_param(&handler.params, known_guards);
    let has_guard = guard_name.is_some();

    // Build function signature
    let sig = build_handler_signature(handler, resource_name, known_guards);
    let ret = build_return_type(handler);

    e.block(&format!("pub async fn {fn_name}({sig}) -> {ret}"), |e| {
        // Guard validation
        if has_guard {
            emit_guard_validation(e, handler.method);
        }

        // Bind params
        emit_param_bindings(e, handler, known_guards);

        // Handler body
        e.emit_comment("--- Handler body ---");
        emit_block_body(e, &handler.body);

        // Default return based on method
        e.newline();
        emit_default_return(e, handler.method);
    });
}

/// Build the function parameter string for a handler.
fn build_handler_signature(
    handler: &ApiHandler,
    resource_name: &str,
    known_guards: &HashSet<String>,
) -> String {
    let mut parts = Vec::new();

    // Path params
    if !handler.path_params.is_empty() {
        if handler.path_params.len() == 1 {
            let p = to_snake_case(&handler.path_params[0]);
            parts.push(format!("Path({p}): Path<String>"));
        } else {
            let names: Vec<String> = handler
                .path_params
                .iter()
                .map(|p| to_snake_case(p))
                .collect();
            let types = vec!["String"; handler.path_params.len()].join(", ");
            parts.push(format!("Path(({})):  Path<({})>", names.join(", "), types));
        }
    }

    // Body/query params
    if !handler.params.is_empty() {
        let guard_name = find_guard_param(&handler.params, known_guards);

        let input_type = if let Some(ref gn) = guard_name {
            gn.clone()
        } else if handler.params.len() == 1 {
            if let Some(ann) = &handler.params[0].ty_ann {
                annotation_to_rust(ann)
            } else {
                "serde_json::Value".into()
            }
        } else {
            let fn_name = handler_fn_name(handler);
            format!(
                "{}{}Input",
                pascal_case(resource_name),
                pascal_case(&fn_name)
            )
        };

        match handler.method {
            HttpMethod::Get => {
                parts.push(format!("Query(input): Query<{input_type}>"));
            }
            _ => {
                parts.push(format!("Json(input): Json<{input_type}>"));
            }
        }
    }

    parts.join(",\n    ")
}

/// Build the return type for a handler.
fn build_return_type(handler: &ApiHandler) -> String {
    match handler.method {
        HttpMethod::Delete => "StatusCode".into(),
        HttpMethod::Post => {
            "Result<(StatusCode, Json<serde_json::Value>), (StatusCode, Json<serde_json::Value>)>"
                .into()
        }
        _ => "Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)>".into(),
    }
}

/// Emit guard validation code.
fn emit_guard_validation(e: &mut RustEmitter, method: HttpMethod) {
    let status = match method {
        HttpMethod::Post => "StatusCode::UNPROCESSABLE_ENTITY",
        _ => "StatusCode::UNPROCESSABLE_ENTITY",
    };

    e.block("if let Err(errors) = input.validate()", |e| {
        let err_return = match method {
            HttpMethod::Delete => format!("return {status};"),
            _ => format!(
                "return Err(({status}, Json(serde_json::json!({{ \
                 \"error\": \"validation_failed\", \"details\": errors \
                 }}))));"
            ),
        };
        e.writeln(&err_return);
    });
    e.newline();
}

/// Emit parameter binding statements.
fn emit_param_bindings(e: &mut RustEmitter, handler: &ApiHandler, known_guards: &HashSet<String>) {
    if handler.params.is_empty() {
        return;
    }

    let guard_name = find_guard_param(&handler.params, known_guards);

    if handler.params.len() == 1 || guard_name.is_some() {
        let p = &handler.params[0];
        let pname = to_snake_case(&p.name);
        e.writeln(&format!("let {pname} = input;"));
    } else {
        for p in &handler.params {
            let pname = to_snake_case(&p.name);
            e.writeln(&format!("let {pname} = input.{pname};"));
        }
    }
    e.newline();
}

/// Emit the default return statement for a handler.
fn emit_default_return(e: &mut RustEmitter, method: HttpMethod) {
    match method {
        HttpMethod::Get => {
            e.writeln("Ok(Json(serde_json::json!(null)))");
        }
        HttpMethod::Post => {
            e.writeln("Ok((StatusCode::CREATED, Json(serde_json::json!(null))))");
        }
        HttpMethod::Put | HttpMethod::Patch => {
            e.writeln("Ok(Json(serde_json::json!(null)))");
        }
        HttpMethod::Delete => {
            e.writeln("StatusCode::NO_CONTENT");
        }
    }
}

// ── Helpers ────────────────────────────────────────────────────────────

/// Find the first param whose type annotation references a known guard.
fn find_guard_param(params: &[Param], known_guards: &HashSet<String>) -> Option<String> {
    for p in params {
        if let Some(TypeAnnotation::Named { name, .. }) = &p.ty_ann {
            if known_guards.contains(name.as_str()) {
                return Some(name.to_string());
            }
        }
    }
    None
}

/// Convert PascalCase to kebab-case.
fn pascal_to_kebab(name: &str) -> String {
    let mut result = String::with_capacity(name.len() + 4);
    for (i, ch) in name.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 {
                result.push('-');
            }
            result.push(ch.to_lowercase().next().unwrap_or(ch));
        } else {
            result.push(ch);
        }
    }
    result
}

/// Convert a camelCase/snake_case name to PascalCase.
fn pascal_case(name: &str) -> String {
    let mut result = String::with_capacity(name.len());
    let mut next_upper = true;
    for ch in name.chars() {
        if ch == '_' {
            next_upper = true;
        } else if next_upper {
            result.push(ch.to_uppercase().next().unwrap_or(ch));
            next_upper = false;
        } else {
            result.push(ch);
        }
    }
    result
}
