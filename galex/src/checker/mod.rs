//! GaleX type checker — walks the AST, infers types, checks correctness.
//!
//! # Architecture
//!
//! The checker operates in two phases:
//!
//! 1. **Walk & Collect** — traverse the AST top-down, assigning type variables
//!    for unknowns, resolving type annotations, inferring expression types,
//!    and collecting type constraints.
//!
//! 2. **Solve & Report** — feed all constraints to the [`ConstraintSolver`],
//!    solve via unification, apply deep substitution, and report type errors.

mod boundary;
mod decl;
pub(crate) mod dom;
mod expr;
mod ops;
mod stmt;
mod template;

use std::collections::HashMap;

use smol_str::SmolStr;

use crate::ast::{Program, TypeAnnotation};
use crate::span::Span;
use crate::types::constraint::{
    Constraint, ConstraintKind, ConstraintSolver, TypeError, TypeErrorKind,
};
use crate::types::env::{Binding, BindingKind, TypeEnv};
use crate::types::ty::*;

/// The GaleX type checker.
///
/// Owns the [`TypeInterner`] and [`TypeEnv`] to avoid borrow conflicts
/// during constraint collection. After checking, the interner and env
/// can be extracted for later compiler phases.
pub struct TypeChecker {
    /// The central type store.
    pub interner: TypeInterner,
    /// The scoped type environment.
    pub env: TypeEnv,
    /// Collected constraints (solved at the end).
    constraints: Vec<Constraint>,
    /// Accumulated type errors.
    errors: Vec<TypeError>,
    /// The expected return type of the current function (if inside one).
    current_return_type: Option<TypeId>,
    /// The name of the store whose method we're currently inside (if any).
    /// Used to enforce that store signals can only be mutated from within
    /// the store's own methods.
    current_store_method: Option<SmolStr>,
    /// Declared env var types — populated by `check_env_decl()`, used by
    /// `Expr::EnvAccess` to return the declared type instead of `string`.
    declared_env_types: HashMap<SmolStr, TypeId>,
}

impl TypeChecker {
    /// Create a new type checker with built-in primitive types registered.
    pub fn new() -> Self {
        let mut interner = TypeInterner::new();
        let mut env = TypeEnv::new();

        // Register built-in type names
        env.register_type("string".into(), interner.string);
        env.register_type("int".into(), interner.int);
        env.register_type("float".into(), interner.float);
        env.register_type("bool".into(), interner.bool_);
        env.register_type("void".into(), interner.void);
        env.register_type("null".into(), interner.null);
        env.register_type("never".into(), interner.never);

        // Register DOM event types as named types (for event handler signature checking)
        let dom_event_types = [
            "Event",
            "MouseEvent",
            "KeyboardEvent",
            "InputEvent",
            "FocusEvent",
            "SubmitEvent",
            "DragEvent",
            "TouchEvent",
            "PointerEvent",
            "WheelEvent",
            "AnimationEvent",
            "TransitionEvent",
            "ClipboardEvent",
        ];
        for name in dom_event_types {
            let ty = interner.make_named(name);
            env.register_type(name.into(), ty);
        }

        // Register DOM element types (for ref: directive checking)
        let dom_element_types = [
            "HTMLElement",
            "HTMLAnchorElement",
            "HTMLAudioElement",
            "HTMLButtonElement",
            "HTMLCanvasElement",
            "HTMLDetailsElement",
            "HTMLDialogElement",
            "HTMLFormElement",
            "HTMLIFrameElement",
            "HTMLImageElement",
            "HTMLInputElement",
            "HTMLLabelElement",
            "HTMLOptionElement",
            "HTMLSelectElement",
            "HTMLTableElement",
            "HTMLTextAreaElement",
            "HTMLVideoElement",
        ];
        for name in dom_element_types {
            let ty = interner.make_named(name);
            env.register_type(name.into(), ty);
        }

        Self {
            interner,
            env,
            constraints: Vec::new(),
            errors: Vec::new(),
            current_return_type: None,
            current_store_method: None,
            declared_env_types: HashMap::new(),
        }
    }

    /// Type-check an entire program. Returns all type errors found.
    pub fn check_program(&mut self, program: &Program) -> Vec<TypeError> {
        // Phase 1: Walk the AST and collect constraints
        for item in &program.items {
            self.check_item(item);
        }

        // Phase 2: Solve constraints
        let solve_errors = self.solve_constraints();
        self.errors.extend(solve_errors);

        std::mem::take(&mut self.errors)
    }

    // ── Type annotation resolution ─────────────────────────────────

    /// Resolve a source-level [`TypeAnnotation`] to an interned [`TypeId`].
    pub fn resolve_annotation(&mut self, ann: &TypeAnnotation) -> TypeId {
        match ann {
            TypeAnnotation::Named { name, span } => self.resolve_named_type(name, *span),
            TypeAnnotation::Array { element, .. } => {
                let elem = self.resolve_annotation(element);
                self.interner.make_array(elem)
            }
            TypeAnnotation::Union { types, .. } => {
                let members: Vec<_> = types.iter().map(|t| self.resolve_annotation(t)).collect();
                self.interner.make_union(members)
            }
            TypeAnnotation::Optional { inner, .. } => {
                let inner_ty = self.resolve_annotation(inner);
                self.interner.make_optional(inner_ty)
            }
            TypeAnnotation::StringLiteral { value, .. } => self.interner.make_string_literal(value),
            TypeAnnotation::Function { params, ret, .. } => {
                let param_types: Vec<FnParam> = params
                    .iter()
                    .enumerate()
                    .map(|(i, p)| FnParam {
                        name: SmolStr::new(format!("_{}", i)),
                        ty: self.resolve_annotation(p),
                        has_default: false,
                    })
                    .collect();
                let ret_ty = self.resolve_annotation(ret);
                self.interner.make_function(FunctionSig {
                    params: param_types,
                    ret: ret_ty,
                    is_async: false,
                })
            }
            TypeAnnotation::Tuple { elements, .. } => {
                let elem_types: Vec<_> = elements
                    .iter()
                    .map(|e| self.resolve_annotation(e))
                    .collect();
                self.interner.intern(TypeData::Tuple(elem_types))
            }
            TypeAnnotation::Object { fields, .. } => {
                let obj_fields: Vec<_> = fields
                    .iter()
                    .map(|f| ObjectField {
                        name: f.name.clone(),
                        ty: self.resolve_annotation(&f.ty),
                        optional: f.optional,
                    })
                    .collect();
                self.interner.intern(TypeData::Object(obj_fields))
            }
        }
    }

    /// Resolve a named type from the type registry or emit an error.
    fn resolve_named_type(&mut self, name: &str, span: Span) -> TypeId {
        if let Some(ty) = self.env.resolve_type(name) {
            ty
        } else {
            self.emit_error(TypeError {
                expected: self.interner.void, // placeholder
                actual: self.interner.void,
                span,
                context: format!("undefined type '{}'", name),
                kind: TypeErrorKind::TypeMismatch,
            });
            // Return a fresh type var so checking can continue
            self.interner.fresh_type_var()
        }
    }

    // ── Constraint helpers ─────────────────────────────────────────

    /// Add an equality constraint: `left = right`.
    fn constrain_equal(&mut self, left: TypeId, right: TypeId, span: Span, context: &str) {
        self.constraints.push(Constraint {
            left,
            right,
            kind: ConstraintKind::Equal,
            span,
            context: context.into(),
        });
    }

    /// Add an assignability constraint: `actual <: expected`.
    fn constrain_assignable(
        &mut self,
        actual: TypeId,
        expected: TypeId,
        span: Span,
        context: &str,
    ) {
        self.constraints.push(Constraint {
            left: actual,
            right: expected,
            kind: ConstraintKind::Assignable,
            span,
            context: context.into(),
        });
    }

    // ── Solve phase ────────────────────────────────────────────────

    /// Create a constraint solver, feed all collected constraints, and solve.
    fn solve_constraints(&mut self) -> Vec<TypeError> {
        let constraints = std::mem::take(&mut self.constraints);
        let mut solver = ConstraintSolver::new(&mut self.interner);
        for c in constraints {
            solver.add_constraint(c);
        }
        solver.solve()
    }

    // ── Error helpers ──────────────────────────────────────────────

    /// Record a type error.
    fn emit_error(&mut self, error: TypeError) {
        self.errors.push(error);
    }

    /// Emit a type mismatch error.
    fn error_type_mismatch(&mut self, expected: TypeId, actual: TypeId, span: Span, context: &str) {
        self.emit_error(TypeError {
            expected,
            actual,
            span,
            context: context.into(),
            kind: TypeErrorKind::TypeMismatch,
        });
    }

    // ── Binding helpers ────────────────────────────────────────────

    /// Define a binding in the current scope, emitting an error on redefinition.
    ///
    /// Automatically computes the [`BoundaryScope`](crate::types::env::BoundaryScope)
    /// for the binding based on:
    /// 1. The binding kind's implicit scope (e.g., `Action` → Server)
    /// 2. The enclosing boundary block (if no implicit scope)
    fn define_binding(&mut self, name: SmolStr, ty: TypeId, kind: BindingKind, span: Span) {
        let enclosing = self.env.current_boundary_scope();
        let boundary = self.effective_boundary(kind, enclosing);
        let binding = Binding {
            ty,
            kind,
            span,
            boundary,
        };
        if let Err(e) = self.env.define(name, binding) {
            self.emit_error(TypeError {
                expected: self.interner.void,
                actual: self.interner.void,
                span: e.new_span,
                context: format!("'{}' is already defined in this scope", e.name),
                kind: TypeErrorKind::TypeMismatch,
            });
        }
    }

    /// Check if a type is "renderable" in a template (string, int, float, bool).
    fn is_renderable(&self, ty: TypeId) -> bool {
        matches!(
            self.interner.get(ty),
            TypeData::String
                | TypeData::Int
                | TypeData::Float
                | TypeData::Bool
                | TypeData::StringLiteral(_)
                | TypeData::IntLiteral(_)
        )
    }

    // ── Reactive source checking ───────────────────────────────────

    /// Check if an expression references at least one reactive source
    /// (a signal or derived binding).
    ///
    /// Used by `watch` to ensure the watch target actually depends on
    /// reactive state — otherwise the watch body would never re-execute.
    fn references_reactive_source(&self, expr: &crate::ast::Expr) -> bool {
        use crate::ast::Expr;
        use crate::types::env::BindingKind;

        match expr {
            Expr::Ident { name, .. } => {
                if let Some(binding) = self.env.lookup(name) {
                    matches!(binding.kind, BindingKind::Signal | BindingKind::Derived)
                } else {
                    false
                }
            }
            Expr::MemberAccess { object, .. } | Expr::OptionalChain { object, .. } => {
                self.references_reactive_source(object)
            }
            Expr::BinaryOp { left, right, .. } => {
                self.references_reactive_source(left) || self.references_reactive_source(right)
            }
            Expr::UnaryOp { operand, .. } => self.references_reactive_source(operand),
            Expr::Ternary {
                condition,
                then_expr,
                else_expr,
                ..
            } => {
                self.references_reactive_source(condition)
                    || self.references_reactive_source(then_expr)
                    || self.references_reactive_source(else_expr)
            }
            Expr::FnCall { callee, args, .. } => {
                self.references_reactive_source(callee)
                    || args.iter().any(|a| self.references_reactive_source(a))
            }
            Expr::NullCoalesce { left, right, .. } => {
                self.references_reactive_source(left) || self.references_reactive_source(right)
            }
            Expr::Pipe { left, right, .. } => {
                self.references_reactive_source(left) || self.references_reactive_source(right)
            }
            Expr::IndexAccess { object, index, .. } => {
                self.references_reactive_source(object) || self.references_reactive_source(index)
            }
            Expr::TemplateLit { parts, .. } => parts.iter().any(|p| {
                if let crate::ast::TemplatePart::Expr(e) = p {
                    self.references_reactive_source(e)
                } else {
                    false
                }
            }),
            Expr::Await { expr, .. } | Expr::Spread { expr, .. } => {
                self.references_reactive_source(expr)
            }
            // Literals and other non-reactive expressions
            _ => false,
        }
    }
}

impl Default for TypeChecker {
    fn default() -> Self {
        Self::new()
    }
}
