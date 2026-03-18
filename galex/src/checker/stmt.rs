//! Statement type checking.

use super::TypeChecker;
use crate::ast::*;
use crate::errors::codes;
use crate::types::env::{BindingKind, ScopeKind};
use crate::types::ty::TypeData;

impl TypeChecker {
    /// Type-check a statement.
    pub(super) fn check_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Let {
                name,
                ty_ann,
                init,
                span,
            } => {
                let init_ty = self.infer_expr(init);
                let ty = if let Some(ann) = ty_ann {
                    let expected = self.resolve_annotation(ann);
                    self.constrain_assignable(
                        init_ty,
                        expected,
                        *span,
                        &format!("in assignment to `{}`", name),
                    );
                    expected
                } else {
                    init_ty
                };
                self.define_binding(name.clone(), ty, BindingKind::Let, *span);
            }

            Stmt::Mut {
                name,
                ty_ann,
                init,
                span,
            } => {
                let init_ty = self.infer_expr(init);
                let ty = if let Some(ann) = ty_ann {
                    let expected = self.resolve_annotation(ann);
                    self.constrain_assignable(
                        init_ty,
                        expected,
                        *span,
                        &format!("in assignment to `{}`", name),
                    );
                    expected
                } else {
                    init_ty
                };
                self.define_binding(name.clone(), ty, BindingKind::Mut, *span);
            }

            Stmt::Signal {
                name,
                ty_ann,
                init,
                span,
            } => {
                let init_ty = self.infer_expr(init);
                let inner_ty = if let Some(ann) = ty_ann {
                    let expected = self.resolve_annotation(ann);
                    self.constrain_assignable(
                        init_ty,
                        expected,
                        *span,
                        &format!("signal '{}' initial value", name),
                    );
                    expected
                } else {
                    init_ty
                };
                let signal_ty = self.interner.make_signal(inner_ty);
                self.define_binding(name.clone(), signal_ty, BindingKind::Signal, *span);
            }

            Stmt::Derive { name, init, span } => {
                let init_ty = self.infer_expr(init);
                let derived_ty = self.interner.make_derived(init_ty);
                self.define_binding(name.clone(), derived_ty, BindingKind::Derived, *span);
            }

            Stmt::Frozen { name, init, span } => {
                let init_ty = self.infer_expr(init);
                self.define_binding(name.clone(), init_ty, BindingKind::Frozen, *span);
            }

            Stmt::RefDecl { name, ty_ann, span } => {
                let elem_ty = self.resolve_annotation(ty_ann);
                let ref_ty = self.interner.make_dom_ref(elem_ty);
                self.define_binding(name.clone(), ref_ty, BindingKind::Let, *span);
            }

            Stmt::FnDecl(decl) => {
                self.check_fn_decl(decl);
            }

            Stmt::If {
                condition,
                then_block,
                else_branch,
                span,
            } => {
                let cond_ty = self.infer_expr(condition);
                self.constrain_assignable(
                    cond_ty,
                    self.interner.bool_,
                    *span,
                    "if condition must be a boolean",
                );
                self.check_block(then_block);
                if let Some(else_b) = else_branch {
                    match else_b {
                        ElseBranch::Else(block) => self.check_block(block),
                        ElseBranch::ElseIf(stmt) => self.check_stmt(stmt),
                    }
                }
            }

            Stmt::For {
                binding,
                index,
                iterable,
                body,
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
                            "for loop requires an iterable (array)",
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
                self.check_block(body);
                self.env.pop_scope();
            }

            Stmt::Return { value, span } => {
                let ret_ty = if let Some(expr) = value {
                    self.infer_expr(expr)
                } else {
                    self.interner.void
                };
                if let Some(expected) = self.current_return_type {
                    self.constrain_assignable(ret_ty, expected, *span, "return type mismatch");
                }
            }

            Stmt::Effect { body, cleanup, .. } => {
                self.env.push_scope(ScopeKind::Block);
                self.check_block(body);
                if let Some(cleanup_block) = cleanup {
                    self.check_block(cleanup_block);
                }
                self.env.pop_scope();
            }

            Stmt::Watch {
                target,
                next_name,
                prev_name,
                body,
                span,
            } => {
                let target_ty = self.infer_expr(target);

                // Validate that the watch target references at least one reactive source
                if !self.references_reactive_source(target) {
                    self.emit_error(crate::types::constraint::TypeError {
                        code: &codes::GX0300,
                        expected: self.interner.void,
                        actual: target_ty,
                        span: *span,
                        context: "watch expression does not reference any reactive source \
                                  (signal or derived) — the watch body will never re-execute"
                            .into(),
                        kind: crate::types::constraint::TypeErrorKind::TypeMismatch,
                    });
                }

                // Unwrap Signal/Derived to get the inner value type
                let value_ty = match self.interner.get(target_ty).clone() {
                    TypeData::Signal(inner) => inner,
                    TypeData::Derived(inner) => inner,
                    _ => target_ty,
                };
                self.env.push_scope(ScopeKind::Block);
                self.define_binding(next_name.clone(), value_ty, BindingKind::Let, *span);
                self.define_binding(prev_name.clone(), value_ty, BindingKind::Let, *span);
                self.check_block(body);
                self.env.pop_scope();
            }

            Stmt::ExprStmt { expr, .. } => {
                self.infer_expr(expr);
            }

            Stmt::Block(block) => {
                self.env.push_scope(ScopeKind::Block);
                self.check_block(block);
                self.env.pop_scope();
            }
        }
    }

    /// Check a block of statements.
    pub(super) fn check_block(&mut self, block: &Block) {
        for stmt in &block.stmts {
            self.check_stmt(stmt);
        }
    }
}
