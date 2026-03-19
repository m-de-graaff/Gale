//! Per-page client hydration script generator.
//!
//! For each GaleX component with interactive elements, generates a
//! JavaScript ES module that imports from the GaleX runtime, recreates
//! the component's reactive graph, and calls `hydrate()` to attach
//! reactivity to the SSR-rendered HTML.
//!
//! The generated script also sets up reactive `when` blocks (via
//! `replaceRegion`) and `each` blocks (via `reconcileList`) that
//! re-render DOM regions when their reactive dependencies change.

use std::collections::HashSet;

use crate::ast::*;
use crate::codegen::js_expr::expr_to_js;

// ── Public API ─────────────────────────────────────────────────────────

/// Check whether a component has any interactive (client-side) elements.
///
/// A component is interactive if its body contains signal/derive/effect/watch/ref
/// declarations, OR if its template contains bind:/on:/ref:/class:/transition:
/// directives, OR if it has reactive `when`/`each` blocks.
pub fn component_has_client_code(decl: &ComponentDecl) -> bool {
    // Check stmts for reactive constructs
    for stmt in &decl.body.stmts {
        match stmt {
            Stmt::Signal { .. }
            | Stmt::Derive { .. }
            | Stmt::Effect { .. }
            | Stmt::Watch { .. }
            | Stmt::RefDecl { .. } => return true,
            _ => {}
        }
    }
    // Check template for interactive directives or reactive control flow
    template_has_interactivity(&decl.body.template)
}

/// Generate the per-page hydration script for a component.
///
/// Returns the JavaScript source as a `String`. The result is an ES module
/// that imports from `/_gale/runtime.js`, recreates reactive state, and
/// calls `hydrate()` with per-element instructions.
pub fn emit_page_script(decl: &ComponentDecl) -> String {
    let mut js = String::with_capacity(1024);

    // Collect signal names for expression conversion
    let signal_names = collect_signal_names(&decl.body.stmts);

    // Determine which runtime features are used
    let mut imports = HashSet::new();
    imports.insert("_readData");

    // Scan stmts for reactive constructs
    let mut signals = Vec::new();
    let mut derives = Vec::new();
    let mut effects = Vec::new();
    let mut watches = Vec::new();
    let mut refs = Vec::new();

    for stmt in &decl.body.stmts {
        match stmt {
            Stmt::Signal { name, init, .. } => {
                imports.insert("signal");
                imports.insert("_registerSignal");
                signals.push((name.as_str(), init));
            }
            Stmt::Derive { name, init, .. } => {
                imports.insert("derive");
                derives.push((name.as_str(), init));
            }
            Stmt::Effect { body, .. } => {
                imports.insert("effect");
                effects.push(body);
            }
            Stmt::Watch {
                target,
                next_name,
                prev_name,
                body,
                ..
            } => {
                imports.insert("watch");
                watches.push((target, next_name.as_str(), prev_name.as_str(), body));
            }
            Stmt::RefDecl { name, .. } => {
                refs.push(name.as_str());
            }
            _ => {}
        }
    }

    // Scan template for hydration instructions and reactive block effects
    let mut instructions = Vec::new();
    let mut block_effects = Vec::new();
    let mut text_expr_counter: u32 = 0;
    scan_template_instructions(
        &decl.body.template,
        &mut instructions,
        &mut block_effects,
        &mut imports,
        &signal_names,
        &mut text_expr_counter,
    );

    // Build hydrate instructions from directive markers
    if !instructions.is_empty() {
        imports.insert("hydrate");
    }

    // ── Emit the module ──────────────────────────────────────

    // Read env vars check — must happen before import list is built
    let needs_env = has_env_access(&decl.body.stmts, &decl.body.template);
    if needs_env {
        imports.insert("_readEnv");
    }

    // Import statement
    let mut import_list: Vec<&&str> = imports.iter().collect();
    import_list.sort();
    js.push_str(&format!(
        "import {{ {} }} from '/_gale/runtime.js';\n\n",
        import_list
            .iter()
            .map(|s| **s)
            .collect::<Vec<_>>()
            .join(", ")
    ));

    // Read server data
    js.push_str("const $data = _readData();\n");

    // Read env vars if any env access in the component
    if needs_env {
        js.push_str("const $env = _readEnv();\n");
    }
    js.push('\n');

    // Ref declarations
    for name in &refs {
        js.push_str(&format!("let {name} = null;\n"));
    }
    if !refs.is_empty() {
        js.push('\n');
    }

    // Signal declarations — initialize from server data where available,
    // register with HMR state preservation system.
    for (name, init) in &signals {
        let init_js = expr_to_js(init, &signal_names);
        js.push_str(&format!(
            "const {name} = _registerSignal(\"{name}\", signal($data.data?.{name} ?? {init_js}));\n"
        ));
    }
    if !signals.is_empty() {
        js.push('\n');
    }

    // Derive declarations
    for (name, init) in &derives {
        let init_js = expr_to_js(init, &signal_names);
        js.push_str(&format!("const {name} = derive(() => {init_js});\n"));
    }
    if !derives.is_empty() {
        js.push('\n');
    }

    // Effects
    for body in &effects {
        js.push_str("effect(() => {\n");
        for stmt in &body.stmts {
            let stmt_js = stmt_to_js(stmt, &signal_names);
            js.push_str(&format!("  {stmt_js}\n"));
        }
        js.push_str("});\n\n");
    }

    // Watches
    for (target, next_name, prev_name, body) in &watches {
        let target_js = expr_to_js(target, &signal_names);
        js.push_str(&format!(
            "watch(() => {target_js}, ({next_name}, {prev_name}) => {{\n"
        ));
        for stmt in &body.stmts {
            let stmt_js = stmt_to_js(stmt, &signal_names);
            js.push_str(&format!("  {stmt_js}\n"));
        }
        js.push_str("});\n\n");
    }

    // Hydration instructions (element-level: bind, on, ref, class, transition)
    if !instructions.is_empty() {
        js.push_str("hydrate({\n");
        for (id, instruction_js) in &instructions {
            js.push_str(&format!("  \"{id}\": {instruction_js},\n"));
        }
        js.push_str("});\n");
        if !block_effects.is_empty() {
            js.push('\n');
        }
    }

    // Block-level effects (when/each reactive template regions)
    for eff in &block_effects {
        js.push_str(eff);
        js.push('\n');
    }

    js
}

// ── Helpers ────────────────────────────────────────────────────────────

/// Collect reactive names (signals and derives) from component stmts.
///
/// Both signals and derives expose `.get()` / `.set()` in the runtime,
/// so both need to be tracked for expression conversion.
fn collect_signal_names(stmts: &[Stmt]) -> HashSet<String> {
    let mut names = HashSet::new();
    for stmt in stmts {
        match stmt {
            Stmt::Signal { name, .. } | Stmt::Derive { name, .. } => {
                names.insert(name.to_string());
            }
            _ => {}
        }
    }
    names
}

/// Check if template has any interactive directives or reactive control flow.
fn template_has_interactivity(nodes: &[TemplateNode]) -> bool {
    for node in nodes {
        match node {
            TemplateNode::Element {
                directives,
                children,
                ..
            } => {
                if directives.iter().any(is_interactive_directive) {
                    return true;
                }
                if template_has_interactivity(children) {
                    return true;
                }
            }
            TemplateNode::SelfClosing { directives, .. } => {
                if directives.iter().any(is_interactive_directive) {
                    return true;
                }
            }
            // When/Each blocks are always potentially reactive
            TemplateNode::When { .. } | TemplateNode::Each { .. } => {
                return true;
            }
            _ => {}
        }
    }
    false
}

fn is_interactive_directive(d: &Directive) -> bool {
    matches!(
        d,
        Directive::Bind { .. }
            | Directive::On { .. }
            | Directive::Ref { .. }
            | Directive::Transition { .. }
            | Directive::Class { .. }
    )
}

/// Scan template nodes and collect hydration instruction JS snippets.
///
/// Each instruction is `(hydration_id, js_code)` where `js_code` is an
/// arrow function like `el => bind(el, count)`.
///
/// Block effects (for `when`/`each` blocks) are collected separately
/// as top-level effect statements.
///
/// The hydration IDs here are assigned sequentially, matching the order
/// the SSR renderer assigns them (since both walk the template in the same order).
fn scan_template_instructions(
    nodes: &[TemplateNode],
    instructions: &mut Vec<(u32, String)>,
    block_effects: &mut Vec<String>,
    imports: &mut HashSet<&'static str>,
    signal_names: &HashSet<String>,
    next_id: &mut u32,
) {
    for node in nodes {
        match node {
            TemplateNode::Element {
                directives,
                children,
                ..
            } => {
                emit_directive_instructions(
                    directives,
                    instructions,
                    imports,
                    signal_names,
                    next_id,
                );
                scan_template_instructions(
                    children,
                    instructions,
                    block_effects,
                    imports,
                    signal_names,
                    next_id,
                );
            }
            TemplateNode::SelfClosing { directives, .. } => {
                emit_directive_instructions(
                    directives,
                    instructions,
                    imports,
                    signal_names,
                    next_id,
                );
            }
            TemplateNode::ExprInterp { expr, .. } => {
                let id = *next_id;
                *next_id += 1;
                let expr_js = expr_to_js(expr, signal_names);
                imports.insert("effect");
                instructions.push((
                    id,
                    format!("el => effect(() => {{ el.textContent = String({expr_js}); }})"),
                ));
            }
            TemplateNode::When {
                condition,
                body,
                else_branch,
                ..
            } => {
                // Consume the `when` marker ID (stays in sync with SSR <!--gx-when:N-->).
                let when_id = *next_id;
                *next_id += 1;

                let cond_js = expr_to_js(condition, signal_names);
                imports.insert("effect");
                imports.insert("replaceRegion");

                let true_html = template_to_js_html(body, signal_names);
                let false_html = match else_branch {
                    Some(WhenElse::Else(nodes)) => template_to_js_html(nodes, signal_names),
                    Some(WhenElse::ElseWhen(_)) => "\"\"".to_string(),
                    None => "\"\"".to_string(),
                };

                block_effects.push(format!(
                    "effect(() => replaceRegion({when_id}, () => ({cond_js}) ? {true_html} : {false_html}));"
                ));

                // Recurse into children for nested directives
                scan_template_instructions(
                    body,
                    instructions,
                    block_effects,
                    imports,
                    signal_names,
                    next_id,
                );
                if let Some(WhenElse::Else(nodes)) = else_branch {
                    scan_template_instructions(
                        nodes,
                        instructions,
                        block_effects,
                        imports,
                        signal_names,
                        next_id,
                    );
                }
            }
            TemplateNode::Each {
                binding,
                iterable,
                body,
                ..
            } => {
                // Consume the `each` marker ID (stays in sync with SSR <!--gx-each:N-->).
                let each_id = *next_id;
                *next_id += 1;

                let iter_js = expr_to_js(iterable, signal_names);
                imports.insert("effect");
                imports.insert("reconcileList");

                let item_html = template_to_js_html(body, signal_names);

                block_effects.push(format!(
                    "effect(() => reconcileList({each_id}, {iter_js}, ({binding}, _i) => _i, ({binding}, _i) => {item_html}));"
                ));

                scan_template_instructions(
                    body,
                    instructions,
                    block_effects,
                    imports,
                    signal_names,
                    next_id,
                );
            }
            TemplateNode::Text { .. }
            | TemplateNode::Slot { .. }
            | TemplateNode::Suspend { .. } => {}
        }
    }
}

/// Emit hydration instructions for directives on a single element.
fn emit_directive_instructions(
    directives: &[Directive],
    instructions: &mut Vec<(u32, String)>,
    imports: &mut HashSet<&'static str>,
    signal_names: &HashSet<String>,
    next_id: &mut u32,
) {
    for directive in directives {
        match directive {
            Directive::Bind { field, .. } => {
                let id = *next_id;
                *next_id += 1;
                imports.insert("bind");
                instructions.push((id, format!("el => bind(el, {field})")));
            }
            Directive::On {
                event,
                modifiers,
                handler,
                ..
            } => {
                let id = *next_id;
                *next_id += 1;
                let handler_js = expr_to_js(handler, signal_names);

                // Build event handler with modifier support
                let has_prevent = modifiers.iter().any(|m| m == "prevent");
                let has_stop = modifiers.iter().any(|m| m == "stop");
                let has_once = modifiers.iter().any(|m| m == "once");
                let has_self_mod = modifiers.iter().any(|m| m == "self");

                let mut body_parts = Vec::new();
                if has_prevent {
                    body_parts.push("e.preventDefault()".to_string());
                }
                if has_stop {
                    body_parts.push("e.stopPropagation()".to_string());
                }
                if has_self_mod {
                    body_parts.push("if (e.target !== el) return".to_string());
                }

                if body_parts.is_empty() && !has_once {
                    // Wrap the handler in an arrow function so it executes
                    // on each click, not once at hydration time.
                    instructions.push((
                        id,
                        format!(
                            "el => el.addEventListener(\"{event}\", () => {{ {handler_js}; }})"
                        ),
                    ));
                } else {
                    body_parts.push(format!("({handler_js})(e)"));
                    let body = body_parts.join("; ");
                    let opts = if has_once { ", { once: true }" } else { "" };
                    instructions.push((
                        id,
                        format!(
                            "el => el.addEventListener(\"{event}\", (e) => {{ {body}; }}{opts})"
                        ),
                    ));
                }
            }
            Directive::Ref { name, .. } => {
                let id = *next_id;
                *next_id += 1;
                instructions.push((id, format!("el => {{ {name} = el; }}")));
            }
            Directive::Transition { kind, .. } => {
                let id = *next_id;
                *next_id += 1;
                imports.insert("transition");
                instructions.push((id, format!("el => transition(el, \"{kind}\")")));
            }
            Directive::Class {
                name, condition, ..
            } => {
                let id = *next_id;
                *next_id += 1;
                imports.insert("effect");
                let cond_js = expr_to_js(condition, signal_names);
                instructions.push((
                    id,
                    format!("el => effect(() => el.classList.toggle(\"{name}\", {cond_js}))"),
                ));
            }
            _ => {} // form:*, key:, into:, prefetch — no client hydration needed
        }
    }
}

/// Convert template nodes to a JS template literal that returns HTML.
///
/// Used by `when`/`each` blocks to produce the render function bodies.
fn template_to_js_html(nodes: &[TemplateNode], signal_names: &HashSet<String>) -> String {
    let mut parts = Vec::new();

    for node in nodes {
        match node {
            TemplateNode::Text { value, .. } => {
                let escaped = value
                    .replace('\\', "\\\\")
                    .replace('`', "\\`")
                    .replace("${", "\\${");
                parts.push(escaped);
            }
            TemplateNode::ExprInterp { expr, .. } => {
                let expr_js = expr_to_js(expr, signal_names);
                parts.push(format!("${{{expr_js}}}"));
            }
            TemplateNode::Element {
                tag,
                attributes,
                children,
                ..
            } => {
                parts.push(format!("<{tag}"));
                for attr in attributes {
                    match &attr.value {
                        AttrValue::String(val) => {
                            parts.push(format!(" {}=\"{}\"", attr.name, val));
                        }
                        AttrValue::Expr(expr) => {
                            let v = expr_to_js(expr, signal_names);
                            parts.push(format!(" {}=\"${{{v}}}\"", attr.name));
                        }
                        AttrValue::Bool => {
                            parts.push(format!(" {}", attr.name));
                        }
                    }
                }
                parts.push(">".to_string());

                let inner = template_to_js_html(children, signal_names);
                // Unwrap inner backticks for nesting
                if inner.starts_with('`') && inner.ends_with('`') {
                    parts.push(inner[1..inner.len() - 1].to_string());
                } else {
                    parts.push(format!("${{{inner}}}"));
                }

                parts.push(format!("</{tag}>"));
            }
            TemplateNode::SelfClosing {
                tag, attributes, ..
            } => {
                parts.push(format!("<{tag}"));
                for attr in attributes {
                    match &attr.value {
                        AttrValue::String(val) => {
                            parts.push(format!(" {}=\"{}\"", attr.name, val));
                        }
                        AttrValue::Expr(expr) => {
                            let v = expr_to_js(expr, signal_names);
                            parts.push(format!(" {}=\"${{{v}}}\"", attr.name));
                        }
                        AttrValue::Bool => {
                            parts.push(format!(" {}", attr.name));
                        }
                    }
                }
                parts.push(" />".to_string());
            }
            TemplateNode::When {
                condition,
                body,
                else_branch,
                ..
            } => {
                let c = expr_to_js(condition, signal_names);
                let t = template_to_js_html(body, signal_names);
                let f = match else_branch {
                    Some(WhenElse::Else(nodes)) => template_to_js_html(nodes, signal_names),
                    _ => "\"\"".to_string(),
                };
                parts.push(format!("${{({c}) ? {t} : {f}}}"));
            }
            TemplateNode::Each {
                binding,
                iterable,
                body,
                ..
            } => {
                let iter_js = expr_to_js(iterable, signal_names);
                let item_html = template_to_js_html(body, signal_names);
                parts.push(format!(
                    "${{{iter_js}.map(({binding}) => {item_html}).join('')}}"
                ));
            }
            _ => {}
        }
    }

    format!("`{}`", parts.join(""))
}

/// Convert a statement to JS (brief, for effect/watch bodies).
fn stmt_to_js(stmt: &Stmt, signal_names: &HashSet<String>) -> String {
    match stmt {
        Stmt::Let { name, init, .. } => {
            format!("const {} = {};", name, expr_to_js(init, signal_names))
        }
        Stmt::Mut { name, init, .. } => {
            format!("let {} = {};", name, expr_to_js(init, signal_names))
        }
        Stmt::ExprStmt { expr, .. } => {
            format!("{};", expr_to_js(expr, signal_names))
        }
        Stmt::Return { value, .. } => match value {
            Some(e) => format!("return {};", expr_to_js(e, signal_names)),
            None => "return;".into(),
        },
        Stmt::If {
            condition,
            then_block,
            ..
        } => {
            let cond = expr_to_js(condition, signal_names);
            let body: Vec<String> = then_block
                .stmts
                .iter()
                .map(|s| format!("  {}", stmt_to_js(s, signal_names)))
                .collect();
            format!("if ({cond}) {{\n{}\n}}", body.join("\n"))
        }
        _ => format!("/* {} */", "unsupported stmt"),
    }
}

/// Check if the component body or template contains any `EnvAccess` expressions.
fn has_env_access(stmts: &[Stmt], template: &[TemplateNode]) -> bool {
    for stmt in stmts {
        if stmt_has_env(stmt) {
            return true;
        }
    }
    template_has_env(template)
}

fn stmt_has_env(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Let { init, .. } | Stmt::Mut { init, .. } | Stmt::Frozen { init, .. } => {
            expr_has_env(init)
        }
        Stmt::Signal { init, .. } => expr_has_env(init),
        Stmt::Derive { init, .. } => expr_has_env(init),
        _ => false,
    }
}

fn expr_has_env(expr: &Expr) -> bool {
    matches!(expr, Expr::EnvAccess { .. })
}

fn template_has_env(nodes: &[TemplateNode]) -> bool {
    for node in nodes {
        match node {
            TemplateNode::ExprInterp { expr, .. } => {
                if expr_has_env(expr) {
                    return true;
                }
            }
            TemplateNode::Element { children, .. } => {
                if template_has_env(children) {
                    return true;
                }
            }
            _ => {}
        }
    }
    false
}
