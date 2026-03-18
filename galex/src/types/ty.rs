//! Core type representation and interning.
//!
//! Types are stored in a central [`TypeInterner`] and referenced by cheap
//! [`TypeId`] handles. The interner uses hash-consing to guarantee that
//! structurally identical types share the same `TypeId`, making equality
//! checks O(1).
//!
//! # Usage
//!
//! ```
//! use galex::types::ty::TypeInterner;
//!
//! let mut interner = TypeInterner::new();
//! let int = interner.int;
//! let arr = interner.make_array(int);
//! assert_eq!(interner.get(arr), &galex::types::ty::TypeData::Array(int));
//! ```

use smol_str::SmolStr;
use std::collections::HashMap;

use super::validation::Validation;

// ── TypeId & TypeVarId ─────────────────────────────────────────────────

/// A cheap, `Copy` handle referencing a type stored in [`TypeInterner`].
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct TypeId(u32);

impl TypeId {
    /// The raw index (for debugging/serialization).
    pub fn raw(self) -> u32 {
        self.0
    }

    /// Create a TypeId from a raw index.
    ///
    /// Intended for testing and deserialization. Production code should
    /// obtain TypeIds from a [`TypeInterner`].
    pub fn from_raw(raw: u32) -> Self {
        Self(raw)
    }
}

impl std::fmt::Debug for TypeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TypeId({})", self.0)
    }
}

impl std::fmt::Display for TypeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "T{}", self.0)
    }
}

/// A unique identifier for an unresolved type variable (used during inference).
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub struct TypeVarId(u32);

impl TypeVarId {
    pub fn raw(self) -> u32 {
        self.0
    }
}

impl std::fmt::Debug for TypeVarId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TypeVar({})", self.0)
    }
}

// ── TypeData ───────────────────────────────────────────────────────────

/// Internal representation of a GaleX type.
///
/// Stored in [`TypeInterner`], referenced by [`TypeId`].
/// Implements `Hash` + `Eq` for hash-consing deduplication.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeData {
    // ── Primitives ──────────────────────────────────────────────────
    /// `string`
    String,
    /// `int`
    Int,
    /// `float`
    Float,
    /// `bool`
    Bool,
    /// The `void` type — functions that return nothing.
    Void,
    /// The `never` type — unreachable code / bottom type.
    Never,
    /// The `null` type — the type of the `null` literal.
    Null,

    // ── Literal types (for const narrowing / string unions) ────────
    /// A specific string value type, e.g. `"primary"`.
    StringLiteral(SmolStr),
    /// A specific integer value type, e.g. `42`.
    IntLiteral(i64),

    // ── Compound ────────────────────────────────────────────────────
    /// `T[]` — array of element type.
    Array(TypeId),
    /// `(T, U, V)` — tuple of positional types.
    Tuple(Vec<TypeId>),
    /// `{ name: string, age: int }` — structural object type.
    Object(Vec<ObjectField>),
    /// `(params) -> return` — function signature.
    Function(FunctionSig),
    /// `T | U | V` — union type (members sorted by TypeId for canonical form).
    Union(Vec<TypeId>),
    /// `T?` — optional type (sugar for `T | null`).
    Optional(TypeId),

    // ── GaleX special types ────────────────────────────────────────
    /// `signal<T>` — reactive state holding a value of type T.
    Signal(TypeId),
    /// `derive<T>` — computed reactive value of type T.
    Derived(TypeId),
    /// `query<T>` — reactive data fetch returning T.
    Query { result: TypeId },
    /// `guard { fields }` — type + runtime validator.
    Guard(GuardDef),
    /// `store { signals, derives, methods }` — reactive state container.
    Store(StoreDef),
    /// `channel<T>` — real-time WebSocket stream.
    Channel(ChannelDef),
    /// UI component with typed props and slots.
    Component(ComponentDef),
    /// `ref<T>` — DOM element reference.
    DomRef(TypeId),
    /// `enum { A, B, C }` — set of named variants.
    Enum(EnumDef),

    // ── Inference / resolution ──────────────────────────────────────
    /// Unresolved type variable (assigned during inference).
    TypeVar(TypeVarId),
    /// Unresolved type name (before lookup in TypeEnv).
    Named(SmolStr),
}

// ── Supporting structures ──────────────────────────────────────────────

/// A field in an object type.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ObjectField {
    pub name: SmolStr,
    pub ty: TypeId,
    pub optional: bool,
}

/// A function signature.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FunctionSig {
    pub params: Vec<FnParam>,
    pub ret: TypeId,
    pub is_async: bool,
}

/// A parameter in a function signature.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FnParam {
    pub name: SmolStr,
    pub ty: TypeId,
    pub has_default: bool,
}

/// A guard definition with validated fields.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GuardDef {
    pub name: SmolStr,
    pub fields: Vec<GuardField>,
    /// Parent guard name for `extends` (e.g., `guard Admin extends User`).
    pub extends: Option<SmolStr>,
    /// Whether any field has non-empty validations (marks as runtime-validated).
    pub has_validators: bool,
}

/// A single field in a guard, with its base type and validation chain.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GuardField {
    pub name: SmolStr,
    pub ty: TypeId,
    pub validations: Vec<Validation>,
}

/// A store definition — reactive state container.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StoreDef {
    pub name: SmolStr,
    pub signals: Vec<StoreSignal>,
    pub derives: Vec<StoreDerive>,
    pub methods: Vec<StoreMethod>,
}

/// A signal inside a store.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StoreSignal {
    pub name: SmolStr,
    pub ty: TypeId,
}

/// A derived value inside a store.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StoreDerive {
    pub name: SmolStr,
    pub ty: TypeId,
}

/// A method inside a store.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StoreMethod {
    pub name: SmolStr,
    pub sig: FunctionSig,
}

/// A channel definition.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChannelDef {
    pub param_ty: TypeId,
    pub msg_ty: TypeId,
    pub direction: ChannelDirection,
}

/// Channel data flow direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChannelDirection {
    /// `->` server to client only
    ServerToClient,
    /// `<-` client to server only
    ClientToServer,
    /// `<->` bidirectional
    Bidirectional,
}

/// A UI component definition.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ComponentDef {
    pub name: SmolStr,
    pub props: Vec<PropDef>,
    pub slots: Vec<SmolStr>,
}

/// A component prop definition.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PropDef {
    pub name: SmolStr,
    pub ty: TypeId,
    pub has_default: bool,
}

/// An enum definition — a set of named string variants.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EnumDef {
    pub name: SmolStr,
    pub variants: Vec<SmolStr>,
}

// ── TypeInterner ───────────────────────────────────────────────────────

/// Central type store with hash-consing deduplication.
///
/// All types in the program are allocated here and referenced by [`TypeId`].
/// Structurally identical types always share the same `TypeId`.
pub struct TypeInterner {
    /// Type storage — indexed by `TypeId`.
    types: Vec<TypeData>,
    /// Hash-consing map for deduplication.
    dedup: HashMap<TypeData, TypeId>,
    /// Counter for generating fresh type variables.
    next_type_var: u32,

    // Pre-allocated primitive TypeIds for fast access.
    /// `string` type.
    pub string: TypeId,
    /// `int` type.
    pub int: TypeId,
    /// `float` type.
    pub float: TypeId,
    /// `bool` type.
    pub bool_: TypeId,
    /// `null` type.
    pub null: TypeId,
    /// `void` type (no return value).
    pub void: TypeId,
    /// `never` type (unreachable / bottom).
    pub never: TypeId,
}

impl TypeInterner {
    /// Create a new interner with all primitive types pre-allocated.
    pub fn new() -> Self {
        let mut interner = Self {
            types: Vec::new(),
            dedup: HashMap::new(),
            next_type_var: 0,
            // Temporaries — will be overwritten immediately
            string: TypeId(0),
            int: TypeId(0),
            float: TypeId(0),
            bool_: TypeId(0),
            null: TypeId(0),
            void: TypeId(0),
            never: TypeId(0),
        };

        // Pre-intern all primitive types
        interner.string = interner.intern(TypeData::String);
        interner.int = interner.intern(TypeData::Int);
        interner.float = interner.intern(TypeData::Float);
        interner.bool_ = interner.intern(TypeData::Bool);
        interner.null = interner.intern(TypeData::Null);
        interner.void = interner.intern(TypeData::Void);
        interner.never = interner.intern(TypeData::Never);

        interner
    }

    /// Intern a type. Returns the existing `TypeId` if an identical type
    /// was previously interned, otherwise allocates a new one.
    pub fn intern(&mut self, data: TypeData) -> TypeId {
        if let Some(&id) = self.dedup.get(&data) {
            return id;
        }
        let id = TypeId(self.types.len() as u32);
        self.types.push(data.clone());
        self.dedup.insert(data, id);
        id
    }

    /// Look up a type by its ID.
    ///
    /// # Panics
    /// Panics if the `TypeId` is invalid (not allocated by this interner).
    pub fn get(&self, id: TypeId) -> &TypeData {
        &self.types[id.0 as usize]
    }

    /// Total number of interned types.
    pub fn len(&self) -> usize {
        self.types.len()
    }

    /// Whether the interner is empty (should never be — primitives are pre-allocated).
    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }

    /// Generate a fresh type variable for inference.
    pub fn fresh_type_var(&mut self) -> TypeId {
        let var_id = TypeVarId(self.next_type_var);
        self.next_type_var += 1;
        self.intern(TypeData::TypeVar(var_id))
    }

    /// Create an `Array(element_type)`.
    pub fn make_array(&mut self, elem: TypeId) -> TypeId {
        self.intern(TypeData::Array(elem))
    }

    /// Create an `Optional(inner)` type — sugar for `T | null`.
    pub fn make_optional(&mut self, inner: TypeId) -> TypeId {
        self.intern(TypeData::Optional(inner))
    }

    /// Create a `Union` type from the given members.
    ///
    /// - Sorts members by `TypeId` for canonical form.
    /// - Deduplicates identical members.
    /// - Flattens nested unions.
    /// - A single-member union is unwrapped to the member itself.
    pub fn make_union(&mut self, types: Vec<TypeId>) -> TypeId {
        let mut flat = Vec::new();
        for ty in types {
            match self.get(ty) {
                TypeData::Union(members) => flat.extend(members.iter().copied()),
                _ => flat.push(ty),
            }
        }
        flat.sort_by_key(|id| id.0);
        flat.dedup();

        if flat.len() == 1 {
            return flat[0];
        }

        self.intern(TypeData::Union(flat))
    }

    /// Create a `Function` type.
    pub fn make_function(&mut self, sig: FunctionSig) -> TypeId {
        self.intern(TypeData::Function(sig))
    }

    /// Create a `Signal(T)` type.
    pub fn make_signal(&mut self, inner: TypeId) -> TypeId {
        self.intern(TypeData::Signal(inner))
    }

    /// Create a `Derived(T)` type.
    pub fn make_derived(&mut self, inner: TypeId) -> TypeId {
        self.intern(TypeData::Derived(inner))
    }

    /// Create a `DomRef(T)` type.
    pub fn make_dom_ref(&mut self, inner: TypeId) -> TypeId {
        self.intern(TypeData::DomRef(inner))
    }

    /// Create a string literal type.
    pub fn make_string_literal(&mut self, value: &str) -> TypeId {
        self.intern(TypeData::StringLiteral(SmolStr::new(value)))
    }

    /// Create an int literal type.
    pub fn make_int_literal(&mut self, value: i64) -> TypeId {
        self.intern(TypeData::IntLiteral(value))
    }

    /// Create a `Guard` type.
    pub fn make_guard(&mut self, def: GuardDef) -> TypeId {
        self.intern(TypeData::Guard(def))
    }

    /// Create a named (unresolved) type reference.
    pub fn make_named(&mut self, name: &str) -> TypeId {
        self.intern(TypeData::Named(SmolStr::new(name)))
    }

    /// Format a type for display (human-readable).
    pub fn display(&self, id: TypeId) -> String {
        match self.get(id) {
            TypeData::String => "string".into(),
            TypeData::Int => "int".into(),
            TypeData::Float => "float".into(),
            TypeData::Bool => "bool".into(),
            TypeData::Void => "void".into(),
            TypeData::Never => "never".into(),
            TypeData::Null => "null".into(),
            TypeData::StringLiteral(s) => format!("\"{}\"", s),
            TypeData::IntLiteral(n) => format!("{}", n),
            TypeData::Array(elem) => format!("{}[]", self.display(*elem)),
            TypeData::Tuple(elems) => {
                let parts: Vec<_> = elems.iter().map(|e| self.display(*e)).collect();
                format!("({})", parts.join(", "))
            }
            TypeData::Object(fields) => {
                let parts: Vec<_> = fields
                    .iter()
                    .map(|f| {
                        let opt = if f.optional { "?" } else { "" };
                        format!("{}{}: {}", f.name, opt, self.display(f.ty))
                    })
                    .collect();
                format!("{{ {} }}", parts.join(", "))
            }
            TypeData::Function(sig) => {
                let params: Vec<_> = sig
                    .params
                    .iter()
                    .map(|p| format!("{}: {}", p.name, self.display(p.ty)))
                    .collect();
                let async_prefix = if sig.is_async { "async " } else { "" };
                format!(
                    "{}({}) -> {}",
                    async_prefix,
                    params.join(", "),
                    self.display(sig.ret)
                )
            }
            TypeData::Union(members) => {
                let parts: Vec<_> = members.iter().map(|m| self.display(*m)).collect();
                parts.join(" | ")
            }
            TypeData::Optional(inner) => format!("{}?", self.display(*inner)),
            TypeData::Signal(inner) => format!("signal<{}>", self.display(*inner)),
            TypeData::Derived(inner) => format!("derived<{}>", self.display(*inner)),
            TypeData::Query { result } => format!("query<{}>", self.display(*result)),
            TypeData::Guard(g) => format!("guard {}", g.name),
            TypeData::Store(s) => format!("store {}", s.name),
            TypeData::Channel(c) => {
                let dir = match c.direction {
                    ChannelDirection::ServerToClient => "->",
                    ChannelDirection::ClientToServer => "<-",
                    ChannelDirection::Bidirectional => "<->",
                };
                format!("channel {} {}", dir, self.display(c.msg_ty))
            }
            TypeData::Component(c) => format!("component {}", c.name),
            TypeData::DomRef(inner) => format!("ref<{}>", self.display(*inner)),
            TypeData::Enum(e) => format!("enum {}", e.name),
            TypeData::TypeVar(v) => format!("?T{}", v.0),
            TypeData::Named(name) => name.to_string(),
        }
    }

    /// Recursively substitute type variables inside compound types.
    ///
    /// Unlike `ConstraintSolver::resolve()` which only follows the TypeVar chain,
    /// this rebuilds compound types with all nested vars replaced.
    pub fn deep_resolve(
        &mut self,
        ty: TypeId,
        subs: &std::collections::HashMap<TypeVarId, TypeId>,
    ) -> TypeId {
        let data = self.get(ty).clone();
        match data {
            TypeData::TypeVar(v) => {
                if let Some(&resolved) = subs.get(&v) {
                    self.deep_resolve(resolved, subs)
                } else {
                    ty
                }
            }
            TypeData::Array(elem) => {
                let elem2 = self.deep_resolve(elem, subs);
                if elem2 == elem {
                    ty
                } else {
                    self.make_array(elem2)
                }
            }
            TypeData::Optional(inner) => {
                let inner2 = self.deep_resolve(inner, subs);
                if inner2 == inner {
                    ty
                } else {
                    self.make_optional(inner2)
                }
            }
            TypeData::Signal(inner) => {
                let inner2 = self.deep_resolve(inner, subs);
                if inner2 == inner {
                    ty
                } else {
                    self.make_signal(inner2)
                }
            }
            TypeData::Derived(inner) => {
                let inner2 = self.deep_resolve(inner, subs);
                if inner2 == inner {
                    ty
                } else {
                    self.make_derived(inner2)
                }
            }
            TypeData::DomRef(inner) => {
                let inner2 = self.deep_resolve(inner, subs);
                if inner2 == inner {
                    ty
                } else {
                    self.make_dom_ref(inner2)
                }
            }
            TypeData::Query { result } => {
                let result2 = self.deep_resolve(result, subs);
                if result2 == result {
                    ty
                } else {
                    self.intern(TypeData::Query { result: result2 })
                }
            }
            TypeData::Tuple(elems) => {
                let elems2: Vec<_> = elems.iter().map(|e| self.deep_resolve(*e, subs)).collect();
                if elems2 == elems {
                    ty
                } else {
                    self.intern(TypeData::Tuple(elems2))
                }
            }
            TypeData::Union(members) => {
                let members2: Vec<_> = members
                    .iter()
                    .map(|m| self.deep_resolve(*m, subs))
                    .collect();
                if members2 == members {
                    ty
                } else {
                    self.make_union(members2)
                }
            }
            TypeData::Function(sig) => {
                let params2: Vec<_> = sig
                    .params
                    .iter()
                    .map(|p| {
                        let ty2 = self.deep_resolve(p.ty, subs);
                        FnParam {
                            name: p.name.clone(),
                            ty: ty2,
                            has_default: p.has_default,
                        }
                    })
                    .collect();
                let ret2 = self.deep_resolve(sig.ret, subs);
                let changed = ret2 != sig.ret
                    || params2
                        .iter()
                        .zip(sig.params.iter())
                        .any(|(a, b)| a.ty != b.ty);
                if !changed {
                    ty
                } else {
                    self.make_function(FunctionSig {
                        params: params2,
                        ret: ret2,
                        is_async: sig.is_async,
                    })
                }
            }
            TypeData::Object(fields) => {
                let fields2: Vec<_> = fields
                    .iter()
                    .map(|f| {
                        let ty2 = self.deep_resolve(f.ty, subs);
                        ObjectField {
                            name: f.name.clone(),
                            ty: ty2,
                            optional: f.optional,
                        }
                    })
                    .collect();
                let changed = fields2.iter().zip(fields.iter()).any(|(a, b)| a.ty != b.ty);
                if !changed {
                    ty
                } else {
                    self.intern(TypeData::Object(fields2))
                }
            }
            TypeData::Guard(g) => {
                let fields2: Vec<_> = g
                    .fields
                    .iter()
                    .map(|f| GuardField {
                        name: f.name.clone(),
                        ty: self.deep_resolve(f.ty, subs),
                        validations: f.validations.clone(),
                    })
                    .collect();
                let changed = fields2
                    .iter()
                    .zip(g.fields.iter())
                    .any(|(a, b)| a.ty != b.ty);
                if !changed {
                    ty
                } else {
                    self.make_guard(GuardDef {
                        name: g.name.clone(),
                        fields: fields2,
                        extends: g.extends.clone(),
                        has_validators: g.has_validators,
                    })
                }
            }
            // Primitives, literals, named, store, etc. — no nested TypeIds to resolve
            _ => ty,
        }
    }
}

impl Default for TypeInterner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitives_pre_allocated() {
        let i = TypeInterner::new();
        assert_eq!(i.get(i.string), &TypeData::String);
        assert_eq!(i.get(i.int), &TypeData::Int);
        assert_eq!(i.get(i.float), &TypeData::Float);
        assert_eq!(i.get(i.bool_), &TypeData::Bool);
        assert_eq!(i.get(i.null), &TypeData::Null);
        assert_eq!(i.get(i.void), &TypeData::Void);
        assert_eq!(i.get(i.never), &TypeData::Never);
        assert_eq!(i.len(), 7);
    }

    #[test]
    fn interning_deduplicates() {
        let mut i = TypeInterner::new();
        let a = i.intern(TypeData::Array(i.int));
        let b = i.intern(TypeData::Array(i.int));
        assert_eq!(a, b, "identical types should share the same TypeId");
    }

    #[test]
    fn different_types_get_different_ids() {
        let mut i = TypeInterner::new();
        let arr_int = i.make_array(i.int);
        let arr_str = i.make_array(i.string);
        assert_ne!(arr_int, arr_str);
    }

    #[test]
    fn fresh_type_vars_are_unique() {
        let mut i = TypeInterner::new();
        let v1 = i.fresh_type_var();
        let v2 = i.fresh_type_var();
        assert_ne!(v1, v2);
    }

    #[test]
    fn union_sorts_and_deduplicates() {
        let mut i = TypeInterner::new();
        let u1 = i.make_union(vec![i.string, i.int, i.string]);
        let u2 = i.make_union(vec![i.int, i.string]);
        assert_eq!(u1, u2, "order and duplicates shouldn't matter");
    }

    #[test]
    fn union_flattens_nested() {
        let mut i = TypeInterner::new();
        let inner = i.make_union(vec![i.int, i.string]);
        let outer = i.make_union(vec![inner, i.bool_]);
        // Should flatten to int | string | bool (3 members)
        match i.get(outer) {
            TypeData::Union(members) => assert_eq!(members.len(), 3),
            other => panic!("expected Union, got {:?}", other),
        }
    }

    #[test]
    fn single_member_union_unwraps() {
        let mut i = TypeInterner::new();
        let u = i.make_union(vec![i.int]);
        assert_eq!(u, i.int, "single-member union should unwrap");
    }

    #[test]
    fn display_primitives() {
        let i = TypeInterner::new();
        assert_eq!(i.display(i.string), "string");
        assert_eq!(i.display(i.int), "int");
        assert_eq!(i.display(i.null), "null");
    }

    #[test]
    fn display_compound() {
        let mut i = TypeInterner::new();
        let arr = i.make_array(i.int);
        assert_eq!(i.display(arr), "int[]");

        let opt = i.make_optional(i.string);
        assert_eq!(i.display(opt), "string?");

        let union = i.make_union(vec![i.string, i.null]);
        assert_eq!(i.display(union), "string | null");
    }

    #[test]
    fn display_function() {
        let mut i = TypeInterner::new();
        let sig = FunctionSig {
            params: vec![
                FnParam {
                    name: "a".into(),
                    ty: i.int,
                    has_default: false,
                },
                FnParam {
                    name: "b".into(),
                    ty: i.int,
                    has_default: false,
                },
            ],
            ret: i.int,
            is_async: false,
        };
        let f = i.make_function(sig);
        assert_eq!(i.display(f), "(a: int, b: int) -> int");
    }

    #[test]
    fn string_literal_types() {
        let mut i = TypeInterner::new();
        let a = i.make_string_literal("primary");
        let b = i.make_string_literal("primary");
        let c = i.make_string_literal("ghost");
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_eq!(i.display(a), "\"primary\"");
    }
}
