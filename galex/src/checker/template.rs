//! Template node type checking and directive validation.

use super::dom;
use super::TypeChecker;
use crate::ast::*;
use crate::types::constraint::TypeErrorKind;
use crate::types::env::{BindingKind, ScopeKind};
use crate::types::ty::{TypeData, TypeId};

impl TypeChecker {
    /// Type-check a list of template nodes.
    pub(super) fn check_template_nodes(&mut self, nodes: &[TemplateNode]) {
        for node in nodes {
            self.check_template_node(node);
        }
    }

    /// Type-check a single template node.
    fn check_template_node(&mut self, node: &TemplateNode) {
        match node {
            TemplateNode::Element {
                tag,
                attributes,
                directives,
                children,
                ..
            } => {
                self.check_attributes(attributes);
                self.check_directives(directives, tag);
                self.check_template_nodes(children);
            }

            TemplateNode::SelfClosing {
                tag,
                attributes,
                directives,
                ..
            } => {
                self.check_attributes(attributes);
                self.check_directives(directives, tag);
            }

            TemplateNode::Text { .. } => {
                // No type checking needed for static text
            }

            TemplateNode::ExprInterp { expr, span } => {
                let ty = self.infer_expr(expr);
                if !self.is_renderable(ty) {
                    self.error_type_mismatch(
                        self.interner.string,
                        ty,
                        *span,
                        "template expression must produce a renderable type (string, int, float, or bool)",
                    );
                }
            }

            TemplateNode::When {
                condition,
                body,
                else_branch,
                span,
            } => {
                let cond_ty = self.infer_expr(condition);
                self.constrain_assignable(
                    cond_ty,
                    self.interner.bool_,
                    *span,
                    "`when` condition must be a boolean",
                );
                self.check_template_nodes(body);
                if let Some(else_b) = else_branch {
                    match else_b {
                        WhenElse::Else(nodes) => self.check_template_nodes(nodes),
                        WhenElse::ElseWhen(node) => self.check_template_node(node),
                    }
                }
            }

            TemplateNode::Each {
                binding,
                index,
                iterable,
                body,
                empty,
                span,
            } => {
                let iter_ty = self.infer_expr(iterable);
                let iter_data = self.interner.get(iter_ty).clone();

                let elem_ty = match iter_data {
                    TypeData::Array(elem) => elem,
                    TypeData::TypeVar(_) => self.interner.fresh_type_var(),
                    _ => {
                        self.error_type_mismatch(
                            self.interner.void,
                            iter_ty,
                            *span,
                            "`each` requires an iterable (array)",
                        );
                        self.interner.never
                    }
                };

                self.env.push_scope(ScopeKind::ForLoop);
                self.define_binding(binding.clone(), elem_ty, BindingKind::ForBinding, *span);
                if let Some(idx_name) = index {
                    self.define_binding(
                        idx_name.clone(),
                        self.interner.int,
                        BindingKind::ForBinding,
                        *span,
                    );
                }
                self.check_template_nodes(body);
                self.env.pop_scope();

                if let Some(empty_nodes) = empty {
                    self.check_template_nodes(empty_nodes);
                }
            }

            TemplateNode::Suspend { fallback, body, .. } => {
                if let Some(fb) = fallback {
                    self.check_template_node(fb);
                }
                self.check_template_nodes(body);
            }

            TemplateNode::Slot { default, .. } => {
                if let Some(default_nodes) = default {
                    self.check_template_nodes(default_nodes);
                }
            }
        }
    }

    /// Check attribute values.
    fn check_attributes(&mut self, attrs: &[Attribute]) {
        for attr in attrs {
            if let AttrValue::Expr(expr) = &attr.value {
                self.infer_expr(expr);
            }
        }
    }

    /// Check template directives with element tag context.
    ///
    /// The `tag` is used by `bind:`, `ref:`, and `on:` to validate
    /// type compatibility with the HTML element.
    fn check_directives(&mut self, directives: &[Directive], tag: &str) {
        let mut form_guard_ty: Option<TypeId> = None;
        let mut form_action_ty: Option<TypeId> = None;
        let mut form_span = crate::span::Span::dummy();

        for directive in directives {
            match directive {
                Directive::Bind { field, span } => {
                    self.check_bind_directive(field, tag, *span);
                }

                Directive::On {
                    event,
                    handler,
                    span,
                    ..
                } => {
                    self.check_on_directive(event, handler, *span);
                }

                Directive::Class {
                    condition, span, ..
                } => {
                    let cond_ty = self.infer_expr(condition);
                    self.constrain_assignable(
                        cond_ty,
                        self.interner.bool_,
                        *span,
                        "`class:` condition must be a boolean",
                    );
                }

                Directive::Ref { name, span } => {
                    self.check_ref_directive(name, tag, *span);
                }

                Directive::Key { expr, span } => {
                    let key_ty = self.infer_expr(expr);
                    let key_data = self.interner.get(key_ty).clone();
                    if !matches!(
                        key_data,
                        TypeData::String
                            | TypeData::Int
                            | TypeData::StringLiteral(_)
                            | TypeData::IntLiteral(_)
                            | TypeData::TypeVar(_)
                    ) {
                        self.error_type_mismatch(
                            self.interner.string,
                            key_ty,
                            *span,
                            "`key` must be a string or integer",
                        );
                    }
                }

                Directive::FormAction { action, span } => {
                    let action_ty = self.infer_expr(action);
                    let data = self.interner.get(action_ty).clone();
                    if !matches!(data, TypeData::Function(_) | TypeData::TypeVar(_)) {
                        self.error_type_mismatch(
                            self.interner.void,
                            action_ty,
                            *span,
                            "`form:action` must be a function or action",
                        );
                    }
                    // Also check if the expression is an identifier bound as Action
                    self.check_form_action_is_action(action, *span);
                    form_action_ty = Some(action_ty);
                    form_span = *span;
                }

                Directive::FormGuard { guard, span } => {
                    let guard_ty = self.infer_expr(guard);
                    let data = self.interner.get(guard_ty).clone();
                    if !matches!(data, TypeData::Guard(_) | TypeData::TypeVar(_)) {
                        self.error_type_mismatch(
                            self.interner.void,
                            guard_ty,
                            *span,
                            "`form:guard` must be a guard type",
                        );
                    }
                    form_guard_ty = Some(guard_ty);
                    form_span = *span;
                }

                Directive::Transition { config, .. } => {
                    if let Some(expr) = config {
                        self.infer_expr(expr);
                    }
                }
                Directive::Into { .. }
                | Directive::FormError { .. }
                | Directive::Prefetch { .. } => {}
            }
        }

        // Cross-validate form:guard + form:action if both present
        if let (Some(guard_ty), Some(action_ty)) = (form_guard_ty, form_action_ty) {
            self.cross_validate_form_guard_action(guard_ty, action_ty, form_span);
        }
    }

    // ── bind: directive ────────────────────────────────────────────

    /// Check a `bind:field` directive.
    ///
    /// 1. The referenced variable must be a Signal.
    /// 2. The signal's inner type must be compatible with the element attribute.
    fn check_bind_directive(&mut self, field: &str, tag: &str, span: crate::span::Span) {
        if let Some(binding) = self.env.lookup(field) {
            let data = self.interner.get(binding.ty).clone();
            match data {
                TypeData::Signal(inner_ty) => {
                    // Check type compatibility with the element attribute
                    let expected_type_name = dom::bind_expected_type(tag, field);
                    if let Some(expected_ty) = self.env.resolve_type(expected_type_name) {
                        self.constrain_assignable(
                            inner_ty,
                            expected_ty,
                            span,
                            &format!(
                                "`bind:{}` on <{}> expects signal<{}>, found signal<{}>",
                                field,
                                tag,
                                expected_type_name,
                                self.interner.display(inner_ty)
                            ),
                        );
                    }
                }
                TypeData::TypeVar(_) => {
                    // Type not yet resolved — defer to constraint solver
                }
                _ => {
                    self.emit_error(crate::types::constraint::TypeError {
                        expected: self.interner.void,
                        actual: binding.ty,
                        span,
                        context: format!(
                            "`bind:{}` requires a signal, but '{}' is not a signal",
                            field, field
                        ),
                        kind: TypeErrorKind::TypeMismatch,
                    });
                }
            }
        } else {
            self.emit_error(crate::types::constraint::TypeError {
                expected: self.interner.void,
                actual: self.interner.void,
                span,
                context: format!(
                    "undefined variable '{}' in `bind:{}` directive",
                    field, field
                ),
                kind: TypeErrorKind::TypeMismatch,
            });
        }
    }

    // ── on: directive ──────────────────────────────────────────────

    /// Check an `on:event` directive handler.
    ///
    /// 1. The handler must be a function.
    /// 2. If the handler takes a parameter, its type must be compatible
    ///    with the expected DOM event type for the event name.
    fn check_on_directive(&mut self, event: &str, handler: &Expr, span: crate::span::Span) {
        let handler_ty = self.infer_expr(handler);
        let handler_data = self.interner.get(handler_ty).clone();

        match handler_data {
            TypeData::Function(sig) => {
                // If the handler takes a parameter, check it against the event type
                if let Some(first_param) = sig.params.first() {
                    if let Some(event_type_name) = dom::event_param_type(event) {
                        if let Some(event_ty) = self.env.resolve_type(event_type_name) {
                            self.constrain_assignable(
                                event_ty,
                                first_param.ty,
                                span,
                                &format!(
                                    "on:{} handler parameter must accept {}, found '{}'",
                                    event,
                                    event_type_name,
                                    self.interner.display(first_param.ty)
                                ),
                            );
                        }
                    }
                }
                // 0-param handlers are always valid (void event handler)
            }
            TypeData::TypeVar(_) => {
                // Type not yet resolved — defer
            }
            _ => {
                self.emit_error(crate::types::constraint::TypeError {
                    expected: self.interner.void,
                    actual: handler_ty,
                    span,
                    context: "event handler must be a function".into(),
                    kind: TypeErrorKind::TypeMismatch,
                });
            }
        }
    }

    // ── ref: directive ─────────────────────────────────────────────

    /// Check a `ref:name` directive.
    ///
    /// 1. The referenced variable must be a DomRef.
    /// 2. The DomRef's inner type must be compatible with the element tag.
    fn check_ref_directive(&mut self, name: &str, tag: &str, span: crate::span::Span) {
        if let Some(binding) = self.env.lookup(name) {
            let data = self.interner.get(binding.ty).clone();
            match data {
                TypeData::DomRef(inner_ty) => {
                    // Check that the ref's inner type matches the element's DOM type
                    let expected_type_name = dom::element_type(tag);
                    if let Some(expected_ty) = self.env.resolve_type(expected_type_name) {
                        self.constrain_assignable(
                            expected_ty,
                            inner_ty,
                            span,
                            &format!(
                                "`ref:{}` on <{}> requires ref<{}>, found ref<{}>",
                                name,
                                tag,
                                expected_type_name,
                                self.interner.display(inner_ty)
                            ),
                        );
                    }
                }
                TypeData::TypeVar(_) => {
                    // Defer
                }
                _ => {
                    self.emit_error(crate::types::constraint::TypeError {
                        expected: self.interner.void,
                        actual: binding.ty,
                        span,
                        context: format!("`ref:{}` requires a ref declaration", name),
                        kind: TypeErrorKind::TypeMismatch,
                    });
                }
            }
        }
    }

    // ── form: directives ───────────────────────────────────────────

    /// Soft check: warn if `form:action` references something that isn't an Action binding.
    fn check_form_action_is_action(&mut self, action_expr: &Expr, _span: crate::span::Span) {
        // Only check direct identifier references
        if let Expr::Ident { name, .. } = action_expr {
            // Look for the __action_ shadow binding registered by check_action_decl
            let action_key = format!("__action_{}", name);
            let is_action = self.env.lookup(&action_key).is_some();
            // If the name resolves but has no action shadow, it's a plain function —
            // still valid, just not semantically an "action" (no warning for now,
            // as this is a soft enhancement).
            let _ = is_action;
        }
    }

    /// Cross-validate that a form:guard's fields are compatible with a form:action's
    /// first parameter type.
    fn cross_validate_form_guard_action(
        &mut self,
        guard_ty: TypeId,
        action_ty: TypeId,
        span: crate::span::Span,
    ) {
        let guard_data = self.interner.get(guard_ty).clone();
        let action_data = self.interner.get(action_ty).clone();

        if let (TypeData::Guard(guard), TypeData::Function(sig)) = (&guard_data, &action_data) {
            if let Some(first_param) = sig.params.first() {
                // The guard should be assignable to the action's first parameter type
                self.constrain_assignable(
                    guard_ty,
                    first_param.ty,
                    span,
                    &format!(
                        "form:guard '{}' must be compatible with form:action parameter type",
                        guard.name
                    ),
                );
            }
        }
    }

    // ── Head block ─────────────────────────────────────────────────

    /// Type-check a `head { ... }` block inside a component.
    ///
    /// Validates:
    /// - Property names are known head properties
    /// - Value types match expected types (e.g., `title` must be string)
    pub(super) fn check_head_block(&mut self, head: &HeadBlock) {
        for field in &head.fields {
            let value_ty = self.infer_expr(&field.value);

            match dom::head_property_type(&field.key) {
                Some(dom::HeadPropertyType::String) => {
                    // Value must be a string
                    self.constrain_assignable(
                        value_ty,
                        self.interner.string,
                        field.span,
                        &format!("head property '{}' must be a string", field.key),
                    );
                }
                Some(dom::HeadPropertyType::StringObject) => {
                    // Value must be an object (with string fields)
                    let data = self.interner.get(value_ty).clone();
                    if !matches!(data, TypeData::Object(_) | TypeData::TypeVar(_)) {
                        self.error_type_mismatch(
                            self.interner.void,
                            value_ty,
                            field.span,
                            &format!(
                                "head property '{}' expects an object with string values",
                                field.key
                            ),
                        );
                    }
                }
                None => {
                    // Unknown head property — emit a warning-level error
                    self.emit_error(crate::types::constraint::TypeError {
                        expected: self.interner.void,
                        actual: self.interner.void,
                        span: field.span,
                        context: format!(
                            "unknown head property '{}' — valid properties include: \
                             title, description, charset, viewport, canonical, og, twitter",
                            field.key
                        ),
                        kind: TypeErrorKind::TypeMismatch,
                    });
                }
            }
        }
    }
}
