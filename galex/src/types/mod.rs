//! GaleX type system — type representation, environments, and constraint solving.
//!
//! # Architecture
//!
//! - [`ty`] — Core type representation (`TypeData`, `TypeId`, `TypeInterner`)
//! - [`env`] — Scoped type environment (`TypeEnv`, `Binding`, `ScopeKind`)
//! - [`constraint`] — Constraint solver with Robinson unification
//! - [`validation`] — Guard field validation constraints

pub mod constraint;
pub mod env;
pub mod ty;
pub mod validation;
