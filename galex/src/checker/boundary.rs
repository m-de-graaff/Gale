//! Server/client boundary checking.
//!
//! Enforces the boundary rules at type-check time:
//!
//! - Client code CANNOT reference server-scoped bindings (except action stubs)
//! - Server code CANNOT reference client-scoped bindings (signals, DOM refs)
//! - Shared code CAN be referenced from either scope
//! - Data flowing from server to client must be serializable
//!
//! Also validates:
//! - `out` exports are coherent with the declaration's boundary scope
//! - `env()` access — server vars only in server scope, `PUBLIC_` vars anywhere
//! - Declarations placed inside the correct boundary blocks

use super::TypeChecker;
use crate::ast::*;
use crate::span::Span;
use crate::types::constraint::{TypeError, TypeErrorKind};
use crate::types::env::{Binding, BindingKind, BoundaryScope, ScopeKind};
use crate::types::ty::{TypeData, TypeId};

impl TypeChecker {
    // ── Implicit boundary mapping ──────────────────────────────────

    /// Determine the implicit boundary scope for a binding kind.
    ///
    /// Some declaration forms are inherently server-only or client-only
    /// regardless of where they appear in the source:
    ///
    /// - `action`, `query` → Server (server-side RPC / data fetching)
    /// - `signal`, `derived`, `component` → Client (reactive UI primitives)
    /// - `guard`, `enum`, `type alias` → Shared (data shapes, valid on both sides)
    /// - Everything else → `None` (inherits from enclosing boundary block)
    pub(super) fn implicit_boundary_for_kind(kind: BindingKind) -> Option<BoundaryScope> {
        match kind {
            BindingKind::Action
            | BindingKind::Query
            | BindingKind::Api
            | BindingKind::Middleware => Some(BoundaryScope::Server),
            BindingKind::Signal | BindingKind::Derived | BindingKind::Component => {
                Some(BoundaryScope::Client)
            }
            BindingKind::Guard | BindingKind::EnumDef | BindingKind::TypeAlias => {
                Some(BoundaryScope::Shared)
            }
            _ => None,
        }
    }

    /// Compute the effective boundary scope for a new binding.
    ///
    /// If the binding kind has an implicit scope (e.g., `action` → Server),
    /// that takes precedence. Otherwise, the enclosing boundary block
    /// determines the scope.
    pub(super) fn effective_boundary(
        &self,
        kind: BindingKind,
        enclosing: BoundaryScope,
    ) -> BoundaryScope {
        Self::implicit_boundary_for_kind(kind).unwrap_or(enclosing)
    }

    // ── Cross-boundary access checking ─────────────────────────────

    /// Check whether a reference to `binding` is valid from the current scope.
    ///
    /// Called whenever an identifier is resolved. The rules:
    ///
    /// | Declaration | Reference Site | Result |
    /// |-------------|----------------|--------|
    /// | Server      | Client         | ERROR (unless Action — stub is OK) |
    /// | Server      | Shared         | ERROR |
    /// | Client      | Server         | ERROR |
    /// | Client      | Shared         | ERROR |
    /// | Shared      | Any            | OK |
    /// | Unscoped    | Any            | OK |
    /// | Any         | Unscoped       | OK |
    pub(super) fn check_boundary_access(
        &mut self,
        name: &str,
        binding: &Binding,
        reference_span: Span,
    ) {
        let ref_scope = self.env.current_boundary_scope();
        let decl_scope = binding.boundary;

        // Unscoped and Shared bindings are universally accessible.
        // References from unscoped code have no boundary restriction.
        if matches!(decl_scope, BoundaryScope::Unscoped | BoundaryScope::Shared)
            || matches!(ref_scope, BoundaryScope::Unscoped)
        {
            return;
        }

        match (decl_scope, ref_scope) {
            // Server binding referenced from client — OK only for action stubs
            (BoundaryScope::Server, BoundaryScope::Client) => {
                if binding.kind == BindingKind::Action {
                    return; // Action stubs are the bridge — client calls them as async RPCs
                }
                self.emit_error(TypeError {
                    expected: self.interner.void,
                    actual: self.interner.void,
                    span: reference_span,
                    context: format!(
                        "cannot reference server binding '{}' from client scope",
                        name
                    ),
                    kind: TypeErrorKind::BoundaryViolation {
                        binding_scope: "server".into(),
                        reference_scope: "client".into(),
                        suggestion: "Wrap this code in a server { } block, or use an action to bridge the boundary".to_string(),
                    },
                });
            }

            // Server binding referenced from shared — not allowed
            (BoundaryScope::Server, BoundaryScope::Shared) => {
                self.emit_error(TypeError {
                    expected: self.interner.void,
                    actual: self.interner.void,
                    span: reference_span,
                    context: format!(
                        "cannot reference server binding '{}' from shared scope",
                        name
                    ),
                    kind: TypeErrorKind::BoundaryViolation {
                        binding_scope: "server".into(),
                        reference_scope: "shared".into(),
                        suggestion: "Shared code must be valid on both server and client. \
                             Move this reference into a server { } block"
                            .into(),
                    },
                });
            }

            // Client binding referenced from server — not allowed
            (BoundaryScope::Client, BoundaryScope::Server) => {
                self.emit_error(TypeError {
                    expected: self.interner.void,
                    actual: self.interner.void,
                    span: reference_span,
                    context: format!(
                        "cannot reference client binding '{}' from server scope",
                        name
                    ),
                    kind: TypeErrorKind::BoundaryViolation {
                        binding_scope: "client".into(),
                        reference_scope: "server".into(),
                        suggestion: "Server code cannot access client-side reactive state. \
                             Move this reference into a client { } block"
                            .to_string(),
                    },
                });
            }

            // Client binding referenced from shared — not allowed
            (BoundaryScope::Client, BoundaryScope::Shared) => {
                self.emit_error(TypeError {
                    expected: self.interner.void,
                    actual: self.interner.void,
                    span: reference_span,
                    context: format!(
                        "cannot reference client binding '{}' from shared scope",
                        name
                    ),
                    kind: TypeErrorKind::BoundaryViolation {
                        binding_scope: "client".into(),
                        reference_scope: "shared".into(),
                        suggestion: "Shared code must be valid on both server and client. \
                             Move this reference into a client { } block"
                            .into(),
                    },
                });
            }

            // Same scope or other valid combinations — OK
            _ => {}
        }
    }

    // ── Serializability checking ────────────────────────────────────

    /// Check whether a type is serializable — i.e., can cross the
    /// server→client boundary.
    ///
    /// Serializable types: primitives, literal types, arrays, tuples,
    /// objects, optionals, unions, guards, enums (if all inner types are
    /// serializable).
    ///
    /// NOT serializable: functions, signals, derived, DOM refs, channels,
    /// stores, queries.
    pub(super) fn is_serializable(&self, ty: TypeId) -> bool {
        match self.interner.get(ty) {
            // Primitives — always serializable
            TypeData::String
            | TypeData::Int
            | TypeData::Float
            | TypeData::Bool
            | TypeData::Void
            | TypeData::Null
            | TypeData::Never
            | TypeData::StringLiteral(_)
            | TypeData::IntLiteral(_) => true,

            // Compound — serializable if all inner types are
            TypeData::Array(elem) => self.is_serializable(*elem),
            TypeData::Tuple(elems) => elems.iter().all(|e| self.is_serializable(*e)),
            TypeData::Object(fields) => fields.iter().all(|f| self.is_serializable(f.ty)),
            TypeData::Optional(inner) => self.is_serializable(*inner),
            TypeData::Union(members) => members.iter().all(|m| self.is_serializable(*m)),

            // Guard — serializable if all field types are (guards are data shapes)
            TypeData::Guard(g) => g.fields.iter().all(|f| self.is_serializable(f.ty)),

            // Enum — always serializable (set of string variants)
            TypeData::Enum(_) => true,

            // NOT serializable — these are runtime constructs
            TypeData::Function(_) => false,
            TypeData::Signal(_) => false,
            TypeData::Derived(_) => false,
            TypeData::DomRef(_) => false,
            TypeData::Channel(_) => false,
            TypeData::Store(_) => false,
            TypeData::Query { .. } => false,
            TypeData::Component(_) => false,

            // Type variables / unresolved — assume serializable (can't know yet)
            TypeData::TypeVar(_) | TypeData::Named(_) => true,
        }
    }

    // ── Declaration-in-boundary validation ──────────────────────────

    /// Validate that a declaration is valid inside the given boundary block.
    ///
    /// Some declarations are inherently scoped and cannot appear in the
    /// wrong boundary:
    /// - `action` in `client { }` → ERROR (actions are server-only)
    /// - `signal` / `derive` / `ref` in `server { }` → ERROR (client-only)
    /// - `component` in `server { }` → ERROR (client-only)
    pub(super) fn check_declaration_in_boundary(&mut self, item: &Item, enclosing: ScopeKind) {
        match (item, enclosing) {
            // Actions cannot be declared inside client blocks
            (Item::ActionDecl(decl), ScopeKind::ClientBlock) => {
                self.emit_error(TypeError {
                    expected: self.interner.void,
                    actual: self.interner.void,
                    span: decl.span,
                    context: format!(
                        "action '{}' cannot be declared inside a client {{ }} block",
                        decl.name
                    ),
                    kind: TypeErrorKind::BoundaryViolation {
                        binding_scope: "server".into(),
                        reference_scope: "client".into(),
                        suggestion:
                            "Actions are server-only. Move this declaration to a server { } block"
                                .into(),
                    },
                });
            }

            // Queries cannot be declared inside client blocks
            (Item::QueryDecl(decl), ScopeKind::ClientBlock) => {
                self.emit_error(TypeError {
                    expected: self.interner.void,
                    actual: self.interner.void,
                    span: decl.span,
                    context: format!(
                        "query '{}' cannot be declared inside a client {{ }} block",
                        decl.name
                    ),
                    kind: TypeErrorKind::BoundaryViolation {
                        binding_scope: "server".into(),
                        reference_scope: "client".into(),
                        suggestion:
                            "Queries are server-only. Move this declaration to a server { } block"
                                .into(),
                    },
                });
            }

            // Components cannot be declared inside server blocks
            (Item::ComponentDecl(decl), ScopeKind::ServerBlock) => {
                self.emit_error(TypeError {
                    expected: self.interner.void,
                    actual: self.interner.void,
                    span: decl.span,
                    context: format!(
                        "component '{}' cannot be declared inside a server {{ }} block",
                        decl.name
                    ),
                    kind: TypeErrorKind::BoundaryViolation {
                        binding_scope: "client".into(),
                        reference_scope: "server".into(),
                        suggestion:
                            "Components are client-only. Move this declaration to a client { } block"
                                .into(),
                    },
                });
            }

            // Layouts cannot be declared inside server blocks
            (Item::LayoutDecl(decl), ScopeKind::ServerBlock) => {
                self.emit_error(TypeError {
                    expected: self.interner.void,
                    actual: self.interner.void,
                    span: decl.span,
                    context: format!(
                        "layout '{}' cannot be declared inside a server {{ }} block",
                        decl.name
                    ),
                    kind: TypeErrorKind::BoundaryViolation {
                        binding_scope: "client".into(),
                        reference_scope: "server".into(),
                        suggestion: "Layouts are client-rendered. Declare them at the top level"
                            .into(),
                    },
                });
            }

            // API routes cannot be declared inside client blocks
            (Item::ApiDecl(decl), ScopeKind::ClientBlock) => {
                self.emit_error(TypeError {
                    expected: self.interner.void,
                    actual: self.interner.void,
                    span: decl.span,
                    context: format!(
                        "api '{}' cannot be declared inside a client {{ }} block",
                        decl.name
                    ),
                    kind: TypeErrorKind::BoundaryViolation {
                        binding_scope: "server".into(),
                        reference_scope: "client".into(),
                        suggestion:
                            "API routes are server-only. Move this to a server { } block or top level"
                                .into(),
                    },
                });
            }

            // Middleware cannot be declared inside client blocks
            (Item::MiddlewareDecl(decl), ScopeKind::ClientBlock) => {
                self.emit_error(TypeError {
                    expected: self.interner.void,
                    actual: self.interner.void,
                    span: decl.span,
                    context: format!(
                        "middleware '{}' cannot be declared inside a client {{ }} block",
                        decl.name
                    ),
                    kind: TypeErrorKind::BoundaryViolation {
                        binding_scope: "server".into(),
                        reference_scope: "client".into(),
                        suggestion:
                            "Middleware is server-only. Move this to a server { } block or top level"
                                .into(),
                    },
                });
            }

            // Env declarations cannot be inside client blocks
            (Item::EnvDecl(decl), ScopeKind::ClientBlock) => {
                self.emit_error(TypeError {
                    expected: self.interner.void,
                    actual: self.interner.void,
                    span: decl.span,
                    context: "env declarations cannot be inside a client { } block".into(),
                    kind: TypeErrorKind::BoundaryViolation {
                        binding_scope: "server".into(),
                        reference_scope: "client".into(),
                        suggestion:
                            "Env declarations are server-only. Move to top level or a server { } block"
                                .into(),
                    },
                });
            }

            // Signals / derives / refs inside server blocks
            (Item::Stmt(stmt), ScopeKind::ServerBlock) => {
                match stmt {
                    Stmt::Signal { name, span, .. } => {
                        self.emit_error(TypeError {
                            expected: self.interner.void,
                            actual: self.interner.void,
                            span: *span,
                            context: format!(
                                "signal '{}' cannot be declared inside a server {{ }} block",
                                name
                            ),
                            kind: TypeErrorKind::BoundaryViolation {
                                binding_scope: "client".into(),
                                reference_scope: "server".into(),
                                suggestion: "Signals are client-only reactive state. \
                                     Move this declaration to a client { } block"
                                    .into(),
                            },
                        });
                    }
                    Stmt::Derive { name, span, .. } => {
                        self.emit_error(TypeError {
                            expected: self.interner.void,
                            actual: self.interner.void,
                            span: *span,
                            context: format!(
                                "derive '{}' cannot be declared inside a server {{ }} block",
                                name
                            ),
                            kind: TypeErrorKind::BoundaryViolation {
                                binding_scope: "client".into(),
                                reference_scope: "server".into(),
                                suggestion: "Derived values are client-only reactive state. \
                                     Move this declaration to a client { } block"
                                    .into(),
                            },
                        });
                    }
                    Stmt::RefDecl { name, span, .. } => {
                        self.emit_error(TypeError {
                            expected: self.interner.void,
                            actual: self.interner.void,
                            span: *span,
                            context: format!(
                                "ref '{}' cannot be declared inside a server {{ }} block",
                                name
                            ),
                            kind: TypeErrorKind::BoundaryViolation {
                                binding_scope: "client".into(),
                                reference_scope: "server".into(),
                                suggestion: "DOM refs are client-only. \
                                     Move this declaration to a client { } block"
                                    .into(),
                            },
                        });
                    }
                    _ => {} // Other statements are fine in server blocks
                }
            }

            // Everything else is valid in its enclosing block
            _ => {}
        }
    }

    // ── env() access validation ────────────────────────────────────

    /// Validate an `env.KEY` access.
    ///
    /// - `env.PUBLIC_*` variables are accessible in all scopes.
    /// - All other env variables are server-only.
    pub(super) fn check_env_access(&mut self, key: &str, span: Span) {
        if key.starts_with("PUBLIC_") {
            return; // Public env vars are accessible everywhere
        }

        let current = self.env.current_boundary_scope();
        if matches!(current, BoundaryScope::Client) {
            self.emit_error(TypeError {
                expected: self.interner.void,
                actual: self.interner.void,
                span,
                context: format!(
                    "env.{} is a server-only environment variable \
                     and cannot be accessed from client scope",
                    key
                ),
                kind: TypeErrorKind::InvalidEnvAccess,
            });
        }
    }

    // ── out export validation ──────────────────────────────────────

    /// Validate that an `out` export is coherent with its boundary scope.
    ///
    /// Rules:
    /// - `out action` → OK (server export, client receives a stub)
    /// - `out ui Component` → OK, but not inside `server { }`
    /// - `out guard` / `out type` / `out enum` → OK (shared exports)
    /// - `out fn` in `server { }` → return type must be serializable
    /// - `out signal` → ERROR (cannot export reactive state)
    /// - `out store` → scope-dependent
    pub(super) fn validate_export(&mut self, inner: &Item, span: Span) {
        let current = self.env.current_boundary_scope();

        match inner {
            // Actions are always valid exports — they're the server→client bridge
            Item::ActionDecl(_) => {}

            // Guards, enums, type aliases — always valid (shared data shapes)
            Item::GuardDecl(_) | Item::EnumDecl(_) | Item::TypeAlias(_) => {}

            // Components — valid, but not from server scope
            Item::ComponentDecl(decl) => {
                if matches!(current, BoundaryScope::Server) {
                    self.emit_error(TypeError {
                        expected: self.interner.void,
                        actual: self.interner.void,
                        span,
                        context: format!(
                            "cannot export component '{}' from a server {{ }} block",
                            decl.name
                        ),
                        kind: TypeErrorKind::InvalidExport {
                            suggestion:
                                "Components are client-side. Move this to a client { } block \
                                 or top level"
                                    .into(),
                        },
                    });
                }
            }

            // Functions — if in server scope, return type must be serializable
            Item::FnDecl(decl) => {
                if matches!(current, BoundaryScope::Server) {
                    // Look up the function's type to check its return
                    if let Some(binding) = self.env.lookup(&decl.name) {
                        let fn_ty = binding.ty;
                        if let TypeData::Function(sig) = self.interner.get(fn_ty).clone() {
                            if !self.is_serializable(sig.ret) {
                                self.emit_error(TypeError {
                                    expected: self.interner.void,
                                    actual: sig.ret,
                                    span,
                                    context: format!(
                                        "exported server function '{}' has a non-serializable \
                                         return type '{}'",
                                        decl.name,
                                        self.interner.display(sig.ret)
                                    ),
                                    kind: TypeErrorKind::NotSerializable,
                                });
                            }
                        }
                    }
                }
            }

            // Stores — only valid from client scope
            Item::StoreDecl(decl) => {
                if matches!(current, BoundaryScope::Server) {
                    self.emit_error(TypeError {
                        expected: self.interner.void,
                        actual: self.interner.void,
                        span,
                        context: format!(
                            "cannot export store '{}' from a server {{ }} block",
                            decl.name
                        ),
                        kind: TypeErrorKind::InvalidExport {
                            suggestion: "Stores contain reactive state and are client-only. \
                                 Move this to a client { } block"
                                .into(),
                        },
                    });
                }
            }

            // Signals cannot be exported directly
            Item::Stmt(Stmt::Signal { name, .. }) => {
                self.emit_error(TypeError {
                    expected: self.interner.void,
                    actual: self.interner.void,
                    span,
                    context: format!("cannot export signal '{}'", name),
                    kind: TypeErrorKind::InvalidExport {
                        suggestion: "Signals are local reactive state and cannot be exported. \
                             Use a store to encapsulate shared reactive state"
                            .into(),
                    },
                });
            }

            // Queries — valid export from server context
            Item::QueryDecl(_) => {}

            // Channels — valid export
            Item::ChannelDecl(_) => {}

            // API resources — valid export (server-only, but OK from unscoped too)
            Item::ApiDecl(_) => {}

            // Middleware — valid export
            Item::MiddlewareDecl(_) => {}

            // Env declarations — valid (not really exported, but not an error)
            Item::EnvDecl(_) => {}

            // Nested out, boundary blocks, etc. — no specific validation
            _ => {}
        }
    }
}
