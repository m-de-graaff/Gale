//! Scoped type environment — maps identifiers to types with lexical scoping.
//!
//! [`TypeEnv`] maintains a stack of scopes. Lookups walk the stack from
//! innermost to outermost, implementing lexical scoping with shadowing.

use smol_str::SmolStr;
use std::collections::HashMap;

use super::ty::TypeId;
use crate::span::Span;

// ── Binding kinds ──────────────────────────────────────────────────────

/// How a name was introduced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingKind {
    /// `let x = ...`
    Let,
    /// `mut x = ...`
    Mut,
    /// `signal x = ...`
    Signal,
    /// `derive x = ...`
    Derived,
    /// `frozen x = ...`
    Frozen,
    /// Function or arrow function parameter.
    Parameter,
    /// `fn name(...)`
    Function,
    /// `guard Name { ... }`
    Guard,
    /// `store Name { ... }`
    Store,
    /// `channel Name(...)`
    Channel,
    /// `out ui Name(...)`
    Component,
    /// `action name(...)`
    Action,
    /// `query name = ...`
    Query,
    /// `for item in ...` loop variable.
    ForBinding,
    /// `type Name = ...`
    TypeAlias,
    /// `enum Name { ... }`
    EnumDef,
    /// `out api ResourceName { ... }`
    Api,
    /// `middleware name(...) { ... }`
    Middleware,
}

impl BindingKind {
    /// Whether this binding kind allows mutation.
    pub fn is_mutable(self) -> bool {
        matches!(self, BindingKind::Mut | BindingKind::Signal)
    }
}

// ── Boundary scopes ───────────────────────────────────────────────────

/// Which side of the server/client boundary a declaration belongs to.
///
/// Every binding is tagged with a `BoundaryScope` when defined. This
/// enables cross-boundary access checking: client code cannot reference
/// server bindings (except action stubs), and vice versa.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundaryScope {
    /// Server-only — `server { }` block or implicitly server (action, query, channel).
    Server,
    /// Client-only — `client { }` block or implicitly client (signal, derive, ref, component).
    Client,
    /// Shared — `shared { }` block or implicitly shared (guard, enum, type alias).
    Shared,
    /// No boundary — top-level declarations not inside any boundary block
    /// and not inherently scoped (fn, let, mut at top level).
    Unscoped,
}

impl std::fmt::Display for BoundaryScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BoundaryScope::Server => write!(f, "server"),
            BoundaryScope::Client => write!(f, "client"),
            BoundaryScope::Shared => write!(f, "shared"),
            BoundaryScope::Unscoped => write!(f, "unscoped"),
        }
    }
}

// ── Scope kinds ────────────────────────────────────────────────────────

/// The kind of scope — affects what declarations are valid inside it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeKind {
    /// Top-level / module scope.
    Global,
    /// Inside a `fn` or arrow function.
    Function,
    /// A `{ ... }` block (if, for, etc.).
    Block,
    /// Inside a `out ui Component(...) { ... }` body.
    ComponentBody,
    /// Inside a `guard { ... }` definition.
    GuardBody,
    /// Inside a `store { ... }` definition.
    StoreBody,
    /// Inside a `server { ... }` block.
    ServerBlock,
    /// Inside a `client { ... }` block.
    ClientBlock,
    /// Inside a `shared { ... }` block.
    SharedBlock,
    /// Inside a `for x in ... { ... }` loop.
    ForLoop,
    /// Inside a `test "name" { ... }` block.
    TestBlock,
}

// ── Binding ────────────────────────────────────────────────────────────

/// A binding entry in the environment — a name mapped to its type and metadata.
#[derive(Debug, Clone)]
pub struct Binding {
    /// The type of the binding.
    pub ty: TypeId,
    /// How the binding was introduced.
    pub kind: BindingKind,
    /// Source location where the binding was declared.
    pub span: Span,
    /// Which side of the server/client boundary this binding belongs to.
    pub boundary: BoundaryScope,
}

impl Binding {
    /// Whether this binding can be mutated.
    pub fn is_mutable(&self) -> bool {
        self.kind.is_mutable()
    }
}

// ── Scope ──────────────────────────────────────────────────────────────

/// A single lexical scope containing bindings.
#[derive(Debug)]
struct Scope {
    bindings: HashMap<SmolStr, Binding>,
    kind: ScopeKind,
}

impl Scope {
    fn new(kind: ScopeKind) -> Self {
        Self {
            bindings: HashMap::new(),
            kind,
        }
    }
}

// ── TypeEnv ────────────────────────────────────────────────────────────

/// Error when defining a binding.
#[derive(Debug, Clone)]
pub struct DefineError {
    pub name: SmolStr,
    pub existing_span: Span,
    pub new_span: Span,
}

impl std::fmt::Display for DefineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "identifier '{}' is already defined in this scope",
            self.name
        )
    }
}

/// Scoped type environment for identifier → type resolution.
///
/// Supports:
/// - Lexical scoping with a stack of scopes
/// - Shadowing (inner scope can redefine outer names)
/// - Named type registry (guards, enums, type aliases)
pub struct TypeEnv {
    scopes: Vec<Scope>,
    /// Named type registry — maps type names to their TypeId.
    /// Separate from bindings because types live in a different namespace.
    type_registry: HashMap<SmolStr, TypeId>,
    /// Types declared inside `shared { }` blocks — available on both server and client.
    shared_types: std::collections::HashSet<SmolStr>,
    /// Boundary context stack — tracks which boundary block (server/client/shared)
    /// we are currently inside. Separate from lexical scopes because boundary
    /// blocks are transparent (they tag declarations but don't hide them).
    boundary_stack: Vec<BoundaryScope>,
}

impl TypeEnv {
    /// Create a new environment with an empty global scope.
    pub fn new() -> Self {
        Self {
            scopes: vec![Scope::new(ScopeKind::Global)],
            type_registry: HashMap::new(),
            shared_types: std::collections::HashSet::new(),
            boundary_stack: Vec::new(),
        }
    }

    /// Push a new scope onto the stack.
    pub fn push_scope(&mut self, kind: ScopeKind) {
        self.scopes.push(Scope::new(kind));
    }

    /// Pop the innermost scope. Panics if only the global scope remains.
    pub fn pop_scope(&mut self) {
        assert!(self.scopes.len() > 1, "cannot pop the global scope");
        self.scopes.pop();
    }

    /// The kind of the current (innermost) scope.
    pub fn current_scope_kind(&self) -> ScopeKind {
        self.scopes.last().unwrap().kind
    }

    /// Current scope depth (1 = global only).
    pub fn depth(&self) -> usize {
        self.scopes.len()
    }

    /// Define a new binding in the current scope.
    ///
    /// Returns `Err` if the name is already defined in the **same** scope
    /// (shadowing across scopes is allowed).
    pub fn define(&mut self, name: SmolStr, binding: Binding) -> Result<(), DefineError> {
        let scope = self.scopes.last_mut().unwrap();
        if let Some(existing) = scope.bindings.get(&name) {
            return Err(DefineError {
                name,
                existing_span: existing.span,
                new_span: binding.span,
            });
        }
        scope.bindings.insert(name, binding);
        Ok(())
    }

    /// Look up a binding by name, searching from innermost to outermost scope.
    ///
    /// Returns `None` if the name is not defined in any visible scope.
    pub fn lookup(&self, name: &str) -> Option<&Binding> {
        for scope in self.scopes.iter().rev() {
            if let Some(binding) = scope.bindings.get(name) {
                return Some(binding);
            }
        }
        None
    }

    /// Register a named type (guard, enum, type alias, etc.).
    pub fn register_type(&mut self, name: SmolStr, ty: TypeId) {
        self.type_registry.insert(name, ty);
    }

    /// Resolve a named type.
    pub fn resolve_type(&self, name: &str) -> Option<TypeId> {
        self.type_registry.get(name).copied()
    }

    /// Check if we're inside a scope of the given kind (anywhere in the stack).
    pub fn is_inside(&self, kind: ScopeKind) -> bool {
        self.scopes.iter().any(|s| s.kind == kind)
    }

    /// Return all bindings visible in the current scope, from inner to outer.
    ///
    /// Used by the LSP for autocomplete — enumerates every reachable binding.
    /// If the same name appears in multiple scopes, only the innermost is returned.
    pub fn all_visible_bindings(&self) -> Vec<(SmolStr, Binding)> {
        let mut result = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for scope in self.scopes.iter().rev() {
            for (name, binding) in &scope.bindings {
                if seen.insert(name.clone()) {
                    result.push((name.clone(), binding.clone()));
                }
            }
        }
        result
    }

    /// Return all registered type names (guards, enums, type aliases).
    ///
    /// Used by the LSP for type-position autocomplete.
    pub fn all_type_names(&self) -> Vec<SmolStr> {
        self.type_registry.keys().cloned().collect()
    }

    /// Push a boundary context (server/client/shared).
    ///
    /// Boundary contexts are separate from lexical scopes — they tag
    /// declarations without hiding them. Bindings defined inside a
    /// boundary block remain visible at the module level.
    pub fn push_boundary(&mut self, boundary: BoundaryScope) {
        self.boundary_stack.push(boundary);
    }

    /// Pop the innermost boundary context.
    pub fn pop_boundary(&mut self) {
        self.boundary_stack.pop();
    }

    /// The current boundary scope.
    ///
    /// Returns the innermost active boundary context, or
    /// [`BoundaryScope::Unscoped`] if not inside any boundary block.
    pub fn current_boundary_scope(&self) -> BoundaryScope {
        self.boundary_stack
            .last()
            .copied()
            .unwrap_or(BoundaryScope::Unscoped)
    }

    /// Register a type as shared (available in both server and client scopes).
    pub fn register_shared_type(&mut self, name: SmolStr) {
        self.shared_types.insert(name);
    }

    /// Check if a type is declared as shared.
    pub fn is_shared_type(&self, name: &str) -> bool {
        self.shared_types.contains(name)
    }
}

impl Default for TypeEnv {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::span::Span;
    use crate::types::ty::TypeId;

    fn dummy_binding(ty: TypeId, kind: BindingKind) -> Binding {
        Binding {
            ty,
            kind,
            span: Span::dummy(),
            boundary: BoundaryScope::Unscoped,
        }
    }

    #[test]
    fn new_env_has_global_scope() {
        let env = TypeEnv::new();
        assert_eq!(env.depth(), 1);
        assert_eq!(env.current_scope_kind(), ScopeKind::Global);
    }

    #[test]
    fn define_and_lookup() {
        let mut env = TypeEnv::new();
        let ty = TypeId::from_raw(0);
        env.define("x".into(), dummy_binding(ty, BindingKind::Let))
            .unwrap();
        let b = env.lookup("x").unwrap();
        assert_eq!(b.ty, ty);
        assert_eq!(b.kind, BindingKind::Let);
    }

    #[test]
    fn lookup_missing_returns_none() {
        let env = TypeEnv::new();
        assert!(env.lookup("missing").is_none());
    }

    #[test]
    fn shadowing_across_scopes() {
        let mut env = TypeEnv::new();
        let outer_ty = TypeId::from_raw(0);
        let inner_ty = TypeId::from_raw(1);

        env.define("x".into(), dummy_binding(outer_ty, BindingKind::Let))
            .unwrap();
        assert_eq!(env.lookup("x").unwrap().ty, outer_ty);

        env.push_scope(ScopeKind::Block);
        env.define("x".into(), dummy_binding(inner_ty, BindingKind::Mut))
            .unwrap();
        assert_eq!(env.lookup("x").unwrap().ty, inner_ty);

        env.pop_scope();
        assert_eq!(env.lookup("x").unwrap().ty, outer_ty);
    }

    #[test]
    fn same_scope_redefine_errors() {
        let mut env = TypeEnv::new();
        let ty = TypeId::from_raw(0);
        env.define("x".into(), dummy_binding(ty, BindingKind::Let))
            .unwrap();
        let result = env.define("x".into(), dummy_binding(ty, BindingKind::Let));
        assert!(result.is_err());
    }

    #[test]
    fn inner_scope_sees_outer_bindings() {
        let mut env = TypeEnv::new();
        let ty = TypeId::from_raw(0);
        env.define("outer".into(), dummy_binding(ty, BindingKind::Let))
            .unwrap();

        env.push_scope(ScopeKind::Function);
        assert!(env.lookup("outer").is_some());
        env.pop_scope();
    }

    #[test]
    fn outer_scope_does_not_see_inner() {
        let mut env = TypeEnv::new();
        env.push_scope(ScopeKind::Block);
        env.define(
            "inner".into(),
            dummy_binding(TypeId::from_raw(0), BindingKind::Let),
        )
        .unwrap();
        env.pop_scope();

        assert!(env.lookup("inner").is_none());
    }

    #[test]
    fn type_registry() {
        let mut env = TypeEnv::new();
        let ty = TypeId::from_raw(42);
        env.register_type("User".into(), ty);
        assert_eq!(env.resolve_type("User"), Some(ty));
        assert_eq!(env.resolve_type("Missing"), None);
    }

    #[test]
    fn is_inside_scope() {
        let mut env = TypeEnv::new();
        assert!(env.is_inside(ScopeKind::Global));
        assert!(!env.is_inside(ScopeKind::ServerBlock));

        env.push_scope(ScopeKind::ServerBlock);
        env.push_scope(ScopeKind::Function);
        assert!(env.is_inside(ScopeKind::ServerBlock));
        assert!(env.is_inside(ScopeKind::Function));
    }

    #[test]
    fn current_boundary_scope_unscoped_by_default() {
        let env = TypeEnv::new();
        assert_eq!(env.current_boundary_scope(), BoundaryScope::Unscoped);
    }

    #[test]
    fn current_boundary_scope_server() {
        let mut env = TypeEnv::new();
        env.push_boundary(BoundaryScope::Server);
        assert_eq!(env.current_boundary_scope(), BoundaryScope::Server);
        // Nested function inside server boundary still reports Server
        env.push_scope(ScopeKind::Function);
        assert_eq!(env.current_boundary_scope(), BoundaryScope::Server);
    }

    #[test]
    fn current_boundary_scope_client() {
        let mut env = TypeEnv::new();
        env.push_boundary(BoundaryScope::Client);
        assert_eq!(env.current_boundary_scope(), BoundaryScope::Client);
    }

    #[test]
    fn current_boundary_scope_shared() {
        let mut env = TypeEnv::new();
        env.push_boundary(BoundaryScope::Shared);
        assert_eq!(env.current_boundary_scope(), BoundaryScope::Shared);
    }

    #[test]
    fn current_boundary_scope_pops_correctly() {
        let mut env = TypeEnv::new();
        env.push_boundary(BoundaryScope::Server);
        assert_eq!(env.current_boundary_scope(), BoundaryScope::Server);
        env.pop_boundary();
        assert_eq!(env.current_boundary_scope(), BoundaryScope::Unscoped);
    }

    #[test]
    fn binding_mutability() {
        assert!(BindingKind::Mut.is_mutable());
        assert!(BindingKind::Signal.is_mutable());
        assert!(!BindingKind::Let.is_mutable());
        assert!(!BindingKind::Frozen.is_mutable());
        assert!(!BindingKind::Parameter.is_mutable());
    }

    #[test]
    #[should_panic(expected = "cannot pop the global scope")]
    fn pop_global_scope_panics() {
        let mut env = TypeEnv::new();
        env.pop_scope();
    }
}
