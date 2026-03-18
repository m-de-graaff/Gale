//! Rust server code generation from GaleX AST.
//!
//! This module walks a type-checked GaleX [`Program`](crate::ast::Program)
//! and emits a complete Rust/Cargo project that compiles into a Gale server.
//!
//! # Architecture
//!
//! The generated project depends on the `gale` crate (library) for its
//! production-grade middleware stack (security headers, compression, caching,
//! rate limiting, TLS, logging). The codegen adds:
//!
//! - **Actions** → Axum handler functions (`POST /action_name`)
//! - **Guards** → Rust structs with `validate()` methods
//! - **Channels** → WebSocket upgrade handlers
//! - **Queries** → HTTP client functions
//! - **Routes** → Page handlers derived from components
//! - **Shared types** → Enums, type aliases, shared functions
//!
//! # Usage
//!
//! ```ignore
//! use galex::codegen::generate;
//! use galex::checker::TypeChecker;
//! use std::path::Path;
//!
//! let mut checker = TypeChecker::new();
//! let errors = checker.check_program(&program);
//! assert!(errors.is_empty());
//!
//! generate(&program, &checker.interner, "my_app", Path::new("gale_build/")).unwrap();
//! ```

pub mod asset_hash;
pub mod bundle;
pub mod emit_action;
pub mod emit_api;
pub mod emit_channel;
pub mod emit_channel_js;
pub mod emit_client;
pub mod emit_client_actions;
pub mod emit_client_runtime;
pub mod emit_env;
pub mod emit_expr;
pub mod emit_form_js;
pub mod emit_guard;
pub mod emit_guard_js;
pub mod emit_middleware;
pub mod emit_query_js;
pub mod emit_router_js;
pub mod emit_shared;
pub mod emit_stmt;
pub mod emit_store_js;
pub mod emit_transitions_css;
pub mod expr;
pub mod head;
pub mod hydration;
pub mod js_emitter;
pub mod js_expr;
pub mod layout;
pub mod minify;
pub mod project;
pub mod route;
pub mod rust_emitter;
pub mod ssr;
pub mod types;

use std::collections::HashSet;
use std::path::Path;

use crate::ast::{Item, Program};
use crate::types::ty::TypeInterner;

use project::{MainModules, ProjectFiles};

// ── CodegenContext ──────────────────────────────────────────────────────

/// Top-level code generation context.
///
/// Walks the AST, dispatches items to per-file emitters, and accumulates
/// the output [`ProjectFiles`] for writing to disk.
pub struct CodegenContext<'a> {
    /// The type interner from the checker (for resolving TypeIds → TypeData).
    pub interner: &'a TypeInterner,
    /// Accumulated output files.
    pub files: ProjectFiles,
    /// Project name (used in Cargo.toml and the binary name).
    pub project_name: String,
    /// Path to the `gale` crate for the generated Cargo.toml dependency.
    pub gale_crate_path: String,
    /// Tracks which top-level module directories have content.
    modules: MainModules,
    /// Names of generated action modules (for the actions/mod.rs).
    action_modules: Vec<String>,
    /// Names of generated guard modules (for the guards/mod.rs).
    guard_modules: Vec<String>,
    /// Names of generated channel modules (for the channels/mod.rs).
    channel_modules: Vec<String>,
    /// Channel route registrations: `(channel_name, module_name)` for build_router().
    channel_routes: Vec<(String, String)>,
    /// Names of generated shared modules (for the shared/mod.rs).
    shared_modules: Vec<String>,
    /// Names of generated route modules (for the routes/mod.rs).
    route_modules: Vec<String>,
    /// Route path registrations for build_router() generation.
    route_paths: Vec<(String, String)>,
    /// Action route registrations: `(action_name, module_name)` for build_router().
    action_routes: Vec<(String, String)>,
    /// Names of generated API modules (for the api/mod.rs).
    api_modules: Vec<String>,
    /// API route groups: `(path, Vec<(method, "api::mod::fn_name")>)` for build_router().
    api_route_groups: Vec<(String, Vec<(String, String)>)>,
    /// Set of known guard names (for action param type resolution).
    known_guards: HashSet<String>,
    /// Guards that have transform validators (trim, precision, default).
    guards_with_transforms: HashSet<String>,
    /// Set of known shared type names (enums, type aliases — for import resolution).
    known_shared_types: HashSet<String>,
    /// Whether the generated project needs the `regex` crate.
    needs_regex: bool,
    /// Whether a layout was found (name stored for reference).
    has_layout: bool,
    /// Whether an env { } declaration was found.
    has_env_decl: bool,
    /// Keys declared in the env { } block (for typed accessor emission).
    pub declared_env_keys: HashSet<String>,
    /// Whether any PUBLIC_ env vars were declared.
    has_public_env_vars: bool,
    /// Names of generated middleware modules (for the middleware/mod.rs).
    middleware_modules: Vec<String>,
    /// Middleware registrations for build_router().
    middleware_registrations: Vec<project::MiddlewareRegistration>,
    /// JS guard metadata for form wiring and SSR script injection.
    js_guard_meta: Vec<emit_guard_js::GuardJsMeta>,
    /// Whether any guard declarations exist (triggers JS guard file emission).
    has_guards: bool,
    /// JS store metadata for component imports.
    js_store_meta: Vec<emit_store_js::StoreJsMeta>,
    /// Whether any store declarations exist (triggers store JS file emission).
    has_stores: bool,
    /// Action declarations collected during `emit_items()` for client-side
    /// JS stub generation. Cloned because `finalize()` has no AST access.
    client_action_decls: Vec<crate::ast::ActionDecl>,
    /// Whether any component has client-side interactive code (signals, effects, directives).
    has_client_code: bool,
    /// Names of pages that have client scripts (for listing generated page JS files).
    client_page_names: Vec<String>,
}

impl<'a> CodegenContext<'a> {
    /// Create a new code generation context.
    pub fn new(interner: &'a TypeInterner, project_name: &str) -> Self {
        Self {
            interner,
            files: ProjectFiles::new(),
            project_name: project_name.to_string(),
            gale_crate_path: "../".to_string(),
            modules: MainModules::default(),
            action_modules: Vec::new(),
            guard_modules: Vec::new(),
            channel_modules: Vec::new(),
            channel_routes: Vec::new(),
            shared_modules: Vec::new(),
            route_modules: Vec::new(),
            route_paths: Vec::new(),
            action_routes: Vec::new(),
            api_modules: Vec::new(),
            api_route_groups: Vec::new(),
            known_guards: HashSet::new(),
            guards_with_transforms: HashSet::new(),
            known_shared_types: HashSet::new(),
            needs_regex: false,
            has_layout: false,
            has_env_decl: false,
            declared_env_keys: HashSet::new(),
            has_public_env_vars: false,
            middleware_modules: Vec::new(),
            middleware_registrations: Vec::new(),
            js_guard_meta: Vec::new(),
            has_guards: false,
            js_store_meta: Vec::new(),
            has_stores: false,
            client_action_decls: Vec::new(),
            has_client_code: false,
            client_page_names: Vec::new(),
        }
    }

    /// Look up JS guard metadata by guard name (for SSR wiring).
    pub fn find_js_guard_meta(&self, guard_name: &str) -> Option<&emit_guard_js::GuardJsMeta> {
        self.js_guard_meta
            .iter()
            .find(|m| m.guard_name == guard_name)
    }

    /// Walk the AST and generate all output files.
    ///
    /// This is the main entry point. It:
    /// 1. Scans the program to discover what needs to be generated.
    /// 2. Generates the project scaffold (Cargo.toml, main.rs).
    /// 3. Dispatches each item to its per-file emitter.
    /// 4. Generates `mod.rs` files for populated directories.
    pub fn emit_program(&mut self, program: &Program) {
        // Phase 1: scan items to determine which modules are needed
        self.scan_items(&program.items);

        // Phase 1b: set declared env keys for expression emitters
        if self.has_env_decl {
            expr::set_declared_env_keys(&self.declared_env_keys);
        }

        // Phase 2: emit individual items
        self.emit_items(&program.items);

        // Phase 3: generate scaffold and mod.rs files
        self.finalize();
    }

    /// Scan items to determine which output modules are needed.
    fn scan_items(&mut self, items: &[Item]) {
        for item in items {
            match item {
                Item::ActionDecl(_) => self.modules.has_actions = true,
                Item::GuardDecl(decl) => {
                    self.modules.has_guards = true;
                    self.known_guards.insert(decl.name.to_string());
                    if decl.fields.iter().any(|f| {
                        f.validators
                            .iter()
                            .any(|v| matches!(v.name.as_str(), "trim" | "precision" | "default"))
                    }) {
                        self.guards_with_transforms.insert(decl.name.to_string());
                    }
                    if decl.fields.iter().any(|f| {
                        f.validators
                            .iter()
                            .any(|v| matches!(v.name.as_str(), "email" | "url" | "uuid" | "regex"))
                    }) {
                        self.needs_regex = true;
                    }
                }
                Item::ChannelDecl(_) => self.modules.has_channels = true,
                Item::EnumDecl(decl) => {
                    self.modules.has_shared = true;
                    self.known_shared_types.insert(decl.name.to_string());
                }
                Item::TypeAlias(decl) => {
                    self.modules.has_shared = true;
                    self.known_shared_types.insert(decl.name.to_string());
                }
                Item::ComponentDecl(_) => self.modules.has_routes = true,
                Item::LayoutDecl(_) => self.has_layout = true,
                Item::ApiDecl(_) => self.modules.has_api = true,
                Item::MiddlewareDecl(_) => self.modules.has_middleware = true,
                Item::EnvDecl(decl) => {
                    self.modules.has_env = true;
                    self.has_env_decl = true;
                    for var in &decl.vars {
                        self.declared_env_keys.insert(var.key.to_string());
                        if var.key.starts_with("PUBLIC_") {
                            self.has_public_env_vars = true;
                        }
                    }
                }
                Item::ServerBlock(block) => self.scan_items(&block.items),
                Item::ClientBlock(_) => { /* client-only — skip for server codegen */ }
                Item::SharedBlock(block) => {
                    // Shared items may include guards, enums, type aliases
                    for inner in &block.items {
                        match inner {
                            Item::GuardDecl(decl) => {
                                self.modules.has_guards = true;
                                self.modules.has_shared = true;
                                self.known_guards.insert(decl.name.to_string());
                                if decl.fields.iter().any(|f| {
                                    f.validators.iter().any(|v| {
                                        matches!(v.name.as_str(), "trim" | "precision" | "default")
                                    })
                                }) {
                                    self.guards_with_transforms.insert(decl.name.to_string());
                                }
                                if decl.fields.iter().any(|f| {
                                    f.validators.iter().any(|v| {
                                        matches!(
                                            v.name.as_str(),
                                            "email" | "url" | "uuid" | "regex"
                                        )
                                    })
                                }) {
                                    self.needs_regex = true;
                                }
                            }
                            Item::EnumDecl(decl) => {
                                self.modules.has_shared = true;
                                self.known_shared_types.insert(decl.name.to_string());
                            }
                            Item::TypeAlias(decl) => {
                                self.modules.has_shared = true;
                                self.known_shared_types.insert(decl.name.to_string());
                            }
                            Item::FnDecl(_) => self.modules.has_shared = true,
                            Item::Out(out) => self.scan_items(&[*out.inner.clone()]),
                            _ => {}
                        }
                    }
                }
                Item::Out(out) => self.scan_items(&[*out.inner.clone()]),
                _ => {}
            }
        }
    }

    /// Emit individual items (stub for Phase 11.1).
    ///
    /// Real implementations (actions, guards, channels, etc.) will be
    /// added in Phase 11.2+. For now this is a no-op that records
    /// module names for the directory structure.
    fn emit_items(&mut self, items: &[Item]) {
        for item in items {
            match item {
                Item::ActionDecl(decl) => {
                    let mod_name = types::to_module_name(&decl.name);
                    self.action_modules.push(mod_name.clone());
                    self.action_routes
                        .push((decl.name.to_string(), mod_name.clone()));
                    // Generate real action handler
                    let mut emitter = rust_emitter::RustEmitter::new();
                    emit_action::emit_action_file(
                        &mut emitter,
                        decl,
                        &self.known_guards,
                        &self.guards_with_transforms,
                        &self.known_shared_types,
                    );
                    self.files
                        .add_file(format!("src/actions/{mod_name}.rs"), emitter.finish());
                    // Store for client-side JS stub generation
                    self.client_action_decls.push(decl.clone());
                }
                Item::GuardDecl(decl) => {
                    let mod_name = types::to_module_name(&decl.name);
                    self.guard_modules.push(mod_name.clone());
                    self.has_guards = true;
                    // Generate Rust guard struct + validate()
                    let mut emitter = rust_emitter::RustEmitter::new();
                    emit_guard::emit_guard_file(&mut emitter, decl, &self.known_shared_types);
                    self.files
                        .add_file(format!("src/guards/{mod_name}.rs"), emitter.finish());
                    // Generate JS guard validator (client-side mirror)
                    let mut js_emitter = js_emitter::JsEmitter::new();
                    let meta = emit_guard_js::emit_guard_js_file(&mut js_emitter, decl);
                    self.files.add_file(
                        format!("static/js/guards/{}.js", meta.module_name),
                        js_emitter.finish(),
                    );
                    self.js_guard_meta.push(meta);
                }
                Item::ChannelDecl(decl) => {
                    let mod_name = types::to_module_name(&decl.name);
                    self.channel_modules.push(mod_name.clone());
                    self.channel_routes
                        .push((decl.name.to_string(), mod_name.clone()));
                    // Generate Rust channel handler (server-side WebSocket)
                    let mut emitter = rust_emitter::RustEmitter::new();
                    emit_channel::emit_channel_file(&mut emitter, decl, &self.known_shared_types);
                    self.files
                        .add_file(format!("src/channels/{mod_name}.rs"), emitter.finish());
                    // Generate JS channel wrapper (client-side)
                    self.has_client_code = true;
                    let mut js_emitter = js_emitter::JsEmitter::new();
                    let _meta = emit_channel_js::emit_channel_js_file(&mut js_emitter, decl);
                    self.files.add_file(
                        format!("public/_gale/channels/{}.js", mod_name),
                        js_emitter.finish(),
                    );
                }
                Item::EnumDecl(decl) => {
                    let mod_name = types::to_module_name(&decl.name);
                    self.shared_modules.push(mod_name.clone());
                    // Generate real shared enum
                    let mut emitter = rust_emitter::RustEmitter::new();
                    emit_shared::emit_enum_file(&mut emitter, decl);
                    self.files
                        .add_file(format!("src/shared/{mod_name}.rs"), emitter.finish());
                }
                Item::TypeAlias(decl) => {
                    let mod_name = types::to_module_name(&decl.name);
                    self.shared_modules.push(mod_name.clone());
                    // Generate real shared type alias / struct
                    let mut emitter = rust_emitter::RustEmitter::new();
                    emit_shared::emit_type_alias_file(&mut emitter, decl);
                    self.files
                        .add_file(format!("src/shared/{mod_name}.rs"), emitter.finish());
                }
                Item::FnDecl(decl) => {
                    // Shared functions (only reached inside SharedBlock recursion)
                    let mod_name = types::to_module_name(&decl.name);
                    if !self.shared_modules.contains(&mod_name) {
                        self.shared_modules.push(mod_name.clone());
                    }
                    let mut emitter = rust_emitter::RustEmitter::new();
                    emit_shared::emit_shared_fn_file(&mut emitter, decl);
                    self.files
                        .add_file(format!("src/shared/{mod_name}.rs"), emitter.finish());
                }
                Item::ComponentDecl(decl) => {
                    // Strip path params from name for module name
                    let base_name = if let Some(bracket) = decl.name.find('[') {
                        &decl.name[..bracket]
                    } else {
                        &decl.name
                    };
                    let mod_name = types::to_module_name(base_name);
                    self.route_modules.push(mod_name.clone());

                    // Record route path for build_router
                    let route_path = route::component_name_to_path(&decl.name);
                    self.route_paths.push((route_path, mod_name.clone()));

                    // Generate real route handler module
                    self.files.add_file(
                        format!("src/routes/{mod_name}.rs"),
                        route::emit_route_module(decl),
                    );

                    // Generate per-page client JS if the component has interactive elements
                    if emit_client::component_has_client_code(decl) {
                        self.has_client_code = true;
                        self.client_page_names.push(mod_name.clone());
                        let page_js = emit_client::emit_page_script(decl);
                        self.files
                            .add_file(format!("public/_gale/pages/{mod_name}.js"), page_js);
                    }
                }
                Item::LayoutDecl(decl) => {
                    self.has_layout = true;
                    self.files
                        .add_file("src/layout.rs", layout::emit_layout_module(decl));
                }
                Item::ApiDecl(decl) => {
                    let mod_name = types::to_module_name(&decl.name);
                    self.api_modules.push(mod_name.clone());

                    // Generate API handler file
                    let mut emitter = rust_emitter::RustEmitter::new();
                    emit_api::emit_api_file(
                        &mut emitter,
                        decl,
                        &self.known_guards,
                        &self.known_shared_types,
                    );
                    self.files
                        .add_file(format!("src/api/{mod_name}.rs"), emitter.finish());

                    // Record route groups for build_router
                    let groups = emit_api::api_route_groups(&decl.name, &decl.handlers);
                    for (path, methods) in groups {
                        let qualified: Vec<(String, String)> = methods
                            .into_iter()
                            .map(|(m, fn_name)| (m, format!("api::{mod_name}::{fn_name}")))
                            .collect();
                        self.api_route_groups.push((path, qualified));
                    }
                }
                Item::MiddlewareDecl(decl) => {
                    let mod_name = types::to_module_name(&decl.name);
                    self.middleware_modules.push(mod_name.clone());

                    // Generate middleware handler file
                    let mut emitter = rust_emitter::RustEmitter::new();
                    emit_middleware::emit_middleware_file(&mut emitter, decl);
                    self.files
                        .add_file(format!("src/middleware/{mod_name}.rs"), emitter.finish());

                    // Record registration for build_router
                    let (target_kind, target_value) = match &decl.target {
                        crate::ast::MiddlewareTarget::Global => {
                            ("global".to_string(), String::new())
                        }
                        crate::ast::MiddlewareTarget::PathPrefix(prefix) => {
                            ("prefix".to_string(), prefix.to_string())
                        }
                        crate::ast::MiddlewareTarget::Resource(name) => {
                            ("resource".to_string(), name.to_string())
                        }
                    };
                    self.middleware_registrations
                        .push(project::MiddlewareRegistration {
                            handler_path: format!("middleware::{mod_name}::middleware_fn"),
                            target_kind,
                            target_value,
                        });
                }
                Item::EnvDecl(decl) => {
                    // Generate env config module
                    let mut emitter = rust_emitter::RustEmitter::new();
                    emit_env::emit_env_file(&mut emitter, decl);
                    self.files.add_file("src/env_config.rs", emitter.finish());
                }
                Item::QueryDecl(decl) => {
                    // Generate JS query wrapper (client-side only)
                    self.has_client_code = true;
                    let mod_name = types::to_module_name(&decl.name);
                    let mut js_emitter = js_emitter::JsEmitter::new();
                    let _meta = emit_query_js::emit_query_js_file(&mut js_emitter, decl);
                    self.files.add_file(
                        format!("public/_gale/queries/{mod_name}.js"),
                        js_emitter.finish(),
                    );
                }
                Item::StoreDecl(decl) => {
                    self.has_stores = true;
                    self.has_client_code = true;
                    // Generate JS store singleton module (client-side only)
                    let mut js_emitter = js_emitter::JsEmitter::new();
                    let meta = emit_store_js::emit_store_js_file(&mut js_emitter, decl);
                    self.files.add_file(
                        format!("public/_gale/stores/{}.js", meta.module_name),
                        js_emitter.finish(),
                    );
                    self.js_store_meta.push(meta);
                }
                Item::ServerBlock(block) => self.emit_items(&block.items),
                Item::SharedBlock(block) => self.emit_items(&block.items),
                Item::Out(out) => self.emit_items(&[*out.inner.clone()]),
                // Client blocks, statements, etc. — skip for server codegen
                _ => {}
            }
        }
    }

    /// Generate scaffold files and module declarations.
    fn finalize(&mut self) {
        // Cargo.toml
        self.files.add_file(
            "Cargo.toml",
            project::generate_cargo_toml(
                &self.project_name,
                self.needs_regex,
                &self.gale_crate_path,
            ),
        );

        // src/main.rs — now with actual route registrations
        self.files.add_file(
            "src/main.rs",
            project::generate_main_rs(
                &self.modules,
                self.has_layout,
                &self.route_paths,
                &self.action_routes,
                &self.api_route_groups,
                &self.channel_routes,
                &self.middleware_registrations,
            ),
        );

        // Layout — generate default if none declared
        if !self.has_layout && self.modules.has_routes {
            self.files
                .add_file("src/layout.rs", layout::emit_default_layout());
        }

        // SSR runtime helper
        if self.modules.has_routes {
            self.files
                .add_file("src/gale_ssr.rs", project::generate_gale_ssr_runtime());
        }

        // Middleware runtime helper
        if self.modules.has_middleware {
            self.files.add_file(
                "src/gale_middleware.rs",
                project::generate_gale_middleware_runtime(),
            );
        }

        // Client runtime JS — embedded from the hand-written runtime source
        if self.has_client_code {
            self.files.add_file(
                "public/_gale/runtime.js",
                include_str!("../runtime/gale_runtime.js").to_string(),
            );
        }

        // Default asset manifest (identity map — no hashing).
        // Production builds replace this with hashed entries via
        // `asset_hash::hash_project_assets()` before writing to disk.
        self.files.add_file(
            "src/asset_manifest.rs",
            asset_hash::AssetManifest::new().generate_rust_module(),
        );

        // Generate mod.rs for each populated directory
        if self.modules.has_actions && !self.action_modules.is_empty() {
            let children: Vec<&str> = self.action_modules.iter().map(|s| s.as_str()).collect();
            self.files
                .add_file("src/actions/mod.rs", project::generate_mod_rs(&children));
        }
        if self.modules.has_guards && !self.guard_modules.is_empty() {
            let children: Vec<&str> = self.guard_modules.iter().map(|s| s.as_str()).collect();
            self.files
                .add_file("src/guards/mod.rs", project::generate_mod_rs(&children));

            // Generate the shared validation module (needed by guard validate() methods)
            self.modules.has_shared = true;
            if !self.shared_modules.contains(&"validation".to_string()) {
                self.shared_modules.push("validation".to_string());
            }
            self.files.add_file(
                "src/shared/validation.rs",
                project::generate_validation_rs(),
            );
        }
        if self.modules.has_middleware && !self.middleware_modules.is_empty() {
            let children: Vec<&str> = self.middleware_modules.iter().map(|s| s.as_str()).collect();
            self.files
                .add_file("src/middleware/mod.rs", project::generate_mod_rs(&children));
        }
        if self.modules.has_api && !self.api_modules.is_empty() {
            let children: Vec<&str> = self.api_modules.iter().map(|s| s.as_str()).collect();
            self.files
                .add_file("src/api/mod.rs", project::generate_mod_rs(&children));
        }
        if self.modules.has_channels && !self.channel_modules.is_empty() {
            let children: Vec<&str> = self.channel_modules.iter().map(|s| s.as_str()).collect();
            self.files
                .add_file("src/channels/mod.rs", project::generate_mod_rs(&children));
        }
        if self.modules.has_shared && !self.shared_modules.is_empty() {
            let children: Vec<&str> = self.shared_modules.iter().map(|s| s.as_str()).collect();
            self.files
                .add_file("src/shared/mod.rs", project::generate_mod_rs(&children));
        }
        if self.modules.has_routes && !self.route_modules.is_empty() {
            let children: Vec<&str> = self.route_modules.iter().map(|s| s.as_str()).collect();
            self.files
                .add_file("src/routes/mod.rs", project::generate_mod_rs(&children));
        }

        // ── JS guard validation files ──────────────────────────────
        // Emit the gale-forms.js runtime if any guards exist
        if self.has_guards {
            self.files.add_file(
                "static/js/gale-forms.js",
                emit_form_js::emit_gale_forms_runtime(),
            );
        }

        // ── JS action stubs (client-side RPC bridge) ──────────────
        if !self.client_action_decls.is_empty() {
            // Error classes, fetch wrapper, and query cache are now part of
            // the consolidated runtime (gale_runtime.js), so we only emit
            // the action stubs file. It imports from /_gale/runtime.js.
            self.files.add_file(
                "public/_gale/actions.js",
                emit_client_actions::generate_client_actions_js(
                    &self.client_action_decls,
                    &self.known_guards,
                    &self.js_guard_meta,
                ),
            );

            // Ensure the consolidated runtime is emitted (may already be
            // triggered by has_client_code, but actions alone also need it)
            if !self.has_client_code {
                self.files.add_file(
                    "public/_gale/runtime.js",
                    include_str!("../runtime/gale_runtime.js").to_string(),
                );
            }
        }

        // ── Built-in transition CSS ────────────────────────────────
        // Always emit transitions CSS if there are interactive components
        // (the CSS is small and cacheable; selective emission can be added later)
        if self.has_client_code {
            self.files.add_file(
                "public/_gale/transitions.css",
                emit_transitions_css::emit_transitions_css(),
            );
        }

        // ── Client-side router ─────────────────────────────────────
        // Emit the router if there are multiple routes (SPA navigation)
        if self.modules.has_routes && self.route_paths.len() > 1 {
            let route_entries: Vec<emit_router_js::RouteEntry> = self
                .route_paths
                .iter()
                .map(|(path, page)| {
                    let params = route::extract_route_params(path);
                    emit_router_js::RouteEntry {
                        path: path.clone(),
                        page: page.clone(),
                        params,
                    }
                })
                .collect();
            self.files.add_file(
                "public/_gale/router.js",
                emit_router_js::emit_router_js(&route_entries),
            );
        }
    }

    /// Write all generated files to disk.
    pub fn write(&self, output_dir: &Path) -> std::io::Result<()> {
        self.files.write_to_disk(output_dir)
    }
}

// ── Public API ─────────────────────────────────────────────────────────

/// Run the full GaleX → Rust code generation pipeline.
///
/// 1. Walks the type-checked AST.
/// 2. Generates a complete Rust/Cargo project in `output_dir`.
/// 3. Writes all files to disk.
///
/// # Errors
///
/// Returns an I/O error if file writing fails.
pub fn generate(
    program: &Program,
    interner: &TypeInterner,
    project_name: &str,
    output_dir: &Path,
    gale_crate_path: Option<&str>,
) -> std::io::Result<()> {
    let mut ctx = CodegenContext::new(interner, project_name);
    if let Some(path) = gale_crate_path {
        ctx.gale_crate_path = path.to_string();
    }
    ctx.emit_program(program);
    ctx.write(output_dir)
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::*;
    use crate::span::Span;

    fn s() -> Span {
        Span::dummy()
    }

    #[test]
    fn empty_program_produces_scaffold() {
        let interner = TypeInterner::new();
        let program = Program {
            items: vec![],
            span: s(),
        };
        let mut ctx = CodegenContext::new(&interner, "test_app");
        ctx.emit_program(&program);

        assert!(ctx.files.contains("Cargo.toml"));
        assert!(ctx.files.contains("src/main.rs"));
        // No action/guard/channel/route directories for empty program
        assert!(!ctx.files.contains("src/actions/mod.rs"));
        assert!(!ctx.files.contains("src/guards/mod.rs"));
    }

    #[test]
    fn program_with_action_creates_actions_dir() {
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

        assert!(ctx.files.contains("src/actions/mod.rs"));
        assert!(ctx.files.contains("src/actions/create_user.rs"));
        let modrs = ctx.files.get("src/actions/mod.rs").unwrap();
        assert!(modrs.contains("pub mod create_user;"));
    }

    #[test]
    fn program_with_guard_creates_guards_dir() {
        let interner = TypeInterner::new();
        let program = Program {
            items: vec![Item::GuardDecl(GuardDecl {
                name: "LoginForm".into(),
                fields: vec![],
                span: s(),
            })],
            span: s(),
        };
        let mut ctx = CodegenContext::new(&interner, "test_app");
        ctx.emit_program(&program);

        assert!(ctx.files.contains("src/guards/mod.rs"));
        assert!(ctx.files.contains("src/guards/login_form.rs"));
    }

    #[test]
    fn program_with_channel_creates_channels_dir() {
        let interner = TypeInterner::new();
        let program = Program {
            items: vec![Item::ChannelDecl(ChannelDecl {
                name: "Chat".into(),
                params: vec![],
                direction: ChannelDirection::Bidirectional,
                msg_ty: TypeAnnotation::Named {
                    name: "string".into(),
                    span: s(),
                },
                handlers: vec![],
                span: s(),
            })],
            span: s(),
        };
        let mut ctx = CodegenContext::new(&interner, "test_app");
        ctx.emit_program(&program);

        assert!(ctx.files.contains("src/channels/mod.rs"));
        assert!(ctx.files.contains("src/channels/chat.rs"));
        // Verify real handler content (not placeholder)
        let chat = ctx.files.get("src/channels/chat.rs").unwrap();
        assert!(chat.contains("pub async fn handler("));
        assert!(chat.contains("WebSocketUpgrade"));
        assert!(chat.contains("handle_socket"));
        // Verify route registration in main.rs
        let main = ctx.files.get("src/main.rs").unwrap();
        assert!(main.contains("mod channels;"));
        assert!(main.contains("/ws/__gx/channels/Chat"));
        assert!(main.contains("channels::chat::handler"));
    }

    #[test]
    fn program_with_enum_creates_shared_dir() {
        let interner = TypeInterner::new();
        let program = Program {
            items: vec![Item::EnumDecl(EnumDecl {
                name: "Status".into(),
                variants: vec!["Active".into()],
                span: s(),
            })],
            span: s(),
        };
        let mut ctx = CodegenContext::new(&interner, "test_app");
        ctx.emit_program(&program);

        assert!(ctx.files.contains("src/shared/mod.rs"));
        assert!(ctx.files.contains("src/shared/status.rs"));
        // Verify real enum content (not placeholder)
        let status = ctx.files.get("src/shared/status.rs").unwrap();
        assert!(status.contains("pub enum Status"));
        assert!(status.contains("Active,"));
        assert!(status.contains("Serialize, Deserialize"));
    }

    #[test]
    fn shared_block_fn_creates_module() {
        let interner = TypeInterner::new();
        let program = Program {
            items: vec![Item::SharedBlock(BoundaryBlock {
                items: vec![Item::FnDecl(FnDecl {
                    name: "validate".into(),
                    params: vec![Param {
                        name: "x".into(),
                        ty_ann: Some(TypeAnnotation::Named {
                            name: "int".into(),
                            span: s(),
                        }),
                        default: None,
                        span: s(),
                    }],
                    ret_ty: Some(TypeAnnotation::Named {
                        name: "bool".into(),
                        span: s(),
                    }),
                    body: Block {
                        stmts: vec![],
                        span: s(),
                    },
                    is_async: false,
                    span: s(),
                })],
                span: s(),
            })],
            span: s(),
        };
        let mut ctx = CodegenContext::new(&interner, "test_app");
        ctx.emit_program(&program);

        assert!(ctx.files.contains("src/shared/mod.rs"));
        assert!(ctx.files.contains("src/shared/validate.rs"));
        let validate = ctx.files.get("src/shared/validate.rs").unwrap();
        assert!(validate.contains("pub fn validate(x: i64) -> bool"));
    }

    #[test]
    fn shared_block_type_alias_creates_module() {
        let interner = TypeInterner::new();
        let program = Program {
            items: vec![Item::SharedBlock(BoundaryBlock {
                items: vec![Item::TypeAlias(TypeAliasDecl {
                    name: "UserId".into(),
                    ty: TypeAnnotation::Named {
                        name: "int".into(),
                        span: s(),
                    },
                    span: s(),
                })],
                span: s(),
            })],
            span: s(),
        };
        let mut ctx = CodegenContext::new(&interner, "test_app");
        ctx.emit_program(&program);

        assert!(ctx.files.contains("src/shared/user_id.rs"));
        let uid = ctx.files.get("src/shared/user_id.rs").unwrap();
        assert!(uid.contains("pub type UserId = i64;"));
    }

    #[test]
    fn program_with_component_creates_routes_dir() {
        let interner = TypeInterner::new();
        let program = Program {
            items: vec![Item::ComponentDecl(ComponentDecl {
                name: "HomePage".into(),
                props: vec![],
                body: ComponentBody {
                    stmts: vec![],
                    template: vec![],
                    head: None,
                    span: s(),
                },
                span: s(),
            })],
            span: s(),
        };
        let mut ctx = CodegenContext::new(&interner, "test_app");
        ctx.emit_program(&program);

        assert!(ctx.files.contains("src/routes/mod.rs"));
        assert!(ctx.files.contains("src/routes/home_page.rs"));
    }

    #[test]
    fn server_block_items_are_scanned() {
        let interner = TypeInterner::new();
        let program = Program {
            items: vec![Item::ServerBlock(BoundaryBlock {
                items: vec![Item::ActionDecl(ActionDecl {
                    name: "deleteUser".into(),
                    params: vec![],
                    ret_ty: None,
                    body: Block {
                        stmts: vec![],
                        span: s(),
                    },
                    span: s(),
                })],
                span: s(),
            })],
            span: s(),
        };
        let mut ctx = CodegenContext::new(&interner, "test_app");
        ctx.emit_program(&program);

        assert!(ctx.files.contains("src/actions/delete_user.rs"));
    }

    #[test]
    fn shared_block_guards_detected() {
        let interner = TypeInterner::new();
        let program = Program {
            items: vec![Item::SharedBlock(BoundaryBlock {
                items: vec![Item::GuardDecl(GuardDecl {
                    name: "Email".into(),
                    fields: vec![],
                    span: s(),
                })],
                span: s(),
            })],
            span: s(),
        };
        let mut ctx = CodegenContext::new(&interner, "test_app");
        ctx.emit_program(&program);

        assert!(ctx.files.contains("src/guards/email.rs"));
    }

    #[test]
    fn out_decl_unwrapped() {
        let interner = TypeInterner::new();
        let program = Program {
            items: vec![Item::Out(OutDecl {
                inner: Box::new(Item::ActionDecl(ActionDecl {
                    name: "save".into(),
                    params: vec![],
                    ret_ty: None,
                    body: Block {
                        stmts: vec![],
                        span: s(),
                    },
                    span: s(),
                })),
                span: s(),
            })],
            span: s(),
        };
        let mut ctx = CodegenContext::new(&interner, "test_app");
        ctx.emit_program(&program);

        assert!(ctx.files.contains("src/actions/save.rs"));
    }

    #[test]
    fn main_rs_declares_populated_modules_only() {
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
                Item::GuardDecl(GuardDecl {
                    name: "MyGuard".into(),
                    fields: vec![],
                    span: s(),
                }),
            ],
            span: s(),
        };
        let mut ctx = CodegenContext::new(&interner, "test_app");
        ctx.emit_program(&program);

        let main = ctx.files.get("src/main.rs").unwrap();
        assert!(main.contains("mod actions;"), "should have actions");
        assert!(main.contains("mod guards;"), "should have guards");
        assert!(!main.contains("mod channels;"), "should not have channels");
        assert!(!main.contains("mod routes;"), "should not have routes");
    }

    #[test]
    fn multiple_actions_in_single_mod_rs() {
        let interner = TypeInterner::new();
        let program = Program {
            items: vec![
                Item::ActionDecl(ActionDecl {
                    name: "createUser".into(),
                    params: vec![],
                    ret_ty: None,
                    body: Block {
                        stmts: vec![],
                        span: s(),
                    },
                    span: s(),
                }),
                Item::ActionDecl(ActionDecl {
                    name: "deleteUser".into(),
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

        let modrs = ctx.files.get("src/actions/mod.rs").unwrap();
        assert!(modrs.contains("pub mod create_user;"));
        assert!(modrs.contains("pub mod delete_user;"));
    }
}
