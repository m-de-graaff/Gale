//! Expression type inference — returns a TypeId for each Expr variant.

use super::TypeChecker;
use crate::ast::*;
use crate::types::constraint::TypeErrorKind;
use crate::types::ty::*;

impl TypeChecker {
    /// Infer the type of an expression.
    pub(super) fn infer_expr(&mut self, expr: &Expr) -> TypeId {
        match expr {
            // ── Literals ────────────────────────────────────────
            Expr::IntLit { .. } => self.interner.int,
            Expr::FloatLit { .. } => self.interner.float,
            Expr::StringLit { value, .. } => self.interner.make_string_literal(value),
            Expr::BoolLit { .. } => self.interner.bool_,
            Expr::NullLit { .. } => self.interner.null,
            Expr::RegexLit { .. } => self.interner.make_named("RegExp"),
            Expr::EnvAccess { key, span } => {
                self.check_env_access(key, *span);
                // Return the declared type if this env var was declared,
                // otherwise fall back to string.
                self.declared_env_types
                    .get(key)
                    .copied()
                    .unwrap_or(self.interner.string)
            }

            // ── Template literal ────────────────────────────────
            Expr::TemplateLit { parts, span } => {
                for part in parts {
                    if let TemplatePart::Expr(e) = part {
                        let ty = self.infer_expr(e);
                        if !self.is_renderable(ty) {
                            self.error_type_mismatch(
                                self.interner.string,
                                ty,
                                *span,
                                "template interpolation must be a renderable type",
                            );
                        }
                    }
                }
                self.interner.string
            }

            // ── Identifier ──────────────────────────────────────
            Expr::Ident { name, span } => {
                if let Some(binding) = self.env.lookup(name) {
                    let binding_clone = binding.clone();
                    // Check cross-boundary access before returning the type
                    self.check_boundary_access(name, &binding_clone, *span);
                    binding_clone.ty
                } else {
                    self.emit_error(crate::types::constraint::TypeError {
                        expected: self.interner.void,
                        actual: self.interner.void,
                        span: *span,
                        context: format!("undefined variable '{}'", name),
                        kind: TypeErrorKind::TypeMismatch,
                    });
                    self.interner.fresh_type_var()
                }
            }

            // ── Binary operations ───────────────────────────────
            Expr::BinaryOp {
                left,
                op,
                right,
                span,
            } => {
                let left_ty = self.infer_expr(left);
                let right_ty = self.infer_expr(right);
                self.infer_binary_op(left_ty, *op, right_ty, *span)
            }

            // ── Unary operations ────────────────────────────────
            Expr::UnaryOp { op, operand, span } => {
                let operand_ty = self.infer_expr(operand);
                self.infer_unary_op(*op, operand_ty, *span)
            }

            // ── Ternary ─────────────────────────────────────────
            Expr::Ternary {
                condition,
                then_expr,
                else_expr,
                span,
            } => {
                let cond_ty = self.infer_expr(condition);
                self.constrain_assignable(cond_ty, self.interner.bool_, *span, "ternary condition");
                let then_ty = self.infer_expr(then_expr);
                let else_ty = self.infer_expr(else_expr);
                // Result is the union of both branches
                self.interner.make_union(vec![then_ty, else_ty])
            }

            // ── Null coalescing ─────────────────────────────────
            Expr::NullCoalesce {
                left,
                right,
                span: _,
            } => {
                let left_ty = self.infer_expr(left);
                let right_ty = self.infer_expr(right);
                let left_data = self.interner.get(left_ty).clone();

                match left_data {
                    TypeData::Optional(inner) => {
                        // T? ?? U → T | U
                        self.interner.make_union(vec![inner, right_ty])
                    }
                    TypeData::Union(members) if members.contains(&self.interner.null) => {
                        // (T | null) ?? U → T | U (remove null, add right)
                        let mut non_null: Vec<_> = members
                            .into_iter()
                            .filter(|&m| m != self.interner.null)
                            .collect();
                        non_null.push(right_ty);
                        self.interner.make_union(non_null)
                    }
                    TypeData::TypeVar(_) => {
                        // Defer — return union
                        self.interner.make_union(vec![left_ty, right_ty])
                    }
                    _ => {
                        // Left isn't nullable — ?? is unnecessary but not an error
                        left_ty
                    }
                }
            }

            // ── Function call ───────────────────────────────────
            Expr::FnCall { callee, args, span } => {
                // Check for guard composition methods: .partial(), .pick(), .omit()
                if let Some(result) = self.try_guard_composition(callee, args, *span) {
                    return result;
                }

                let callee_ty = self.infer_expr(callee);
                let callee_data = self.interner.get(callee_ty).clone();

                match callee_data {
                    TypeData::Function(sig) => {
                        // Check arity
                        let required = sig.params.iter().filter(|p| !p.has_default).count();
                        if args.len() < required || args.len() > sig.params.len() {
                            self.emit_error(crate::types::constraint::TypeError {
                                expected: callee_ty,
                                actual: callee_ty,
                                span: *span,
                                context: format!(
                                    "expected {} argument(s), found {}",
                                    sig.params.len(),
                                    args.len()
                                ),
                                kind: TypeErrorKind::ArityMismatch {
                                    expected: sig.params.len(),
                                    actual: args.len(),
                                },
                            });
                        }
                        // Check arg types
                        for (i, arg) in args.iter().enumerate() {
                            let arg_ty = self.infer_expr(arg);
                            if let Some(param) = sig.params.get(i) {
                                self.constrain_assignable(
                                    arg_ty,
                                    param.ty,
                                    *span,
                                    &format!("argument {} of function call", i + 1),
                                );
                            }
                        }
                        sig.ret
                    }
                    TypeData::TypeVar(_) => {
                        // Callee type unknown — infer args and return fresh var
                        for arg in args {
                            self.infer_expr(arg);
                        }
                        self.interner.fresh_type_var()
                    }
                    _ => {
                        self.error_type_mismatch(
                            self.interner.void,
                            callee_ty,
                            *span,
                            "expression is not callable",
                        );
                        self.interner.never
                    }
                }
            }

            // ── Member access ───────────────────────────────────
            Expr::MemberAccess {
                object,
                field,
                span,
            } => {
                let obj_ty = self.infer_expr(object);
                self.resolve_member(obj_ty, field, *span, false)
            }

            // ── Optional chaining ───────────────────────────────
            Expr::OptionalChain {
                object,
                field,
                span,
            } => {
                let obj_ty = self.infer_expr(object);
                let field_ty = self.resolve_member(obj_ty, field, *span, true);
                self.interner.make_optional(field_ty)
            }

            // ── Index access ────────────────────────────────────
            Expr::IndexAccess {
                object,
                index,
                span,
            } => {
                let obj_ty = self.infer_expr(object);
                let idx_ty = self.infer_expr(index);
                let obj_data = self.interner.get(obj_ty).clone();

                match obj_data {
                    TypeData::Array(elem) => {
                        self.constrain_assignable(idx_ty, self.interner.int, *span, "array index");
                        elem
                    }
                    TypeData::Tuple(elems) => {
                        self.constrain_assignable(idx_ty, self.interner.int, *span, "tuple index");
                        // Can't know which element at compile time — return union
                        self.interner.make_union(elems)
                    }
                    TypeData::TypeVar(_) => self.interner.fresh_type_var(),
                    _ => {
                        self.error_type_mismatch(
                            self.interner.void,
                            obj_ty,
                            *span,
                            "expression is not indexable",
                        );
                        self.interner.never
                    }
                }
            }

            // ── Array literal ───────────────────────────────────
            Expr::ArrayLit { elements, span } => {
                if elements.is_empty() {
                    let elem = self.interner.fresh_type_var();
                    self.interner.make_array(elem)
                } else {
                    let first_ty = self.infer_expr(&elements[0]);
                    for elem in &elements[1..] {
                        let elem_ty = self.infer_expr(elem);
                        self.constrain_assignable(elem_ty, first_ty, *span, "array element type");
                    }
                    self.interner.make_array(first_ty)
                }
            }

            // ── Object literal ──────────────────────────────────
            Expr::ObjectLit { fields, .. } => {
                let obj_fields: Vec<_> = fields
                    .iter()
                    .map(|f| {
                        let ty = self.infer_expr(&f.value);
                        ObjectField {
                            name: f.key.clone(),
                            ty,
                            optional: false,
                        }
                    })
                    .collect();
                self.interner.intern(TypeData::Object(obj_fields))
            }

            // ── Arrow function ──────────────────────────────────
            Expr::ArrowFn {
                params,
                ret_ty,
                body,
                span,
            } => {
                self.env.push_scope(crate::types::env::ScopeKind::Function);

                let fn_params: Vec<FnParam> = params
                    .iter()
                    .map(|p| {
                        let ty = if let Some(ann) = &p.ty_ann {
                            self.resolve_annotation(ann)
                        } else {
                            self.interner.fresh_type_var()
                        };
                        self.define_binding(
                            p.name.clone(),
                            ty,
                            crate::types::env::BindingKind::Parameter,
                            p.span,
                        );
                        FnParam {
                            name: p.name.clone(),
                            ty,
                            has_default: p.default.is_some(),
                        }
                    })
                    .collect();

                let ret = if let Some(ret_ann) = ret_ty {
                    self.resolve_annotation(ret_ann)
                } else {
                    self.interner.fresh_type_var()
                };

                let old_return = self.current_return_type.replace(ret);

                let body_ty = match body {
                    ArrowBody::Expr(e) => self.infer_expr(e),
                    ArrowBody::Block(block) => {
                        self.check_block(block);
                        self.interner.void
                    }
                };

                // For expression bodies, the body IS the return value
                if matches!(body, ArrowBody::Expr(_)) {
                    self.constrain_assignable(body_ty, ret, *span, "arrow function return");
                }

                self.current_return_type = old_return;
                self.env.pop_scope();

                self.interner.make_function(FunctionSig {
                    params: fn_params,
                    ret,
                    is_async: false,
                })
            }

            // ── Spread ──────────────────────────────────────────
            Expr::Spread { expr, .. } => self.infer_expr(expr),

            // ── Range ───────────────────────────────────────────
            Expr::Range { start, end, span } => {
                let start_ty = self.infer_expr(start);
                let end_ty = self.infer_expr(end);
                self.constrain_assignable(start_ty, self.interner.int, *span, "range start");
                self.constrain_assignable(end_ty, self.interner.int, *span, "range end");
                self.interner.make_array(self.interner.int)
            }

            // ── Pipe ────────────────────────────────────────────
            Expr::Pipe { left, right, span } => {
                let left_ty = self.infer_expr(left);
                let right_ty = self.infer_expr(right);
                let right_data = self.interner.get(right_ty).clone();

                match right_data {
                    TypeData::Function(sig) => {
                        if let Some(first_param) = sig.params.first() {
                            self.constrain_assignable(
                                left_ty,
                                first_param.ty,
                                *span,
                                "pipe operator input",
                            );
                        }
                        sig.ret
                    }
                    TypeData::TypeVar(_) => self.interner.fresh_type_var(),
                    _ => {
                        self.error_type_mismatch(
                            self.interner.void,
                            right_ty,
                            *span,
                            "right side of `|>` must be a function",
                        );
                        self.interner.never
                    }
                }
            }

            // ── Await ───────────────────────────────────────────
            Expr::Await { expr, .. } => {
                // For now, await just returns the inner type
                // (full async/Promise support is a later phase)
                self.infer_expr(expr)
            }

            // ── Assign ──────────────────────────────────────────
            Expr::Assign {
                target,
                value,
                span,
                ..
            } => {
                let target_ty = self.infer_expr(target);
                let value_ty = self.infer_expr(value);

                // Check mutability — direct identifier
                if let Expr::Ident {
                    name,
                    span: id_span,
                } = target.as_ref()
                {
                    if let Some(binding) = self.env.lookup(name) {
                        if !binding.is_mutable() {
                            self.emit_error(crate::types::constraint::TypeError {
                                expected: self.interner.void,
                                actual: self.interner.void,
                                span: *id_span,
                                context: format!(
                                    "cannot assign to '{}' because it is not mutable",
                                    name
                                ),
                                kind: TypeErrorKind::TypeMismatch,
                            });
                        }
                    }
                }

                // Check member-access mutations: store signals + frozen values
                self.check_member_assign_restrictions(target, *span);

                // When assigning to a signal, unwrap to the inner type —
                // `signal.set(value)` semantics: the value must match the
                // signal's inner type, not the Signal wrapper itself.
                let effective_target_ty = match self.interner.get(target_ty).clone() {
                    TypeData::Signal(inner) => inner,
                    _ => target_ty,
                };
                self.constrain_assignable(value_ty, effective_target_ty, *span, "assignment");
                self.interner.void
            }

            // ── Assert ──────────────────────────────────────────
            Expr::Assert { expr, span } => {
                let ty = self.infer_expr(expr);
                self.constrain_assignable(ty, self.interner.bool_, *span, "assert expression");
                self.interner.void
            }
        }
    }

    /// Resolve a member (field) access on a type.
    fn resolve_member(
        &mut self,
        obj_ty: TypeId,
        field: &str,
        span: crate::span::Span,
        _optional: bool,
    ) -> TypeId {
        let obj_data = self.interner.get(obj_ty).clone();

        match obj_data {
            TypeData::Object(fields) => {
                if let Some(f) = fields.iter().find(|f| f.name == field) {
                    f.ty
                } else {
                    self.error_type_mismatch(
                        self.interner.void,
                        obj_ty,
                        span,
                        &format!("property '{}' does not exist on this object", field),
                    );
                    self.interner.never
                }
            }
            TypeData::Guard(g) => {
                if let Some(f) = g.fields.iter().find(|f| f.name == field) {
                    f.ty
                } else {
                    self.error_type_mismatch(
                        self.interner.void,
                        obj_ty,
                        span,
                        &format!("field '{}' does not exist on guard '{}'", field, g.name),
                    );
                    self.interner.never
                }
            }
            TypeData::Store(s) => {
                if let Some(sig) = s.signals.iter().find(|x| x.name == field) {
                    sig.ty
                } else if let Some(d) = s.derives.iter().find(|x| x.name == field) {
                    d.ty
                } else if let Some(m) = s.methods.iter().find(|x| x.name == field) {
                    self.interner.make_function(m.sig.clone())
                } else {
                    self.error_type_mismatch(
                        self.interner.void,
                        obj_ty,
                        span,
                        &format!("member '{}' does not exist on store '{}'", field, s.name),
                    );
                    self.interner.never
                }
            }
            TypeData::Component(c) => {
                if let Some(p) = c.props.iter().find(|p| p.name == field) {
                    p.ty
                } else {
                    self.error_type_mismatch(
                        self.interner.void,
                        obj_ty,
                        span,
                        &format!("prop '{}' does not exist on component '{}'", field, c.name),
                    );
                    self.interner.never
                }
            }
            // Array built-in: .length
            TypeData::Array(_) if field == "length" => self.interner.int,
            // String built-in: .length
            TypeData::String | TypeData::StringLiteral(_) if field == "length" => self.interner.int,
            TypeData::TypeVar(_) => self.interner.fresh_type_var(),
            TypeData::Signal(inner) => {
                // Auto-unwrap signal for member access
                self.resolve_member(inner, field, span, _optional)
            }
            TypeData::Derived(inner) => {
                // Auto-unwrap derived for member access
                self.resolve_member(inner, field, span, _optional)
            }
            _ => {
                self.error_type_mismatch(
                    self.interner.void,
                    obj_ty,
                    span,
                    &format!("cannot access property '{}' on this type", field),
                );
                self.interner.never
            }
        }
    }

    /// Try to handle guard composition methods: `.partial()`, `.pick(...)`, `.omit(...)`.
    ///
    /// Returns `Some(TypeId)` if this was a guard composition call, `None` otherwise
    /// (to fall through to normal function call handling).
    fn try_guard_composition(
        &mut self,
        callee: &Expr,
        args: &[Expr],
        span: crate::span::Span,
    ) -> Option<TypeId> {
        // Must be a member access: SomeGuard.partial(), SomeGuard.pick("f1"), etc.
        let (object, method) = match callee {
            Expr::MemberAccess { object, field, .. } => (object, field.as_str()),
            _ => return None,
        };

        // Check if the method is a guard composition method
        if !matches!(method, "partial" | "pick" | "omit") {
            return None;
        }

        // Infer the object's type and check it's a Guard
        let obj_ty = self.infer_expr(object);
        let obj_data = self.interner.get(obj_ty).clone();
        let guard = match obj_data {
            TypeData::Guard(g) => g,
            _ => return None, // Not a guard — fall through to normal handling
        };

        match method {
            "partial" => {
                // .partial() — all fields become optional (add Optional validation)
                let fields: Vec<GuardField> = guard
                    .fields
                    .iter()
                    .map(|f| {
                        let mut validations = f.validations.clone();
                        if !validations
                            .iter()
                            .any(|v| matches!(v, crate::types::validation::Validation::Optional))
                        {
                            validations.push(crate::types::validation::Validation::Optional);
                        }
                        GuardField {
                            name: f.name.clone(),
                            ty: self.interner.make_optional(f.ty),
                            validations,
                        }
                    })
                    .collect();
                let new_guard = self.interner.make_guard(GuardDef {
                    name: smol_str::SmolStr::new(format!("Partial<{}>", guard.name)),
                    fields,
                    extends: Some(guard.name.clone()),
                    has_validators: guard.has_validators,
                });
                Some(new_guard)
            }
            "pick" => {
                // .pick("field1", "field2") — keep only named fields
                let pick_names: Vec<&str> = args
                    .iter()
                    .filter_map(|a| match a {
                        Expr::StringLit { value, .. } => Some(value.as_str()),
                        _ => None,
                    })
                    .collect();

                if pick_names.is_empty() {
                    self.emit_error(crate::types::constraint::TypeError {
                        expected: self.interner.void,
                        actual: obj_ty,
                        span,
                        context: "`.pick()` requires string field name arguments".into(),
                        kind: crate::types::constraint::TypeErrorKind::TypeMismatch,
                    });
                    return Some(obj_ty);
                }

                let fields: Vec<GuardField> = guard
                    .fields
                    .iter()
                    .filter(|f| pick_names.contains(&f.name.as_str()))
                    .cloned()
                    .collect();

                // Warn about unknown field names
                for name in &pick_names {
                    if !guard.fields.iter().any(|f| f.name == *name) {
                        self.emit_error(crate::types::constraint::TypeError {
                            expected: self.interner.void,
                            actual: obj_ty,
                            span,
                            context: format!(
                                "field '{}' does not exist on guard '{}'",
                                name, guard.name
                            ),
                            kind: crate::types::constraint::TypeErrorKind::TypeMismatch,
                        });
                    }
                }

                let new_guard = self.interner.make_guard(GuardDef {
                    name: smol_str::SmolStr::new(format!("Pick<{}>", guard.name)),
                    fields,
                    extends: Some(guard.name.clone()),
                    has_validators: guard.has_validators,
                });
                Some(new_guard)
            }
            "omit" => {
                // .omit("field1") — remove named fields
                let omit_names: Vec<&str> = args
                    .iter()
                    .filter_map(|a| match a {
                        Expr::StringLit { value, .. } => Some(value.as_str()),
                        _ => None,
                    })
                    .collect();

                if omit_names.is_empty() {
                    self.emit_error(crate::types::constraint::TypeError {
                        expected: self.interner.void,
                        actual: obj_ty,
                        span,
                        context: "`.omit()` requires string field name arguments".into(),
                        kind: crate::types::constraint::TypeErrorKind::TypeMismatch,
                    });
                    return Some(obj_ty);
                }

                let fields: Vec<GuardField> = guard
                    .fields
                    .iter()
                    .filter(|f| !omit_names.contains(&f.name.as_str()))
                    .cloned()
                    .collect();

                let new_guard = self.interner.make_guard(GuardDef {
                    name: smol_str::SmolStr::new(format!("Omit<{}>", guard.name)),
                    fields,
                    extends: Some(guard.name.clone()),
                    has_validators: guard.has_validators,
                });
                Some(new_guard)
            }
            _ => None,
        }
    }

    // ── Assignment restriction checks ──────────────────────────────

    /// Check restrictions on member-access assignment targets.
    ///
    /// Enforces:
    /// 1. Store signals can only be mutated from within the store's own methods.
    /// 2. Frozen values cannot have their members mutated (deep immutability).
    fn check_member_assign_restrictions(&mut self, target: &Expr, span: crate::span::Span) {
        // Walk to find root + check store member assignments
        if let Expr::MemberAccess {
            object,
            field,
            span: member_span,
        } = target
        {
            // Check if assigning to a store member signal
            let obj_ty = self.infer_expr(object);
            let obj_data = self.interner.get(obj_ty).clone();
            if let TypeData::Store(ref store) = obj_data {
                // Check if the field is a signal in this store
                if store.signals.iter().any(|s| s.name == *field) {
                    // Store signals can only be mutated inside the store's own methods
                    let allowed = self
                        .current_store_method
                        .as_ref()
                        .is_some_and(|name| *name == store.name);
                    if !allowed {
                        self.emit_error(crate::types::constraint::TypeError {
                            expected: self.interner.void,
                            actual: obj_ty,
                            span: *member_span,
                            context: format!(
                                "cannot mutate signal '{}.{}' outside of store methods — \
                                 use a store method to modify state",
                                store.name, field
                            ),
                            kind: TypeErrorKind::TypeMismatch,
                        });
                    }
                }
            }

            // Check if the root of the member chain is frozen (deep immutability)
            if let Some((root_name, root_binding_kind)) = self.find_root_binding(target) {
                if root_binding_kind == crate::types::env::BindingKind::Frozen {
                    self.emit_error(crate::types::constraint::TypeError {
                        expected: self.interner.void,
                        actual: self.interner.void,
                        span,
                        context: format!(
                            "cannot mutate property of frozen value '{}' — \
                             frozen values are deeply immutable",
                            root_name
                        ),
                        kind: TypeErrorKind::TypeMismatch,
                    });
                }
            }
        }
    }

    /// Walk a member-access chain to find the root identifier's binding kind.
    ///
    /// For `a.b.c`, returns `("a", binding_kind_of_a)`.
    fn find_root_binding(
        &self,
        expr: &Expr,
    ) -> Option<(smol_str::SmolStr, crate::types::env::BindingKind)> {
        match expr {
            Expr::Ident { name, .. } => self.env.lookup(name).map(|b| (name.clone(), b.kind)),
            Expr::MemberAccess { object, .. } => self.find_root_binding(object),
            _ => None,
        }
    }
}
