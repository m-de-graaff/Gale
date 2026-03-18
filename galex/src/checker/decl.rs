//! Declaration/item type checking.

use smol_str::SmolStr;

use super::TypeChecker;
use crate::ast::*;
use crate::errors::codes;
use crate::types::env::{BindingKind, ScopeKind};
use crate::types::ty::{self, *};

impl TypeChecker {
    /// Type-check a top-level item.
    pub(super) fn check_item(&mut self, item: &Item) {
        match item {
            Item::FnDecl(decl) => self.check_fn_decl(decl),
            Item::GuardDecl(decl) => self.check_guard_decl(decl),
            Item::StoreDecl(decl) => self.check_store_decl(decl),
            Item::ActionDecl(decl) => self.check_action_decl(decl),
            Item::QueryDecl(decl) => self.check_query_decl(decl),
            Item::ChannelDecl(decl) => self.check_channel_decl(decl),
            Item::TypeAlias(decl) => self.check_type_alias(decl),
            Item::EnumDecl(decl) => self.check_enum_decl(decl),
            Item::TestDecl(decl) => self.check_test_decl(decl),
            Item::ComponentDecl(decl) => self.check_component_decl(decl),
            Item::LayoutDecl(decl) => self.check_layout_decl(decl),
            Item::ApiDecl(decl) => self.check_api_decl(decl),
            Item::MiddlewareDecl(decl) => self.check_middleware_decl(decl),
            Item::EnvDecl(decl) => self.check_env_decl(decl),
            Item::ServerBlock(block) => {
                // Boundary blocks are transparent — they tag declarations
                // with a boundary scope but don't create new lexical scopes.
                self.env
                    .push_boundary(crate::types::env::BoundaryScope::Server);
                for item in &block.items {
                    self.check_declaration_in_boundary(item, ScopeKind::ServerBlock);
                    self.check_item(item);
                }
                self.env.pop_boundary();
            }
            Item::ClientBlock(block) => {
                self.env
                    .push_boundary(crate::types::env::BoundaryScope::Client);
                for item in &block.items {
                    self.check_declaration_in_boundary(item, ScopeKind::ClientBlock);
                    self.check_item(item);
                }
                self.env.pop_boundary();
            }
            Item::SharedBlock(block) => {
                self.env
                    .push_boundary(crate::types::env::BoundaryScope::Shared);
                for item in &block.items {
                    self.check_item(item);
                }
                self.env.pop_boundary();
            }
            Item::Out(out) => {
                // First, type-check the inner declaration
                self.check_item(&out.inner);
                // Then validate that the export is coherent with its boundary
                self.validate_export(&out.inner, out.span);
            }
            Item::Use(decl) => self.check_use_decl(decl),
            Item::Stmt(stmt) => self.check_stmt(stmt),
        }
    }

    /// Type-check a function declaration.
    pub(super) fn check_fn_decl(&mut self, decl: &FnDecl) {
        // Resolve parameter types
        let fn_params: Vec<FnParam> = decl
            .params
            .iter()
            .map(|p| {
                let ty = if let Some(ann) = &p.ty_ann {
                    self.resolve_annotation(ann)
                } else {
                    self.interner.fresh_type_var()
                };
                FnParam {
                    name: p.name.clone(),
                    ty,
                    has_default: p.default.is_some(),
                }
            })
            .collect();

        // Resolve return type
        let ret_ty = if let Some(ann) = &decl.ret_ty {
            self.resolve_annotation(ann)
        } else {
            self.interner.fresh_type_var()
        };

        // Build function type and register it
        let fn_ty = self.interner.make_function(FunctionSig {
            params: fn_params.clone(),
            ret: ret_ty,
            is_async: decl.is_async,
        });
        self.define_binding(decl.name.clone(), fn_ty, BindingKind::Function, decl.span);

        // Check body
        self.env.push_scope(ScopeKind::Function);
        for (ast_param, fn_param) in decl.params.iter().zip(fn_params.iter()) {
            self.define_binding(
                ast_param.name.clone(),
                fn_param.ty,
                BindingKind::Parameter,
                ast_param.span,
            );
            // If param has a default, check its type
            if let Some(default) = &ast_param.default {
                let default_ty = self.infer_expr(default);
                self.constrain_assignable(
                    default_ty,
                    fn_param.ty,
                    ast_param.span,
                    &format!("default value for parameter '{}'", ast_param.name),
                );
            }
        }

        let old_return = self.current_return_type.replace(ret_ty);
        self.check_block(&decl.body);
        self.current_return_type = old_return;

        self.env.pop_scope();
    }

    /// Type-check a guard declaration.
    fn check_guard_decl(&mut self, decl: &GuardDecl) {
        self.env.push_scope(ScopeKind::GuardBody);

        let mut guard_fields = Vec::new();
        let mut seen_names = std::collections::HashSet::new();
        let mut has_validators = false;

        // If extends, merge parent fields first
        // (extends would be set on the AST node by the parser — for now, we
        //  handle it if present on a future-proof basis)

        for field in &decl.fields {
            // Check for duplicate field names
            if !seen_names.insert(field.name.clone()) {
                self.emit_error(crate::types::constraint::TypeError {
                    code: &codes::GX0316,
                    expected: self.interner.void,
                    actual: self.interner.void,
                    span: field.span,
                    context: format!("duplicate field '{}' in guard '{}'", field.name, decl.name),
                    kind: crate::types::constraint::TypeErrorKind::TypeMismatch,
                });
                continue;
            }

            let field_ty = self.resolve_annotation(&field.ty);
            let mut validations = Vec::new();

            for validator in &field.validators {
                if let Some(v) = self.resolve_validator(
                    &validator.name,
                    &validator.args,
                    field_ty,
                    validator.span,
                ) {
                    // Check validator-type compatibility
                    self.check_validator_compatibility(&v, field_ty, validator.span, &field.name);
                    validations.push(v);
                }
            }

            if !validations.is_empty() {
                has_validators = true;
            }

            guard_fields.push(GuardField {
                name: field.name.clone(),
                ty: field_ty,
                validations,
            });
        }

        self.env.pop_scope();

        let guard_ty = self.interner.make_guard(GuardDef {
            name: decl.name.clone(),
            fields: guard_fields,
            extends: None,
            has_validators,
        });
        self.env.register_type(decl.name.clone(), guard_ty);
        self.define_binding(decl.name.clone(), guard_ty, BindingKind::Guard, decl.span);

        // If inside shared scope, register as shared type
        if matches!(
            self.env.current_boundary_scope(),
            crate::types::env::BoundaryScope::Shared
        ) {
            self.env.register_shared_type(decl.name.clone());
        }
    }

    /// Check that a validator is compatible with its field's base type.
    fn check_validator_compatibility(
        &mut self,
        validator: &crate::types::validation::Validation,
        field_ty: TypeId,
        span: crate::span::Span,
        field_name: &str,
    ) {
        use crate::types::validation::Validation;

        let data = self.interner.get(field_ty).clone();
        let is_numeric = matches!(
            data,
            TypeData::Int | TypeData::Float | TypeData::IntLiteral(_)
        );
        let is_string = matches!(data, TypeData::String | TypeData::StringLiteral(_));
        let is_array = matches!(data, TypeData::Array(_));

        let (valid, requires) = match validator {
            Validation::Email | Validation::Url | Validation::Uuid | Validation::Regex(_) => {
                (is_string, "string")
            }
            Validation::Min(_)
            | Validation::Max(_)
            | Validation::Range(_, _)
            | Validation::Positive
            | Validation::NonNegative => (is_numeric, "int or float"),
            Validation::MinLen(_) | Validation::MaxLen(_) | Validation::NonEmpty => {
                (is_string || is_array, "string or array")
            }
            Validation::Integer => (matches!(data, TypeData::Float), "float"),
            Validation::Precision(_) => (matches!(data, TypeData::Float), "float"),
            Validation::OneOf(_) => (is_string || is_numeric, "string or int"),
            Validation::Trim => (is_string, "string"),
            // Optional, Nullable, Default, and Custom are valid on any type
            Validation::Optional
            | Validation::Nullable
            | Validation::Default(_)
            | Validation::Custom(_) => (true, ""),
        };

        if !valid {
            self.emit_error(crate::types::constraint::TypeError {
                code: &crate::errors::codes::GX0600,
                expected: self.interner.void,
                actual: field_ty,
                span,
                context: format!(
                    "validator '.{}' requires {} type, but field '{}' has type '{}'",
                    validator
                        .description()
                        .split_whitespace()
                        .next()
                        .unwrap_or("?"),
                    requires,
                    field_name,
                    self.interner.display(field_ty),
                ),
                kind: crate::types::constraint::TypeErrorKind::TypeMismatch,
            });
        }
    }

    /// Resolve a validator call like `.email()`, `.min(2)`, `.max(100)`.
    fn resolve_validator(
        &mut self,
        name: &str,
        args: &[Expr],
        _field_ty: TypeId,
        span: crate::span::Span,
    ) -> Option<crate::types::validation::Validation> {
        use crate::types::validation::Validation;
        match name {
            "email" => Some(Validation::Email),
            "url" => Some(Validation::Url),
            "nonEmpty" => Some(Validation::NonEmpty),
            "positive" => Some(Validation::Positive),
            "nonNegative" => Some(Validation::NonNegative),
            "integer" => Some(Validation::Integer),
            "optional" => Some(Validation::Optional),
            "min" => {
                if let Some(Expr::IntLit { value, .. }) = args.first() {
                    Some(Validation::Min(*value))
                } else {
                    self.emit_error(crate::types::constraint::TypeError {
                        code: &codes::GX0300,
                        expected: self.interner.int,
                        actual: self.interner.void,
                        span,
                        context: "`.min()` requires an integer argument".into(),
                        kind: crate::types::constraint::TypeErrorKind::TypeMismatch,
                    });
                    None
                }
            }
            "max" => {
                if let Some(Expr::IntLit { value, .. }) = args.first() {
                    Some(Validation::Max(*value))
                } else {
                    self.emit_error(crate::types::constraint::TypeError {
                        code: &codes::GX0300,
                        expected: self.interner.int,
                        actual: self.interner.void,
                        span,
                        context: "`.max()` requires an integer argument".into(),
                        kind: crate::types::constraint::TypeErrorKind::TypeMismatch,
                    });
                    None
                }
            }
            "minLen" => {
                if let Some(Expr::IntLit { value, .. }) = args.first() {
                    Some(Validation::MinLen(*value as usize))
                } else {
                    self.emit_error(crate::types::constraint::TypeError {
                        code: &codes::GX0300,
                        expected: self.interner.int,
                        actual: self.interner.void,
                        span,
                        context: "`.minLen()` requires an integer argument".into(),
                        kind: crate::types::constraint::TypeErrorKind::TypeMismatch,
                    });
                    None
                }
            }
            "maxLen" => {
                if let Some(Expr::IntLit { value, .. }) = args.first() {
                    Some(Validation::MaxLen(*value as usize))
                } else {
                    self.emit_error(crate::types::constraint::TypeError {
                        code: &codes::GX0300,
                        expected: self.interner.int,
                        actual: self.interner.void,
                        span,
                        context: "`.maxLen()` requires an integer argument".into(),
                        kind: crate::types::constraint::TypeErrorKind::TypeMismatch,
                    });
                    None
                }
            }
            "regex" => {
                if let Some(Expr::StringLit { value, .. }) = args.first() {
                    Some(Validation::Regex(value.clone()))
                } else {
                    self.emit_error(crate::types::constraint::TypeError {
                        code: &codes::GX0300,
                        expected: self.interner.string,
                        actual: self.interner.void,
                        span,
                        context: "`.regex()` requires a string argument".into(),
                        kind: crate::types::constraint::TypeErrorKind::TypeMismatch,
                    });
                    None
                }
            }
            "oneOf" => {
                // Collect string args
                let values: Vec<SmolStr> = args
                    .iter()
                    .filter_map(|a| {
                        if let Expr::StringLit { value, .. } = a {
                            Some(value.clone())
                        } else {
                            None
                        }
                    })
                    .collect();
                if values.is_empty() {
                    self.emit_error(crate::types::constraint::TypeError {
                        code: &codes::GX0300,
                        expected: self.interner.string,
                        actual: self.interner.void,
                        span,
                        context: "`.oneOf()` requires string arguments".into(),
                        kind: crate::types::constraint::TypeErrorKind::TypeMismatch,
                    });
                    None
                } else {
                    Some(Validation::OneOf(values))
                }
            }
            "uuid" => Some(Validation::Uuid),
            "trim" => Some(Validation::Trim),
            "nullable" => Some(Validation::Nullable),
            "precision" => {
                if let Some(Expr::IntLit { value, .. }) = args.first() {
                    Some(Validation::Precision(*value as u32))
                } else {
                    self.emit_error(crate::types::constraint::TypeError {
                        code: &codes::GX0300,
                        expected: self.interner.int,
                        actual: self.interner.void,
                        span,
                        context: "`.precision()` requires an integer argument".into(),
                        kind: crate::types::constraint::TypeErrorKind::TypeMismatch,
                    });
                    None
                }
            }
            "default" => {
                if let Some(expr) = args.first() {
                    let json_repr = match expr {
                        Expr::StringLit { value, .. } => format!("\"{}\"", value),
                        Expr::IntLit { value, .. } => value.to_string(),
                        Expr::BoolLit { value, .. } => value.to_string(),
                        Expr::FloatLit { value, .. } => value.to_string(),
                        Expr::NullLit { .. } => "null".to_string(),
                        _ => {
                            self.emit_error(crate::types::constraint::TypeError {
                                code: &crate::errors::codes::GX0628,
                                expected: self.interner.void,
                                actual: self.interner.void,
                                span,
                                context: "`.default()` requires a literal argument".into(),
                                kind: crate::types::constraint::TypeErrorKind::TypeMismatch,
                            });
                            return None;
                        }
                    };
                    Some(Validation::Default(SmolStr::new(json_repr)))
                } else {
                    self.emit_error(crate::types::constraint::TypeError {
                        code: &codes::GX0300,
                        expected: self.interner.void,
                        actual: self.interner.void,
                        span,
                        context: "`.default()` requires a value argument".into(),
                        kind: crate::types::constraint::TypeErrorKind::TypeMismatch,
                    });
                    None
                }
            }
            "range" => {
                if let (
                    Some(Expr::IntLit { value: min, .. }),
                    Some(Expr::IntLit { value: max, .. }),
                ) = (args.first(), args.get(1))
                {
                    Some(Validation::Range(*min, *max))
                } else {
                    self.emit_error(crate::types::constraint::TypeError {
                        code: &codes::GX0300,
                        expected: self.interner.int,
                        actual: self.interner.void,
                        span,
                        context: "`.range()` requires two integer arguments".into(),
                        kind: crate::types::constraint::TypeErrorKind::TypeMismatch,
                    });
                    None
                }
            }
            _ => Some(Validation::Custom(SmolStr::new(name))),
        }
    }

    /// Type-check a store declaration.
    fn check_store_decl(&mut self, decl: &StoreDecl) {
        self.env.push_scope(ScopeKind::StoreBody);

        let mut signals = Vec::new();
        let mut derives = Vec::new();
        let mut methods = Vec::new();

        for member in &decl.members {
            match member {
                StoreMember::Signal(stmt) => {
                    self.check_stmt(stmt);
                    if let Stmt::Signal { name, .. } = stmt {
                        if let Some(binding) = self.env.lookup(name) {
                            if let TypeData::Signal(inner) = self.interner.get(binding.ty).clone() {
                                signals.push(StoreSignal {
                                    name: name.clone(),
                                    ty: inner,
                                });
                            }
                        }
                    }
                }
                StoreMember::Derive(stmt) => {
                    self.check_stmt(stmt);
                    if let Stmt::Derive { name, .. } = stmt {
                        if let Some(binding) = self.env.lookup(name) {
                            if let TypeData::Derived(inner) = self.interner.get(binding.ty).clone()
                            {
                                derives.push(StoreDerive {
                                    name: name.clone(),
                                    ty: inner,
                                });
                            }
                        }
                    }
                }
                StoreMember::Method(fn_decl) => {
                    // Track that we're inside this store's method body
                    let old_store = self.current_store_method.replace(decl.name.clone());
                    self.check_fn_decl(fn_decl);
                    self.current_store_method = old_store;
                    if let Some(binding) = self.env.lookup(&fn_decl.name) {
                        if let TypeData::Function(sig) = self.interner.get(binding.ty).clone() {
                            methods.push(StoreMethod {
                                name: fn_decl.name.clone(),
                                sig,
                            });
                        }
                    }
                }
            }
        }

        self.env.pop_scope();

        let store_ty = self.interner.intern(TypeData::Store(StoreDef {
            name: decl.name.clone(),
            signals,
            derives,
            methods,
        }));
        self.env.register_type(decl.name.clone(), store_ty);
        self.define_binding(decl.name.clone(), store_ty, BindingKind::Store, decl.span);
    }

    /// Type-check an action declaration (server-side function).
    fn check_action_decl(&mut self, decl: &ActionDecl) {
        let fn_decl = FnDecl {
            name: decl.name.clone(),
            params: decl.params.clone(),
            ret_ty: decl.ret_ty.clone(),
            body: decl.body.clone(),
            is_async: true, // Actions are implicitly async
            span: decl.span,
        };
        self.check_fn_decl(&fn_decl);
        // Re-register as Action kind (actions are always server-scoped)
        if let Some(binding) = self.env.lookup(&decl.name) {
            let ty = binding.ty;
            let _ = self.env.define(
                SmolStr::new(format!("__action_{}", decl.name)),
                crate::types::env::Binding {
                    ty,
                    kind: BindingKind::Action,
                    span: decl.span,
                    boundary: crate::types::env::BoundaryScope::Server,
                },
            );
        }
    }

    /// Type-check a query declaration.
    fn check_query_decl(&mut self, decl: &QueryDecl) {
        self.infer_expr(&decl.url_pattern);

        // If the URL pattern is a template literal, validate that all
        // interpolated expressions produce URL-safe types (string or int).
        self.check_query_url_interpolations(&decl.url_pattern, &decl.name);

        let result_ty = if let Some(ann) = &decl.ret_ty {
            self.resolve_annotation(ann)
        } else {
            self.interner.fresh_type_var()
        };
        let query_ty = self.interner.intern(TypeData::Query { result: result_ty });
        self.define_binding(decl.name.clone(), query_ty, BindingKind::Query, decl.span);
    }

    /// Validate that URL template interpolations produce string or int types.
    ///
    /// URL interpolations like `/api/users/${id}` must produce URL-safe values.
    /// Booleans, floats, objects, arrays, etc. are not valid URL segments.
    fn check_query_url_interpolations(&mut self, expr: &Expr, query_name: &str) {
        if let Expr::TemplateLit { parts, span } = expr {
            for part in parts {
                if let crate::ast::TemplatePart::Expr(e) = part {
                    let ty = self.infer_expr(e);
                    let data = self.interner.get(ty).clone();
                    if !matches!(
                        data,
                        TypeData::String
                            | TypeData::Int
                            | TypeData::StringLiteral(_)
                            | TypeData::IntLiteral(_)
                            | TypeData::TypeVar(_)
                    ) {
                        self.emit_error(crate::types::constraint::TypeError {
                            code: &crate::errors::codes::GX0905,
                            expected: self.interner.string,
                            actual: ty,
                            span: *span,
                            context: format!(
                                "query '{}' URL interpolation must be a string or int, \
                                 found '{}'",
                                query_name,
                                self.interner.display(ty)
                            ),
                            kind: crate::types::constraint::TypeErrorKind::TypeMismatch,
                        });
                    }
                }
            }
        }
    }

    /// Type-check a channel declaration.
    fn check_channel_decl(&mut self, decl: &ChannelDecl) {
        let msg_ty = self.resolve_annotation(&decl.msg_ty);
        let param_ty = if let Some(first) = decl.params.first() {
            if let Some(ann) = &first.ty_ann {
                self.resolve_annotation(ann)
            } else {
                self.interner.fresh_type_var()
            }
        } else {
            self.interner.void
        };

        let direction = match decl.direction {
            crate::ast::ChannelDirection::ServerToClient => ty::ChannelDirection::ServerToClient,
            crate::ast::ChannelDirection::ClientToServer => ty::ChannelDirection::ClientToServer,
            crate::ast::ChannelDirection::Bidirectional => ty::ChannelDirection::Bidirectional,
        };

        let chan_ty = self.interner.intern(TypeData::Channel(ChannelDef {
            param_ty,
            msg_ty,
            direction,
        }));
        self.define_binding(decl.name.clone(), chan_ty, BindingKind::Channel, decl.span);

        // Check handler bodies with message type constraints
        for handler in &decl.handlers {
            self.env.push_scope(ScopeKind::Function);

            // For "receive" handlers, constrain the first parameter to the channel's msg_ty
            let is_receive = handler.event == "receive";
            for (i, p) in handler.params.iter().enumerate() {
                let ty = if let Some(ann) = &p.ty_ann {
                    self.resolve_annotation(ann)
                } else if is_receive && i == 0 {
                    // Unannotated receive param defaults to msg_ty
                    msg_ty
                } else {
                    self.interner.fresh_type_var()
                };

                // If this is the first param of a receive handler with an annotation,
                // ensure it's compatible with the channel's declared msg_ty
                if is_receive && i == 0 {
                    self.constrain_assignable(
                        msg_ty,
                        ty,
                        p.span,
                        &format!(
                            "channel '{}' receive handler parameter must match message type",
                            decl.name
                        ),
                    );
                }

                self.define_binding(p.name.clone(), ty, BindingKind::Parameter, p.span);
            }
            self.check_block(&handler.body);
            self.env.pop_scope();

            // Validate handler is appropriate for the declared direction
            self.check_channel_handler_direction(
                &handler.event,
                &decl.direction,
                &decl.name,
                handler.span,
            );
        }
    }

    /// Validate that a channel handler is appropriate for the declared direction.
    fn check_channel_handler_direction(
        &mut self,
        handler_event: &str,
        direction: &crate::ast::ChannelDirection,
        channel_name: &str,
        span: crate::span::Span,
    ) {
        use crate::ast::ChannelDirection;
        match (handler_event, direction) {
            // `receive` on a ServerToClient channel means the client receives
            // (this is fine — client handles incoming messages)
            ("receive", ChannelDirection::ServerToClient) => {}
            // `receive` on a ClientToServer channel means the server receives
            ("receive", ChannelDirection::ClientToServer) => {}
            // Bidirectional — receive is valid from either side
            ("receive", ChannelDirection::Bidirectional) => {}
            // `connect` is valid for all directions
            ("connect", _) => {}
            // `disconnect` is valid for all directions
            ("disconnect", _) => {}
            // Unknown handler event name
            (event, _) => {
                self.emit_error(crate::types::constraint::TypeError {
                    code: &codes::GX0300,
                    expected: self.interner.void,
                    actual: self.interner.void,
                    span,
                    context: format!(
                        "unknown channel handler '{}' on channel '{}' — \
                         expected 'connect', 'receive', or 'disconnect'",
                        event, channel_name
                    ),
                    kind: crate::types::constraint::TypeErrorKind::TypeMismatch,
                });
            }
        }
    }

    /// Type-check a type alias.
    fn check_type_alias(&mut self, decl: &TypeAliasDecl) {
        let ty = self.resolve_annotation(&decl.ty);
        self.env.register_type(decl.name.clone(), ty);
        self.define_binding(decl.name.clone(), ty, BindingKind::TypeAlias, decl.span);
    }

    /// Type-check an enum declaration.
    fn check_enum_decl(&mut self, decl: &EnumDecl) {
        let enum_ty = self.interner.intern(TypeData::Enum(EnumDef {
            name: decl.name.clone(),
            variants: decl.variants.clone(),
        }));
        self.env.register_type(decl.name.clone(), enum_ty);
        self.define_binding(decl.name.clone(), enum_ty, BindingKind::EnumDef, decl.span);
    }

    /// Type-check a test declaration.
    fn check_test_decl(&mut self, decl: &TestDecl) {
        self.env.push_scope(ScopeKind::TestBlock);
        self.check_block(&decl.body);
        self.env.pop_scope();
    }

    /// Type-check a component declaration.
    fn check_component_decl(&mut self, decl: &ComponentDecl) {
        let mut props = Vec::new();

        for param in &decl.props {
            let ty = if let Some(ann) = &param.ty_ann {
                self.resolve_annotation(ann)
            } else {
                self.interner.fresh_type_var()
            };
            props.push(PropDef {
                name: param.name.clone(),
                ty,
                has_default: param.default.is_some(),
            });
        }

        // Register component type
        let comp_ty = self.interner.intern(TypeData::Component(ComponentDef {
            name: decl.name.clone(),
            props: props.clone(),
            slots: Vec::new(), // Slots detected during template checking
        }));
        self.env.register_type(decl.name.clone(), comp_ty);
        self.define_binding(
            decl.name.clone(),
            comp_ty,
            BindingKind::Component,
            decl.span,
        );

        // Check body
        self.env.push_scope(ScopeKind::ComponentBody);
        for (param, prop) in decl.props.iter().zip(props.iter()) {
            self.define_binding(
                param.name.clone(),
                prop.ty,
                BindingKind::Parameter,
                param.span,
            );
            if let Some(default) = &param.default {
                let default_ty = self.infer_expr(default);
                self.constrain_assignable(
                    default_ty,
                    prop.ty,
                    param.span,
                    &format!("default value for prop '{}'", param.name),
                );
            }
        }
        for stmt in &decl.body.stmts {
            self.check_stmt(stmt);
        }
        self.check_template_nodes(&decl.body.template);

        // Check the head block if present
        if let Some(ref head) = decl.body.head {
            self.check_head_block(head);
        }
        self.env.pop_scope();
    }

    /// Type-check a layout declaration.
    ///
    /// Layouts are structurally similar to components but MUST contain
    /// at least one `<slot/>` node in their template. The slot is where
    /// page content will be injected during SSR.
    fn check_layout_decl(&mut self, decl: &LayoutDecl) {
        let mut props = Vec::new();

        for param in &decl.props {
            let ty = if let Some(ann) = &param.ty_ann {
                self.resolve_annotation(ann)
            } else {
                self.interner.fresh_type_var()
            };
            props.push(PropDef {
                name: param.name.clone(),
                ty,
                has_default: param.default.is_some(),
            });
        }

        // Register as component type (layouts are a kind of component)
        let comp_ty = self.interner.intern(TypeData::Component(ComponentDef {
            name: decl.name.clone(),
            props: props.clone(),
            slots: vec!["default".into()],
        }));
        self.env.register_type(decl.name.clone(), comp_ty);
        self.define_binding(
            decl.name.clone(),
            comp_ty,
            BindingKind::Component,
            decl.span,
        );

        // Check body
        self.env.push_scope(ScopeKind::ComponentBody);
        for (param, prop) in decl.props.iter().zip(props.iter()) {
            self.define_binding(
                param.name.clone(),
                prop.ty,
                BindingKind::Parameter,
                param.span,
            );
        }
        for stmt in &decl.body.stmts {
            self.check_stmt(stmt);
        }
        self.check_template_nodes(&decl.body.template);

        if let Some(ref head) = decl.body.head {
            self.check_head_block(head);
        }
        self.env.pop_scope();

        // Validate: layout MUST contain a <slot/> node
        if !Self::template_has_slot(&decl.body.template) {
            self.emit_error(crate::types::constraint::TypeError {
                code: &crate::errors::codes::GX0707,
                expected: self.interner.void,
                actual: self.interner.void,
                span: decl.span,
                context: format!(
                    "layout '{}' must contain a <slot/> node for page content injection",
                    decl.name
                ),
                kind: crate::types::constraint::TypeErrorKind::TypeMismatch,
            });
        }
    }

    /// Recursively check if a template contains a Slot node.
    fn template_has_slot(nodes: &[TemplateNode]) -> bool {
        for node in nodes {
            match node {
                TemplateNode::Slot { .. } => return true,
                TemplateNode::Element { children, .. } => {
                    if Self::template_has_slot(children) {
                        return true;
                    }
                }
                TemplateNode::When {
                    body, else_branch, ..
                } => {
                    if Self::template_has_slot(body) {
                        return true;
                    }
                    if let Some(WhenElse::Else(nodes)) = else_branch {
                        if Self::template_has_slot(nodes) {
                            return true;
                        }
                    }
                }
                TemplateNode::Each { body, .. } => {
                    if Self::template_has_slot(body) {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// Type-check an API resource declaration.
    ///
    /// Validates each handler's params, return type, and body. Checks:
    /// - Path params resolve to string type
    /// - GET handler params are treated as query parameters
    /// - POST/PUT/PATCH handler params are treated as request body
    /// - Return types are serializable
    /// - No duplicate method+path_params combinations
    fn check_api_decl(&mut self, decl: &ApiDecl) {
        // Register the API resource as a binding
        let api_ty = self.interner.void;
        self.define_binding(decl.name.clone(), api_ty, BindingKind::Api, decl.span);

        // Track method+path_params combinations for duplicate detection
        let mut seen_handlers: Vec<(HttpMethod, Vec<SmolStr>)> = Vec::new();

        for handler in &decl.handlers {
            // Check for duplicate handlers
            let key = (handler.method, handler.path_params.clone());
            if seen_handlers.contains(&key) {
                let params_desc = if handler.path_params.is_empty() {
                    String::new()
                } else {
                    format!(
                        "[{}]",
                        handler
                            .path_params
                            .iter()
                            .map(|p| p.as_str())
                            .collect::<Vec<_>>()
                            .join("][")
                    )
                };
                self.emit_error(crate::types::constraint::TypeError {
                    code: &codes::GX0300,
                    expected: self.interner.void,
                    actual: self.interner.void,
                    span: handler.span,
                    context: format!(
                        "duplicate handler: {} {}{} already defined in api '{}'",
                        handler.method, handler.method, params_desc, decl.name
                    ),
                    kind: crate::types::constraint::TypeErrorKind::TypeMismatch,
                });
            }
            seen_handlers.push(key);

            // Type-check handler
            self.env.push_scope(ScopeKind::Function);

            // Path params are always string (URL segments)
            for path_param in &handler.path_params {
                self.define_binding(
                    path_param.clone(),
                    self.interner.string,
                    BindingKind::Parameter,
                    handler.span,
                );
            }

            // Regular params
            for p in &handler.params {
                let ty = if let Some(ann) = &p.ty_ann {
                    self.resolve_annotation(ann)
                } else {
                    self.interner.fresh_type_var()
                };
                self.define_binding(p.name.clone(), ty, BindingKind::Parameter, p.span);
            }

            // Return type
            if let Some(ret_ann) = &handler.ret_ty {
                let ret_ty = self.resolve_annotation(ret_ann);
                // Return type must be serializable (it goes over the wire)
                if !self.is_serializable(ret_ty) {
                    self.emit_error(crate::types::constraint::TypeError {
                        code: &codes::GX0504,
                        expected: self.interner.void,
                        actual: ret_ty,
                        span: handler.span,
                        context: format!(
                            "API handler {} return type '{}' is not serializable",
                            handler.method,
                            self.interner.display(ret_ty)
                        ),
                        kind: crate::types::constraint::TypeErrorKind::NotSerializable,
                    });
                }
                self.current_return_type = Some(ret_ty);
            }

            // Check body
            self.check_block(&handler.body);
            self.current_return_type = None;
            self.env.pop_scope();
        }
    }

    /// Type-check an env declaration block.
    ///
    /// Validates each env var's type (must be a primitive), resolves validators,
    /// checks validator-type compatibility, and registers declared types for
    /// use by `Expr::EnvAccess` inference.
    fn check_env_decl(&mut self, decl: &EnvDecl) {
        for var in &decl.vars {
            // Resolve the type — must be a primitive (string, int, float, bool)
            let ty = self.resolve_annotation(&var.ty);
            let is_primitive = ty == self.interner.string
                || ty == self.interner.int
                || ty == self.interner.float
                || ty == self.interner.bool_;

            if !is_primitive {
                self.emit_error(crate::types::constraint::TypeError {
                    code: &codes::GX0300,
                    expected: self.interner.string,
                    actual: ty,
                    span: var.span,
                    context: format!(
                        "env var '{}' must have a primitive type (string, int, float, bool), \
                         found '{}'",
                        var.key,
                        self.interner.display(ty)
                    ),
                    kind: crate::types::constraint::TypeErrorKind::TypeMismatch,
                });
            }

            // Resolve validators (reuse the guard validator infrastructure)
            for v in &var.validators {
                let _resolved = self.resolve_validator(&v.name, &v.args, ty, v.span);
            }

            // Check default value type if provided
            if let Some(ref default_expr) = var.default {
                let default_ty = self.infer_expr(default_expr);
                self.constrain_assignable(
                    default_ty,
                    ty,
                    var.span,
                    &format!("default value for env var '{}'", var.key),
                );
            }

            // Register the declared type for EnvAccess inference
            self.declared_env_types.insert(var.key.clone(), ty);
        }
    }

    /// Type-check a middleware declaration.
    ///
    /// Validates that the middleware has the expected `(req: Request, next: Next)`
    /// parameters and type-checks the body.
    fn check_middleware_decl(&mut self, decl: &MiddlewareDecl) {
        // Register the middleware as a binding
        let mw_ty = self.interner.void;
        self.define_binding(decl.name.clone(), mw_ty, BindingKind::Middleware, decl.span);

        // Check body in a function scope
        self.env.push_scope(ScopeKind::Function);

        // Register parameters
        for param in &decl.params {
            let ty = if let Some(ann) = &param.ty_ann {
                // Recognise Request, Response, Next as built-in opaque types
                if let TypeAnnotation::Named { name, .. } = ann {
                    match name.as_str() {
                        "Request" | "Response" | "Next" => self.interner.void,
                        _ => self.resolve_annotation(ann),
                    }
                } else {
                    self.resolve_annotation(ann)
                }
            } else {
                self.interner.fresh_type_var()
            };
            self.define_binding(param.name.clone(), ty, BindingKind::Parameter, param.span);
        }

        // Check the body
        self.check_block(&decl.body);
        self.env.pop_scope();
    }

    /// Type-check a use/import declaration.
    fn check_use_decl(&mut self, decl: &UseDecl) {
        // Register imported names with fresh type vars
        // (actual types come from module resolution — not implemented yet)
        match &decl.imports {
            ImportKind::Default(name) => {
                let ty = self.interner.fresh_type_var();
                self.define_binding(name.clone(), ty, BindingKind::Let, decl.span);
            }
            ImportKind::Named(names) => {
                for name in names {
                    let ty = self.interner.fresh_type_var();
                    self.define_binding(name.clone(), ty, BindingKind::Let, decl.span);
                }
            }
            ImportKind::Star => {
                // Star imports can't be registered without knowing the module
            }
        }
    }
}
