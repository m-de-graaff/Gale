//! Type constraints and unification solver.
//!
//! The constraint solver implements Robinson's unification algorithm to resolve
//! type variables and verify type compatibility. Constraints are collected during
//! the type-checking AST walk, then solved as a batch.
//!
//! # Unification rules
//!
//! - `TypeVar(v) = T` → substitute `v := T` (with occurs check)
//! - `Primitive = same Primitive` → OK
//! - `Array(T) = Array(U)` → unify `T = U`
//! - `Function(P₁→R₁) = Function(P₂→R₂)` → unify params pairwise + return
//! - `Signal(T) = Signal(U)` → unify `T = U`
//! - `T <: Union(A|B|C)` → OK if `T <: any member`
//! - `Named(n)` → resolve first, then unify
//! - Otherwise → `TypeError`

use std::collections::HashMap;
use std::fmt;

use super::ty::{TypeData, TypeId, TypeInterner, TypeVarId};
use crate::span::Span;

// ── Constraint ─────────────────────────────────────────────────────────

/// The kind of relationship between two types in a constraint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintKind {
    /// Types must be structurally identical: `T = U`.
    Equal,
    /// Left must be assignable to right (subtype): `T <: U`.
    /// e.g., `"primary" <: string`, `int <: int | null`.
    Assignable,
}

/// A type constraint: an assertion that two types must be related.
#[derive(Debug, Clone)]
pub struct Constraint {
    /// The left-hand type.
    pub left: TypeId,
    /// The right-hand type.
    pub right: TypeId,
    /// What relationship is required.
    pub kind: ConstraintKind,
    /// Source location for error reporting.
    pub span: Span,
    /// Human-readable context (e.g., "in assignment to `x`").
    pub context: String,
}

// ── TypeError ──────────────────────────────────────────────────────────

/// A type error discovered during constraint solving.
#[derive(Debug, Clone)]
pub struct TypeError {
    /// The expected type.
    pub expected: TypeId,
    /// The actual type found.
    pub actual: TypeId,
    /// Source location.
    pub span: Span,
    /// Human-readable context.
    pub context: String,
    /// The specific kind of type error.
    pub kind: TypeErrorKind,
}

/// Specific kind of type error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeErrorKind {
    /// Two types that should be equal are not.
    TypeMismatch,
    /// A type is not assignable to the target type.
    NotAssignable,
    /// Occurs check failed — infinite type detected (e.g., `T = Array<T>`).
    OccursCheck,
    /// Function parameter count mismatch.
    ArityMismatch { expected: usize, actual: usize },
    /// Tuple length mismatch.
    TupleLengthMismatch { expected: usize, actual: usize },
    /// Boundary violation: accessing a binding from the wrong scope.
    ///
    /// e.g., client code referencing a server-scoped variable, or
    /// server code referencing a client-scoped signal.
    BoundaryViolation {
        binding_scope: String,
        reference_scope: String,
        suggestion: String,
    },
    /// Data crossing the server→client boundary is not serializable.
    ///
    /// Functions, signals, DOM refs, channels, and stores cannot be
    /// sent from server to client.
    NotSerializable,
    /// Invalid `out` export — scope mismatch or non-exportable declaration.
    InvalidExport { suggestion: String },
    /// Invalid `env()` access — private env vars in client scope.
    InvalidEnvAccess,
}

impl fmt::Display for TypeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            TypeErrorKind::TypeMismatch => {
                write!(f, "type mismatch: {}", self.context)
            }
            TypeErrorKind::NotAssignable => {
                write!(f, "type not assignable: {}", self.context)
            }
            TypeErrorKind::OccursCheck => {
                write!(f, "infinite type detected: {}", self.context)
            }
            TypeErrorKind::ArityMismatch { expected, actual } => {
                write!(
                    f,
                    "expected {} parameter(s), found {}: {}",
                    expected, actual, self.context
                )
            }
            TypeErrorKind::TupleLengthMismatch { expected, actual } => {
                write!(
                    f,
                    "expected tuple of {} element(s), found {}: {}",
                    expected, actual, self.context
                )
            }
            TypeErrorKind::BoundaryViolation {
                binding_scope,
                reference_scope,
                suggestion,
            } => {
                write!(
                    f,
                    "boundary violation: cannot reference {} binding from {} scope: {}. {}",
                    binding_scope, reference_scope, self.context, suggestion
                )
            }
            TypeErrorKind::NotSerializable => {
                write!(
                    f,
                    "not serializable: {} — data crossing the server/client boundary must be serializable",
                    self.context
                )
            }
            TypeErrorKind::InvalidExport { suggestion } => {
                write!(f, "invalid export: {}. {}", self.context, suggestion)
            }
            TypeErrorKind::InvalidEnvAccess => {
                write!(f, "invalid env access: {}", self.context)
            }
        }
    }
}

// ── ConstraintSolver ───────────────────────────────────────────────────

/// Solves type constraints via Robinson's unification algorithm.
///
/// # Usage
///
/// ```ignore
/// let mut solver = ConstraintSolver::new(&mut interner);
/// solver.add_constraint(Constraint { left, right, kind: Equal, span, context });
/// let errors = solver.solve();
/// let resolved_type = solver.resolve(some_type_id);
/// ```
pub struct ConstraintSolver<'a> {
    interner: &'a mut TypeInterner,
    constraints: Vec<Constraint>,
    /// Substitution map: type variable → resolved type.
    substitutions: HashMap<TypeVarId, TypeId>,
    /// Accumulated type errors.
    errors: Vec<TypeError>,
}

impl<'a> ConstraintSolver<'a> {
    /// Create a new solver backed by the given type interner.
    pub fn new(interner: &'a mut TypeInterner) -> Self {
        Self {
            interner,
            constraints: Vec::new(),
            substitutions: HashMap::new(),
            errors: Vec::new(),
        }
    }

    /// Add a constraint to be solved.
    pub fn add_constraint(&mut self, constraint: Constraint) {
        self.constraints.push(constraint);
    }

    /// Convenience: add an equality constraint.
    pub fn constrain_equal(
        &mut self,
        left: TypeId,
        right: TypeId,
        span: Span,
        context: impl Into<String>,
    ) {
        self.add_constraint(Constraint {
            left,
            right,
            kind: ConstraintKind::Equal,
            span,
            context: context.into(),
        });
    }

    /// Convenience: add an assignability constraint (left <: right).
    pub fn constrain_assignable(
        &mut self,
        actual: TypeId,
        expected: TypeId,
        span: Span,
        context: impl Into<String>,
    ) {
        self.add_constraint(Constraint {
            left: actual,
            right: expected,
            kind: ConstraintKind::Assignable,
            span,
            context: context.into(),
        });
    }

    /// Solve all accumulated constraints.
    /// Returns the list of type errors (empty if all constraints are satisfied).
    pub fn solve(&mut self) -> Vec<TypeError> {
        let constraints = std::mem::take(&mut self.constraints);
        for c in constraints {
            match c.kind {
                ConstraintKind::Equal => {
                    self.unify(c.left, c.right, c.span, &c.context);
                }
                ConstraintKind::Assignable => {
                    self.check_assignable(c.left, c.right, c.span, &c.context);
                }
            }
        }
        std::mem::take(&mut self.errors)
    }

    /// Apply all substitutions to resolve a type — follows the substitution
    /// chain until a non-variable type is found.
    pub fn resolve(&self, ty: TypeId) -> TypeId {
        match self.interner.get(ty) {
            TypeData::TypeVar(var_id) => {
                if let Some(&resolved) = self.substitutions.get(var_id) {
                    self.resolve(resolved)
                } else {
                    ty
                }
            }
            _ => ty,
        }
    }

    /// Get the current substitutions (for inspection/testing).
    pub fn substitutions(&self) -> &HashMap<TypeVarId, TypeId> {
        &self.substitutions
    }

    // ── Internal unification ───────────────────────────────────────

    /// Unify two types: assert they are equal, recording substitutions.
    fn unify(&mut self, a: TypeId, b: TypeId, span: Span, context: &str) {
        let a = self.resolve(a);
        let b = self.resolve(b);

        if a == b {
            return; // Already equal (or same TypeId after interning)
        }

        // Clone to avoid borrow conflict with self.interner
        let a_data = self.interner.get(a).clone();
        let b_data = self.interner.get(b).clone();

        match (&a_data, &b_data) {
            // TypeVar on either side → substitute
            (TypeData::TypeVar(var), _) => {
                if self.occurs_check(*var, b) {
                    self.errors.push(TypeError {
                        expected: b,
                        actual: a,
                        span,
                        context: context.into(),
                        kind: TypeErrorKind::OccursCheck,
                    });
                } else {
                    self.substitutions.insert(*var, b);
                }
            }
            (_, TypeData::TypeVar(var)) => {
                if self.occurs_check(*var, a) {
                    self.errors.push(TypeError {
                        expected: a,
                        actual: b,
                        span,
                        context: context.into(),
                        kind: TypeErrorKind::OccursCheck,
                    });
                } else {
                    self.substitutions.insert(*var, a);
                }
            }

            // Array(T) = Array(U) → unify T = U
            (TypeData::Array(elem_a), TypeData::Array(elem_b)) => {
                self.unify(*elem_a, *elem_b, span, context);
            }

            // Optional(T) = Optional(U) → unify T = U
            (TypeData::Optional(inner_a), TypeData::Optional(inner_b)) => {
                self.unify(*inner_a, *inner_b, span, context);
            }

            // Signal(T) = Signal(U) → unify T = U
            (TypeData::Signal(inner_a), TypeData::Signal(inner_b)) => {
                self.unify(*inner_a, *inner_b, span, context);
            }

            // Derived(T) = Derived(U) → unify T = U
            (TypeData::Derived(inner_a), TypeData::Derived(inner_b)) => {
                self.unify(*inner_a, *inner_b, span, context);
            }

            // DomRef(T) = DomRef(U) → unify T = U
            (TypeData::DomRef(inner_a), TypeData::DomRef(inner_b)) => {
                self.unify(*inner_a, *inner_b, span, context);
            }

            // Tuple(Ts) = Tuple(Us) → unify pairwise
            (TypeData::Tuple(elems_a), TypeData::Tuple(elems_b)) => {
                if elems_a.len() != elems_b.len() {
                    self.errors.push(TypeError {
                        expected: b,
                        actual: a,
                        span,
                        context: context.into(),
                        kind: TypeErrorKind::TupleLengthMismatch {
                            expected: elems_b.len(),
                            actual: elems_a.len(),
                        },
                    });
                } else {
                    for (ea, eb) in elems_a.iter().zip(elems_b.iter()) {
                        self.unify(*ea, *eb, span, context);
                    }
                }
            }

            // Function(P₁→R₁) = Function(P₂→R₂) → unify params + return
            (TypeData::Function(sig_a), TypeData::Function(sig_b)) => {
                if sig_a.params.len() != sig_b.params.len() {
                    self.errors.push(TypeError {
                        expected: b,
                        actual: a,
                        span,
                        context: context.into(),
                        kind: TypeErrorKind::ArityMismatch {
                            expected: sig_b.params.len(),
                            actual: sig_a.params.len(),
                        },
                    });
                } else {
                    for (pa, pb) in sig_a.params.iter().zip(sig_b.params.iter()) {
                        self.unify(pa.ty, pb.ty, span, context);
                    }
                    self.unify(sig_a.ret, sig_b.ret, span, context);
                }
            }

            // Query(T) = Query(U)
            (TypeData::Query { result: ra }, TypeData::Query { result: rb }) => {
                self.unify(*ra, *rb, span, context);
            }

            // Object structural equality — unify matching fields
            (TypeData::Object(fields_a), TypeData::Object(fields_b)) => {
                for fb in fields_b.iter() {
                    if let Some(fa) = fields_a.iter().find(|f| f.name == fb.name) {
                        self.unify(fa.ty, fb.ty, span, context);
                    } else if !fb.optional {
                        self.errors.push(TypeError {
                            expected: b,
                            actual: a,
                            span,
                            context: format!("{}: missing field '{}'", context, fb.name),
                            kind: TypeErrorKind::TypeMismatch,
                        });
                    }
                }
            }

            // Primitives and other structural types must match exactly
            _ => {
                if a_data != b_data {
                    self.errors.push(TypeError {
                        expected: b,
                        actual: a,
                        span,
                        context: context.into(),
                        kind: TypeErrorKind::TypeMismatch,
                    });
                }
            }
        }
    }

    /// Check if `actual` is assignable to `expected` (subtyping).
    fn check_assignable(&mut self, actual: TypeId, expected: TypeId, span: Span, context: &str) {
        let actual = self.resolve(actual);
        let expected = self.resolve(expected);

        if actual == expected {
            return;
        }

        let actual_data = self.interner.get(actual).clone();
        let expected_data = self.interner.get(expected).clone();

        match (&actual_data, &expected_data) {
            // TypeVar → substitute
            (TypeData::TypeVar(_), _) | (_, TypeData::TypeVar(_)) => {
                self.unify(actual, expected, span, context);
            }

            // T <: T | U | V → OK if T is assignable to any member
            (_, TypeData::Union(members)) => {
                let assignable = members.iter().any(|m| self.is_assignable_to(actual, *m));
                if !assignable {
                    self.errors.push(TypeError {
                        expected,
                        actual,
                        span,
                        context: context.into(),
                        kind: TypeErrorKind::NotAssignable,
                    });
                }
            }

            // T <: Optional(U) → OK if T <: U or T is null
            (_, TypeData::Optional(inner)) => {
                let null_ty = self.interner.null;
                if actual == null_ty || self.is_assignable_to(actual, *inner) {
                    // OK
                } else {
                    self.errors.push(TypeError {
                        expected,
                        actual,
                        span,
                        context: context.into(),
                        kind: TypeErrorKind::NotAssignable,
                    });
                }
            }

            // StringLiteral("x") <: String → OK
            (TypeData::StringLiteral(_), TypeData::String) => { /* OK */ }
            // IntLiteral(42) <: Int → OK
            (TypeData::IntLiteral(_), TypeData::Int) => { /* OK */ }
            // Int <: Float → OK (numeric widening)
            (TypeData::Int, TypeData::Float) => { /* OK */ }
            // IntLiteral <: Float → OK
            (TypeData::IntLiteral(_), TypeData::Float) => { /* OK */ }

            // Never <: anything → OK (bottom type)
            (TypeData::Never, _) => { /* OK */ }

            // Array(T) <: Array(U) if T <: U
            (TypeData::Array(elem_a), TypeData::Array(elem_b)) => {
                self.check_assignable(*elem_a, *elem_b, span, context);
            }

            // Object structural subtyping (width subtyping: extra fields OK)
            (TypeData::Object(fields_a), TypeData::Object(fields_b)) => {
                for fb in fields_b.iter() {
                    if let Some(fa) = fields_a.iter().find(|f| f.name == fb.name) {
                        self.check_assignable(fa.ty, fb.ty, span, context);
                    } else if !fb.optional {
                        self.errors.push(TypeError {
                            expected,
                            actual,
                            span,
                            context: format!("{}: missing field '{}'", context, fb.name),
                            kind: TypeErrorKind::NotAssignable,
                        });
                    }
                }
            }

            // Guard <: Object — guard fields structurally match object fields
            (TypeData::Guard(g), TypeData::Object(obj_fields)) => {
                for of in obj_fields.iter() {
                    if let Some(gf) = g.fields.iter().find(|f| f.name == of.name) {
                        self.check_assignable(gf.ty, of.ty, span, context);
                    } else if !of.optional {
                        self.errors.push(TypeError {
                            expected,
                            actual,
                            span,
                            context: format!(
                                "{}: guard '{}' missing field '{}'",
                                context, g.name, of.name
                            ),
                            kind: TypeErrorKind::NotAssignable,
                        });
                    }
                }
            }

            // Object <: Guard — object can satisfy guard shape
            (TypeData::Object(obj_fields), TypeData::Guard(g)) => {
                for gf in g.fields.iter() {
                    if let Some(of) = obj_fields.iter().find(|f| f.name == gf.name) {
                        self.check_assignable(of.ty, gf.ty, span, context);
                    } else if !gf
                        .validations
                        .iter()
                        .any(|v| matches!(v, crate::types::validation::Validation::Optional))
                    {
                        self.errors.push(TypeError {
                            expected,
                            actual,
                            span,
                            context: format!(
                                "{}: object missing required field '{}' of guard",
                                context, gf.name
                            ),
                            kind: TypeErrorKind::NotAssignable,
                        });
                    }
                }
            }

            // Otherwise, fall back to equality
            _ => {
                if actual_data != expected_data {
                    self.errors.push(TypeError {
                        expected,
                        actual,
                        span,
                        context: context.into(),
                        kind: TypeErrorKind::NotAssignable,
                    });
                }
            }
        }
    }

    /// Quick check: is `actual` assignable to `expected`?
    /// Does NOT record errors — used for union member checking.
    fn is_assignable_to(&self, actual: TypeId, expected: TypeId) -> bool {
        let actual = self.resolve(actual);
        let expected = self.resolve(expected);

        if actual == expected {
            return true;
        }

        let actual_data = self.interner.get(actual);
        let expected_data = self.interner.get(expected);

        match (actual_data, expected_data) {
            (TypeData::StringLiteral(_), TypeData::String) => true,
            (TypeData::IntLiteral(_), TypeData::Int) => true,
            (TypeData::Int, TypeData::Float) => true,
            (TypeData::IntLiteral(_), TypeData::Float) => true,
            (TypeData::Never, _) => true,
            (TypeData::Null, TypeData::Optional(_)) => true,
            (_, TypeData::Optional(inner)) => self.is_assignable_to(actual, *inner),
            (_, TypeData::Union(members)) => {
                members.iter().any(|m| self.is_assignable_to(actual, *m))
            }
            _ => actual_data == expected_data,
        }
    }

    /// Occurs check: does the type variable `var` appear anywhere inside `ty`?
    /// Prevents infinite types like `T = Array<T>`.
    fn occurs_check(&self, var: TypeVarId, ty: TypeId) -> bool {
        let ty = self.resolve(ty);
        match self.interner.get(ty) {
            TypeData::TypeVar(v) => *v == var,
            TypeData::Array(elem) => self.occurs_check(var, *elem),
            TypeData::Optional(inner) => self.occurs_check(var, *inner),
            TypeData::Signal(inner) => self.occurs_check(var, *inner),
            TypeData::Derived(inner) => self.occurs_check(var, *inner),
            TypeData::DomRef(inner) => self.occurs_check(var, *inner),
            TypeData::Tuple(elems) => elems.iter().any(|e| self.occurs_check(var, *e)),
            TypeData::Union(members) => members.iter().any(|m| self.occurs_check(var, *m)),
            TypeData::Function(sig) => {
                sig.params.iter().any(|p| self.occurs_check(var, p.ty))
                    || self.occurs_check(var, sig.ret)
            }
            TypeData::Query { result } => self.occurs_check(var, *result),
            TypeData::Guard(g) => g.fields.iter().any(|f| self.occurs_check(var, f.ty)),
            TypeData::Object(fields) => fields.iter().any(|f| self.occurs_check(var, f.ty)),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::ty::{FnParam, FunctionSig};
    use super::*;

    #[test]
    fn unify_identical_primitives() {
        let mut i = TypeInterner::new();
        let (int,) = (i.int,);
        let mut s = ConstraintSolver::new(&mut i);
        s.constrain_equal(int, int, Span::dummy(), "test");
        let errors = s.solve();
        assert!(errors.is_empty());
    }

    #[test]
    fn unify_different_primitives_fails() {
        let mut i = TypeInterner::new();
        let (int, string) = (i.int, i.string);
        let mut s = ConstraintSolver::new(&mut i);
        s.constrain_equal(int, string, Span::dummy(), "test");
        let errors = s.solve();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].kind, TypeErrorKind::TypeMismatch);
    }

    #[test]
    fn unify_type_var_substitutes() {
        let mut i = TypeInterner::new();
        let var = i.fresh_type_var();
        let int = i.int;
        let mut s = ConstraintSolver::new(&mut i);
        s.constrain_equal(var, int, Span::dummy(), "test");
        let errors = s.solve();
        assert!(errors.is_empty());
        assert_eq!(s.resolve(var), int);
    }

    #[test]
    fn unify_arrays() {
        let mut i = TypeInterner::new();
        let int = i.int;
        let arr_int = i.make_array(int);
        let var = i.fresh_type_var();
        let arr_var = i.make_array(var);
        let mut s = ConstraintSolver::new(&mut i);
        s.constrain_equal(arr_var, arr_int, Span::dummy(), "test");
        let errors = s.solve();
        assert!(errors.is_empty());
        assert_eq!(s.resolve(var), int);
    }

    #[test]
    fn unify_functions() {
        let mut i = TypeInterner::new();
        let (int, string) = (i.int, i.string);
        let sig1 = FunctionSig {
            params: vec![FnParam {
                name: "x".into(),
                ty: int,
                has_default: false,
            }],
            ret: string,
            is_async: false,
        };
        let sig2 = FunctionSig {
            params: vec![FnParam {
                name: "x".into(),
                ty: int,
                has_default: false,
            }],
            ret: string,
            is_async: false,
        };
        let f1 = i.make_function(sig1);
        let f2 = i.make_function(sig2);
        let mut s = ConstraintSolver::new(&mut i);
        s.constrain_equal(f1, f2, Span::dummy(), "test");
        let errors = s.solve();
        assert!(errors.is_empty());
    }

    #[test]
    fn unify_function_arity_mismatch() {
        let mut i = TypeInterner::new();
        let (int, void) = (i.int, i.void);
        let sig1 = FunctionSig {
            params: vec![FnParam {
                name: "x".into(),
                ty: int,
                has_default: false,
            }],
            ret: void,
            is_async: false,
        };
        let sig2 = FunctionSig {
            params: vec![],
            ret: void,
            is_async: false,
        };
        let f1 = i.make_function(sig1);
        let f2 = i.make_function(sig2);
        let mut s = ConstraintSolver::new(&mut i);
        s.constrain_equal(f1, f2, Span::dummy(), "test");
        let errors = s.solve();
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0].kind,
            TypeErrorKind::ArityMismatch { .. }
        ));
    }

    #[test]
    fn occurs_check_prevents_infinite_type() {
        let mut i = TypeInterner::new();
        let var = i.fresh_type_var();
        let arr_var = i.make_array(var);
        let mut s = ConstraintSolver::new(&mut i);
        s.constrain_equal(var, arr_var, Span::dummy(), "test");
        let errors = s.solve();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].kind, TypeErrorKind::OccursCheck);
    }

    #[test]
    fn assignable_string_literal_to_string() {
        let mut i = TypeInterner::new();
        let string = i.string;
        let lit = i.make_string_literal("primary");
        let mut s = ConstraintSolver::new(&mut i);
        s.constrain_assignable(lit, string, Span::dummy(), "test");
        let errors = s.solve();
        assert!(errors.is_empty());
    }

    #[test]
    fn assignable_to_union() {
        let mut i = TypeInterner::new();
        let int = i.int;
        let union = i.make_union(vec![i.string, int]);
        let mut s = ConstraintSolver::new(&mut i);
        s.constrain_assignable(int, union, Span::dummy(), "test");
        let errors = s.solve();
        assert!(errors.is_empty());
    }

    #[test]
    fn not_assignable_to_union() {
        let mut i = TypeInterner::new();
        let bool_ = i.bool_;
        let union = i.make_union(vec![i.string, i.int]);
        let mut s = ConstraintSolver::new(&mut i);
        s.constrain_assignable(bool_, union, Span::dummy(), "test");
        let errors = s.solve();
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].kind, TypeErrorKind::NotAssignable);
    }

    #[test]
    fn assignable_null_to_optional() {
        let mut i = TypeInterner::new();
        let null = i.null;
        let opt = i.make_optional(i.string);
        let mut s = ConstraintSolver::new(&mut i);
        s.constrain_assignable(null, opt, Span::dummy(), "test");
        let errors = s.solve();
        assert!(errors.is_empty());
    }

    #[test]
    fn assignable_never_to_anything() {
        let mut i = TypeInterner::new();
        let (never, string, int) = (i.never, i.string, i.int);
        let mut s = ConstraintSolver::new(&mut i);
        s.constrain_assignable(never, string, Span::dummy(), "test");
        s.constrain_assignable(never, int, Span::dummy(), "test");
        let errors = s.solve();
        assert!(errors.is_empty());
    }

    #[test]
    fn int_assignable_to_float() {
        let mut i = TypeInterner::new();
        let (int, float) = (i.int, i.float);
        let mut s = ConstraintSolver::new(&mut i);
        s.constrain_assignable(int, float, Span::dummy(), "test");
        let errors = s.solve();
        assert!(errors.is_empty());
    }

    #[test]
    fn chained_type_var_resolution() {
        let mut i = TypeInterner::new();
        let v1 = i.fresh_type_var();
        let v2 = i.fresh_type_var();
        let int = i.int;
        let mut s = ConstraintSolver::new(&mut i);
        s.constrain_equal(v1, v2, Span::dummy(), "test");
        s.constrain_equal(v2, int, Span::dummy(), "test");
        let errors = s.solve();
        assert!(errors.is_empty());
        assert_eq!(s.resolve(v1), int);
        assert_eq!(s.resolve(v2), int);
    }
}
