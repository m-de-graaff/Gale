//! Operator typing rules for binary and unary operations.

use super::TypeChecker;
use crate::ast::BinOp;
use crate::errors::codes;
use crate::span::Span;
use crate::types::constraint::TypeErrorKind;
use crate::types::ty::{TypeData, TypeId};

impl TypeChecker {
    /// Infer the result type of a binary operation.
    ///
    /// Returns the result type, and emits constraints/errors for operands.
    pub(super) fn infer_binary_op(
        &mut self,
        left_ty: TypeId,
        op: BinOp,
        right_ty: TypeId,
        span: Span,
    ) -> TypeId {
        match op {
            // ── Arithmetic ──────────────────────────────────────
            BinOp::Add => self.infer_add(left_ty, right_ty, span),
            BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod => {
                self.infer_numeric_op(left_ty, right_ty, op, span)
            }

            // ── Comparison ──────────────────────────────────────
            BinOp::Eq | BinOp::NotEq => {
                // Any compatible types, result is bool
                self.interner.bool_
            }
            BinOp::Lt | BinOp::Gt | BinOp::LtEq | BinOp::GtEq => {
                self.check_comparable(left_ty, right_ty, span);
                self.interner.bool_
            }

            // ── Logical ─────────────────────────────────────────
            BinOp::And | BinOp::Or => {
                self.constrain_assignable(left_ty, self.interner.bool_, span, "logical operand");
                self.constrain_assignable(right_ty, self.interner.bool_, span, "logical operand");
                self.interner.bool_
            }

            // ── Range ───────────────────────────────────────────
            BinOp::DotDot => {
                self.constrain_assignable(left_ty, self.interner.int, span, "range start");
                self.constrain_assignable(right_ty, self.interner.int, span, "range end");
                self.interner.make_array(self.interner.int)
            }
        }
    }

    /// `+` is special: works for numeric types AND string concatenation.
    fn infer_add(&mut self, left: TypeId, right: TypeId, span: Span) -> TypeId {
        let left_data = self.interner.get(left).clone();
        let right_data = self.interner.get(right).clone();

        match (&left_data, &right_data) {
            // int + int → int
            (TypeData::Int | TypeData::IntLiteral(_), TypeData::Int | TypeData::IntLiteral(_)) => {
                self.interner.int
            }
            // float + float → float, int + float → float, float + int → float
            (TypeData::Float, TypeData::Float)
            | (TypeData::Int | TypeData::IntLiteral(_), TypeData::Float)
            | (TypeData::Float, TypeData::Int | TypeData::IntLiteral(_)) => self.interner.float,
            // string + string → string
            (
                TypeData::String | TypeData::StringLiteral(_),
                TypeData::String | TypeData::StringLiteral(_),
            ) => self.interner.string,
            // TypeVar — defer to constraint solver
            (TypeData::TypeVar(_), _) | (_, TypeData::TypeVar(_)) => {
                // Can't determine at this point; create a fresh var and constrain
                let result = self.interner.fresh_type_var();
                self.constrain_equal(left, right, span, "operands of `+`");
                result
            }
            _ => {
                self.error_type_mismatch(
                    self.interner.int, // suggest numeric
                    left,
                    span,
                    "operator `+` requires numeric or string operands",
                );
                self.interner.never
            }
        }
    }

    /// Numeric operators: `- * / %` — require numeric operands.
    fn infer_numeric_op(&mut self, left: TypeId, right: TypeId, _op: BinOp, span: Span) -> TypeId {
        let left_data = self.interner.get(left).clone();
        let right_data = self.interner.get(right).clone();

        match (&left_data, &right_data) {
            (TypeData::Int | TypeData::IntLiteral(_), TypeData::Int | TypeData::IntLiteral(_)) => {
                self.interner.int
            }
            (TypeData::Float, TypeData::Float)
            | (TypeData::Int | TypeData::IntLiteral(_), TypeData::Float)
            | (TypeData::Float, TypeData::Int | TypeData::IntLiteral(_)) => self.interner.float,
            (TypeData::TypeVar(_), _) | (_, TypeData::TypeVar(_)) => {
                let result = self.interner.fresh_type_var();
                self.constrain_equal(left, right, span, "numeric operands");
                result
            }
            _ => {
                self.error_type_mismatch(
                    self.interner.int,
                    left,
                    span,
                    "arithmetic operator requires numeric operands",
                );
                self.interner.never
            }
        }
    }

    /// Check that comparison operands are comparable (numeric or string).
    fn check_comparable(&mut self, left: TypeId, right: TypeId, span: Span) {
        let left_data = self.interner.get(left).clone();
        let right_data = self.interner.get(right).clone();

        let left_ok = matches!(
            left_data,
            TypeData::Int
                | TypeData::Float
                | TypeData::String
                | TypeData::IntLiteral(_)
                | TypeData::StringLiteral(_)
                | TypeData::TypeVar(_)
        );
        let right_ok = matches!(
            right_data,
            TypeData::Int
                | TypeData::Float
                | TypeData::String
                | TypeData::IntLiteral(_)
                | TypeData::StringLiteral(_)
                | TypeData::TypeVar(_)
        );

        if !left_ok || !right_ok {
            self.emit_error(crate::types::constraint::TypeError {
                code: &codes::GX0310,
                expected: self.interner.int,
                actual: if !left_ok { left } else { right },
                span,
                context: "comparison requires numeric or string operands".into(),
                kind: TypeErrorKind::TypeMismatch,
            });
        }
    }

    /// Infer the result type of a unary operation.
    pub(super) fn infer_unary_op(
        &mut self,
        op: crate::ast::UnaryOp,
        operand_ty: TypeId,
        span: Span,
    ) -> TypeId {
        match op {
            crate::ast::UnaryOp::Neg => {
                // -x requires numeric
                let data = self.interner.get(operand_ty).clone();
                match data {
                    TypeData::Int | TypeData::IntLiteral(_) => self.interner.int,
                    TypeData::Float => self.interner.float,
                    TypeData::TypeVar(_) => operand_ty, // defer
                    _ => {
                        self.error_type_mismatch(
                            self.interner.int,
                            operand_ty,
                            span,
                            "unary `-` requires a numeric operand",
                        );
                        self.interner.never
                    }
                }
            }
            crate::ast::UnaryOp::Not => {
                self.constrain_assignable(
                    operand_ty,
                    self.interner.bool_,
                    span,
                    "unary `!` requires a boolean operand",
                );
                self.interner.bool_
            }
        }
    }
}
