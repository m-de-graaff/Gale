//! Form validation runtime and SSR wiring script generation.
//!
//! Generates two kinds of JS output:
//!
//! 1. **`gale-forms.js`** — A static runtime module providing `wireForm()`,
//!    which connects guard validators to DOM forms. Emitted once per project.
//!
//! 2. **Per-page wiring `<script>`** — Inline ES module scripts injected into
//!    SSR output that import the guard validator + runtime and wire a specific
//!    form element. Emitted via Rust codegen in [`emit_form_wiring_script`].

use crate::codegen::emit_guard_js::GuardJsMeta;
use crate::codegen::js_emitter::JsEmitter;
use crate::codegen::rust_emitter::RustEmitter;

// ── Static runtime: gale-forms.js ──────────────────────────────────────

/// Generate the `gale-forms.js` runtime module.
///
/// This is a static JS file emitted once during `finalize()`. It provides
/// the `wireForm` function that connects guard validators to form elements,
/// handling submit and blur validation events and `data-gale-error` display.
pub fn emit_gale_forms_runtime() -> String {
    let mut e = JsEmitter::new();
    e.emit_file_header("GaleX form validation runtime.");
    e.newline();

    // ── getFormData helper ──────────────────────────────────────
    e.emit_comment("Extract form field values as a plain object.");
    e.emit_fn("getFormData", &["form"], |e| {
        e.writeln("const fd = new FormData(form);");
        e.writeln("const data = {};");
        e.emit_for_of("entry", "fd.entries()", |e| {
            e.writeln("const [key, value] = entry;");
            e.emit_if("!key.startsWith('__')", |e| {
                e.writeln("data[key] = value;");
            });
        });
        e.writeln("return data;");
    });
    e.newline();

    // ── clearErrors helper ─────────────────────────────────────
    e.emit_comment("Clear all error messages in a form.");
    e.emit_fn("clearErrors", &["form"], |e| {
        e.writeln("form.querySelectorAll('[data-gale-error]').forEach(function(el) {");
        e.indent();
        e.writeln("el.textContent = '';");
        e.writeln("el.hidden = true;");
        e.dedent();
        e.writeln("});");
    });
    e.newline();

    // ── showErrors helper ──────────────────────────────────────
    e.emit_comment("Display validation errors in their corresponding error elements.");
    e.emit_fn("showErrors", &["form", "errors"], |e| {
        e.emit_for_of("err", "errors", |e| {
            e.writeln("const el = form.querySelector('[data-gale-error=\"' + err.field + '\"]');");
            e.emit_if("el", |e| {
                e.writeln("el.textContent = err.message;");
                e.writeln("el.hidden = false;");
            });
        });
    });
    e.newline();

    // ── showFieldError helper ──────────────────────────────────
    e.emit_comment("Show/hide error for a single field.");
    e.emit_fn("showFieldError", &["form", "field", "errors"], |e| {
        e.writeln("const fieldErr = errors.find(function(e) { return e.field === field; });");
        e.writeln("const el = form.querySelector('[data-gale-error=\"' + field + '\"]');");
        e.emit_if("!el", |e| {
            e.writeln("return;");
        });
        e.emit_if_else(
            "fieldErr",
            |e| {
                e.writeln("el.textContent = fieldErr.message;");
                e.writeln("el.hidden = false;");
            },
            |e| {
                e.writeln("el.textContent = '';");
                e.writeln("el.hidden = true;");
            },
        );
    });
    e.newline();

    // ── wireForm (main export) ─────────────────────────────────
    e.emit_comment("Wire a form element to a guard validator.");
    e.emit_comment("");
    e.emit_comment("Options:");
    e.emit_comment("  validate  — function(data) => { ok, data?, errors? }");
    e.emit_comment("  sanitize  — function(data) => data (optional)");
    e.emit_comment("  fields    — string[] of field names for blur validation");
    e.emit_comment("  onResult  — function(json) called with the server response (optional)");
    e.emit_export_fn("wireForm", &["form", "opts"], |e| {
        e.writeln("const validate = opts.validate;");
        e.writeln("const sanitize = opts.sanitize;");
        e.writeln("const fields = opts.fields;");
        e.writeln("const onResult = opts.onResult || null;");
        e.newline();

        // ── Blur validation (per-field) ────────────────────────
        e.emit_comment("Blur validation: validate the field the user just left.");
        e.emit_for_of("field", "fields", |e| {
            e.writeln("const input = form.querySelector('[name=\"' + field + '\"]');");
            e.emit_if("input", |e| {
                e.writeln("input.addEventListener('blur', function() {");
                e.indent();
                e.writeln("const raw = getFormData(form);");
                e.writeln("const data = sanitize ? sanitize(raw) : raw;");
                e.writeln("const result = validate(data);");
                e.writeln("const errs = result.ok ? [] : result.errors;");
                e.writeln("showFieldError(form, field, errs);");
                e.dedent();
                e.writeln("});");
            });
        });
        e.newline();

        // ── Submit: always use fetch(), never native form POST ─
        e.emit_comment("Submit: always preventDefault and POST via fetch with JSON.");
        e.writeln("form.addEventListener('submit', function(e) {");
        e.indent();
        e.writeln("e.preventDefault();");
        e.writeln("clearErrors(form);");
        e.writeln("const raw = getFormData(form);");
        e.writeln("const data = sanitize ? sanitize(raw) : raw;");
        e.writeln("const result = validate(data);");
        e.emit_if("!result.ok", |e| {
            e.writeln("showErrors(form, result.errors);");
            e.writeln("return;");
        });
        e.writeln("const action = form.getAttribute('action');");
        e.writeln("fetch(action, {");
        e.indent();
        e.writeln("method: 'POST',");
        e.writeln("headers: { 'Content-Type': 'application/json' },");
        e.writeln("body: JSON.stringify(data)");
        e.dedent();
        e.writeln("})");
        e.writeln(".then(function(res) { return res.json(); })");
        e.writeln(".then(function(json) {");
        e.indent();
        e.writeln("if (onResult) onResult(json);");
        e.dedent();
        e.writeln("})");
        e.writeln(".catch(function(err) {");
        e.indent();
        e.writeln("console.error('Form action failed:', err);");
        e.dedent();
        e.writeln("});");
        e.dedent();
        e.writeln("});");
    });

    e.finish()
}

// ── SSR wiring script injection ────────────────────────────────────────

/// Emit Rust code that appends a `<script type="module">` block to the
/// SSR `html` buffer, wiring a form to its guard validator.
///
/// Called during SSR template rendering when a form has `form:guard`.
/// The generated Rust code pushes HTML strings into the `html` variable.
///
/// # Arguments
///
/// * `e` — The Rust emitter (SSR context, writing to `html: String`)
/// * `meta` — JS guard metadata (function names, field list)
pub fn emit_form_wiring_script(e: &mut RustEmitter, meta: &GuardJsMeta) {
    let guard_name = &meta.guard_name;
    let module_name = &meta.module_name;
    let validate_fn = &meta.validate_fn;

    // Build the import list
    let mut imports = vec![validate_fn.as_str()];
    if let Some(ref san_fn) = meta.sanitize_fn {
        imports.push(san_fn.as_str());
    }
    let import_list = imports.join(", ");

    // Build the fields array literal
    let fields_js: Vec<String> = meta.fields.iter().map(|f| format!("'{f}'")).collect();
    let fields_literal = format!("[{}]", fields_js.join(", "));

    // Sanitize option
    let sanitize_opt = match &meta.sanitize_fn {
        Some(f) => f.clone(),
        None => "null".to_string(),
    };

    e.writeln("// Form validation wiring script");
    e.writeln("html.push_str(\"<script type=\\\"module\\\">\");");
    e.writeln(&format!(
        "html.push_str(\"import {{ {import_list} }} from '/js/guards/{module_name}.js';\");",
    ));
    e.writeln("html.push_str(\"import { wireForm } from '/js/gale-forms.js';\");");
    e.writeln(&format!(
        "html.push_str(\"var __f=document.querySelector('[data-gale-guard=\\\\\\\"{guard_name}\\\\\\\"]');\");",
    ));
    e.writeln("html.push_str(\"if(__f)wireForm(__f,{\");");
    e.writeln(&format!("html.push_str(\"validate:{validate_fn},\");",));
    e.writeln(&format!("html.push_str(\"sanitize:{sanitize_opt},\");",));
    e.writeln(&format!("html.push_str(\"fields:{fields_literal},\");",));
    // Wire onResult to update the 'result' signal if it exists in the HMR registry.
    e.writeln("html.push_str(\"onResult:function(j){if(window.__gale_signals__&&window.__gale_signals__.result)window.__gale_signals__.result.set(typeof j==='string'?j:JSON.stringify(j,null,2))}\");");
    e.writeln("html.push_str(\"});\");");
    e.writeln("html.push_str(\"</script>\");");
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gale_forms_runtime_contains_wire_form() {
        let out = emit_gale_forms_runtime();
        assert!(out.contains("export function wireForm(form, opts)"));
    }

    #[test]
    fn gale_forms_runtime_contains_get_form_data() {
        let out = emit_gale_forms_runtime();
        assert!(out.contains("function getFormData(form)"));
        assert!(out.contains("new FormData(form)"));
    }

    #[test]
    fn gale_forms_runtime_contains_clear_errors() {
        let out = emit_gale_forms_runtime();
        assert!(out.contains("function clearErrors(form)"));
        assert!(out.contains("data-gale-error"));
    }

    #[test]
    fn gale_forms_runtime_contains_show_errors() {
        let out = emit_gale_forms_runtime();
        assert!(out.contains("function showErrors(form, errors)"));
    }

    #[test]
    fn gale_forms_runtime_has_blur_handler() {
        let out = emit_gale_forms_runtime();
        assert!(out.contains("addEventListener('blur'"));
    }

    #[test]
    fn gale_forms_runtime_has_submit_handler() {
        let out = emit_gale_forms_runtime();
        assert!(out.contains("addEventListener('submit'"));
        assert!(out.contains("e.preventDefault()"));
        // Must use fetch with JSON, not native form POST
        assert!(out.contains("fetch(action"));
        assert!(out.contains("'Content-Type': 'application/json'"));
        assert!(out.contains("JSON.stringify(data)"));
    }

    #[test]
    fn gale_forms_runtime_skips_hidden_fields() {
        let out = emit_gale_forms_runtime();
        assert!(out.contains("!key.startsWith('__')"));
    }

    #[test]
    fn wiring_script_emits_module_import() {
        let meta = GuardJsMeta {
            guard_name: "LoginForm".into(),
            module_name: "login_form".into(),
            validate_fn: "validateLoginForm".into(),
            sanitize_fn: None,
            fields: vec!["email".into(), "password".into()],
        };
        let mut e = RustEmitter::new();
        emit_form_wiring_script(&mut e, &meta);
        let out = e.finish();

        assert!(out.contains("script type=\\\"module\\\""));
        assert!(out.contains("import { validateLoginForm }"));
        assert!(out.contains("/js/guards/login_form.js"));
        assert!(out.contains("gale-forms.js"));
    }

    #[test]
    fn wiring_script_includes_sanitize_when_present() {
        let meta = GuardJsMeta {
            guard_name: "SignUp".into(),
            module_name: "sign_up".into(),
            validate_fn: "validateSignUp".into(),
            sanitize_fn: Some("sanitizeSignUp".into()),
            fields: vec!["name".into()],
        };
        let mut e = RustEmitter::new();
        emit_form_wiring_script(&mut e, &meta);
        let out = e.finish();

        assert!(out.contains("import { validateSignUp, sanitizeSignUp }"));
        assert!(out.contains("sanitize:sanitizeSignUp"));
    }

    #[test]
    fn wiring_script_null_sanitize_when_absent() {
        let meta = GuardJsMeta {
            guard_name: "Simple".into(),
            module_name: "simple".into(),
            validate_fn: "validateSimple".into(),
            sanitize_fn: None,
            fields: vec!["x".into()],
        };
        let mut e = RustEmitter::new();
        emit_form_wiring_script(&mut e, &meta);
        let out = e.finish();

        assert!(out.contains("sanitize:null"));
    }

    #[test]
    fn wiring_script_includes_fields_array() {
        let meta = GuardJsMeta {
            guard_name: "G".into(),
            module_name: "g".into(),
            validate_fn: "validateG".into(),
            sanitize_fn: None,
            fields: vec!["email".into(), "name".into(), "age".into()],
        };
        let mut e = RustEmitter::new();
        emit_form_wiring_script(&mut e, &meta);
        let out = e.finish();

        assert!(out.contains("['email', 'name', 'age']"));
    }

    #[test]
    fn wiring_script_uses_data_gale_guard_selector() {
        let meta = GuardJsMeta {
            guard_name: "MyForm".into(),
            module_name: "my_form".into(),
            validate_fn: "validateMyForm".into(),
            sanitize_fn: None,
            fields: vec![],
        };
        let mut e = RustEmitter::new();
        emit_form_wiring_script(&mut e, &meta);
        let out = e.finish();

        assert!(out.contains("data-gale-guard="));
        assert!(out.contains("MyForm"));
    }
}
