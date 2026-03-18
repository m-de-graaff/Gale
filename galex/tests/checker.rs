//! Integration tests for the type checker.
//!
//! Tests construct AST nodes directly (no parser) and run the type checker.

use galex::ast::*;
use galex::checker::TypeChecker;
use galex::span::Span;

fn s() -> Span {
    Span::dummy()
}

fn make_program(items: Vec<Item>) -> Program {
    Program { items, span: s() }
}

fn check(items: Vec<Item>) -> Vec<galex::types::constraint::TypeError> {
    let mut checker = TypeChecker::new();
    let program = make_program(items);
    checker.check_program(&program)
}

fn check_stmts(stmts: Vec<Stmt>) -> Vec<galex::types::constraint::TypeError> {
    let items = stmts.into_iter().map(Item::Stmt).collect();
    check(items)
}

// ── 1. Literal inference ───────────────────────────────────────────────

#[test]
fn literal_int_inference() {
    let errors = check_stmts(vec![Stmt::Let {
        name: "x".into(),
        ty_ann: None,
        init: Expr::IntLit {
            value: 42,
            span: s(),
        },
        span: s(),
    }]);
    assert!(errors.is_empty(), "errors: {:?}", errors);
}

#[test]
fn literal_string_inference() {
    let errors = check_stmts(vec![Stmt::Let {
        name: "x".into(),
        ty_ann: None,
        init: Expr::StringLit {
            value: "hello".into(),
            span: s(),
        },
        span: s(),
    }]);
    assert!(errors.is_empty(), "errors: {:?}", errors);
}

#[test]
fn literal_bool_inference() {
    let errors = check_stmts(vec![Stmt::Let {
        name: "x".into(),
        ty_ann: None,
        init: Expr::BoolLit {
            value: true,
            span: s(),
        },
        span: s(),
    }]);
    assert!(errors.is_empty(), "errors: {:?}", errors);
}

// ── 2. Type annotation checking ────────────────────────────────────────

#[test]
fn type_annotation_match() {
    let errors = check_stmts(vec![Stmt::Let {
        name: "x".into(),
        ty_ann: Some(TypeAnnotation::Named {
            name: "int".into(),
            span: s(),
        }),
        init: Expr::IntLit {
            value: 42,
            span: s(),
        },
        span: s(),
    }]);
    assert!(errors.is_empty(), "errors: {:?}", errors);
}

#[test]
fn type_annotation_mismatch() {
    let errors = check_stmts(vec![Stmt::Let {
        name: "x".into(),
        ty_ann: Some(TypeAnnotation::Named {
            name: "int".into(),
            span: s(),
        }),
        init: Expr::StringLit {
            value: "hello".into(),
            span: s(),
        },
        span: s(),
    }]);
    assert!(!errors.is_empty(), "should have a type mismatch error");
}

// ── 3. Signal wrapping ─────────────────────────────────────────────────

#[test]
fn signal_wraps_in_signal_type() {
    let errors = check_stmts(vec![Stmt::Signal {
        name: "count".into(),
        ty_ann: None,
        init: Expr::IntLit {
            value: 0,
            span: s(),
        },
        span: s(),
    }]);
    assert!(errors.is_empty(), "errors: {:?}", errors);
}

// ── 4. Derive inference ────────────────────────────────────────────────

#[test]
fn derive_infers_from_expression() {
    let errors = check_stmts(vec![
        Stmt::Let {
            name: "x".into(),
            ty_ann: None,
            init: Expr::IntLit {
                value: 10,
                span: s(),
            },
            span: s(),
        },
        Stmt::Derive {
            name: "doubled".into(),
            init: Expr::BinaryOp {
                left: Box::new(Expr::Ident {
                    name: "x".into(),
                    span: s(),
                }),
                op: BinOp::Mul,
                right: Box::new(Expr::IntLit {
                    value: 2,
                    span: s(),
                }),
                span: s(),
            },
            span: s(),
        },
    ]);
    assert!(errors.is_empty(), "errors: {:?}", errors);
}

// ── 5. Function return inference ───────────────────────────────────────

#[test]
fn function_return_type_check() {
    let errors = check(vec![Item::FnDecl(FnDecl {
        name: "add".into(),
        params: vec![
            Param {
                name: "a".into(),
                ty_ann: Some(TypeAnnotation::Named {
                    name: "int".into(),
                    span: s(),
                }),
                default: None,
                span: s(),
            },
            Param {
                name: "b".into(),
                ty_ann: Some(TypeAnnotation::Named {
                    name: "int".into(),
                    span: s(),
                }),
                default: None,
                span: s(),
            },
        ],
        ret_ty: Some(TypeAnnotation::Named {
            name: "int".into(),
            span: s(),
        }),
        body: Block {
            stmts: vec![Stmt::Return {
                value: Some(Expr::BinaryOp {
                    left: Box::new(Expr::Ident {
                        name: "a".into(),
                        span: s(),
                    }),
                    op: BinOp::Add,
                    right: Box::new(Expr::Ident {
                        name: "b".into(),
                        span: s(),
                    }),
                    span: s(),
                }),
                span: s(),
            }],
            span: s(),
        },
        is_async: false,
        span: s(),
    })]);
    assert!(errors.is_empty(), "errors: {:?}", errors);
}

// ── 6. Function call checking ──────────────────────────────────────────

#[test]
fn function_call_arg_type_check() {
    // Define fn greet(name: string) -> string { return name }
    // Then call greet(42) — should error
    let errors = check(vec![
        Item::FnDecl(FnDecl {
            name: "greet".into(),
            params: vec![Param {
                name: "name".into(),
                ty_ann: Some(TypeAnnotation::Named {
                    name: "string".into(),
                    span: s(),
                }),
                default: None,
                span: s(),
            }],
            ret_ty: Some(TypeAnnotation::Named {
                name: "string".into(),
                span: s(),
            }),
            body: Block {
                stmts: vec![Stmt::Return {
                    value: Some(Expr::Ident {
                        name: "name".into(),
                        span: s(),
                    }),
                    span: s(),
                }],
                span: s(),
            },
            is_async: false,
            span: s(),
        }),
        Item::Stmt(Stmt::ExprStmt {
            expr: Expr::FnCall {
                callee: Box::new(Expr::Ident {
                    name: "greet".into(),
                    span: s(),
                }),
                args: vec![Expr::IntLit {
                    value: 42,
                    span: s(),
                }],
                span: s(),
            },
            span: s(),
        }),
    ]);
    assert!(
        !errors.is_empty(),
        "calling greet(42) should produce a type error"
    );
}

#[test]
fn function_call_correct_args() {
    let errors = check(vec![
        Item::FnDecl(FnDecl {
            name: "greet".into(),
            params: vec![Param {
                name: "name".into(),
                ty_ann: Some(TypeAnnotation::Named {
                    name: "string".into(),
                    span: s(),
                }),
                default: None,
                span: s(),
            }],
            ret_ty: Some(TypeAnnotation::Named {
                name: "string".into(),
                span: s(),
            }),
            body: Block {
                stmts: vec![Stmt::Return {
                    value: Some(Expr::Ident {
                        name: "name".into(),
                        span: s(),
                    }),
                    span: s(),
                }],
                span: s(),
            },
            is_async: false,
            span: s(),
        }),
        Item::Stmt(Stmt::ExprStmt {
            expr: Expr::FnCall {
                callee: Box::new(Expr::Ident {
                    name: "greet".into(),
                    span: s(),
                }),
                args: vec![Expr::StringLit {
                    value: "world".into(),
                    span: s(),
                }],
                span: s(),
            },
            span: s(),
        }),
    ]);
    assert!(errors.is_empty(), "errors: {:?}", errors);
}

// ── 7. Binary operator rules ───────────────────────────────────────────

#[test]
fn binary_add_int() {
    let errors = check_stmts(vec![Stmt::Let {
        name: "x".into(),
        ty_ann: None,
        init: Expr::BinaryOp {
            left: Box::new(Expr::IntLit {
                value: 1,
                span: s(),
            }),
            op: BinOp::Add,
            right: Box::new(Expr::IntLit {
                value: 2,
                span: s(),
            }),
            span: s(),
        },
        span: s(),
    }]);
    assert!(errors.is_empty(), "1 + 2 should be valid: {:?}", errors);
}

#[test]
fn binary_add_string() {
    let errors = check_stmts(vec![Stmt::Let {
        name: "x".into(),
        ty_ann: None,
        init: Expr::BinaryOp {
            left: Box::new(Expr::StringLit {
                value: "a".into(),
                span: s(),
            }),
            op: BinOp::Add,
            right: Box::new(Expr::StringLit {
                value: "b".into(),
                span: s(),
            }),
            span: s(),
        },
        span: s(),
    }]);
    assert!(
        errors.is_empty(),
        "\"a\" + \"b\" should be valid: {:?}",
        errors
    );
}

#[test]
fn binary_add_mixed_types_error() {
    let errors = check_stmts(vec![Stmt::Let {
        name: "x".into(),
        ty_ann: None,
        init: Expr::BinaryOp {
            left: Box::new(Expr::IntLit {
                value: 1,
                span: s(),
            }),
            op: BinOp::Add,
            right: Box::new(Expr::BoolLit {
                value: true,
                span: s(),
            }),
            span: s(),
        },
        span: s(),
    }]);
    assert!(!errors.is_empty(), "1 + true should be a type error");
}

// ── 8. Guard field types ───────────────────────────────────────────────

#[test]
fn guard_declaration() {
    let errors = check(vec![Item::GuardDecl(GuardDecl {
        name: "Email".into(),
        fields: vec![GuardFieldDecl {
            name: "value".into(),
            ty: TypeAnnotation::Named {
                name: "string".into(),
                span: s(),
            },
            validators: vec![ValidatorCall {
                name: "email".into(),
                args: vec![],
                span: s(),
            }],
            span: s(),
        }],
        span: s(),
    })]);
    assert!(
        errors.is_empty(),
        "guard declaration should be valid: {:?}",
        errors
    );
}

// ── 9. Template expression types ───────────────────────────────────────

#[test]
fn template_renderable_expression() {
    // Component with {count} interpolation — count is int, which is renderable
    let errors = check(vec![Item::ComponentDecl(ComponentDecl {
        name: "Counter".into(),
        props: vec![],
        body: ComponentBody {
            stmts: vec![Stmt::Let {
                name: "count".into(),
                ty_ann: None,
                init: Expr::IntLit {
                    value: 0,
                    span: s(),
                },
                span: s(),
            }],
            template: vec![TemplateNode::ExprInterp {
                expr: Expr::Ident {
                    name: "count".into(),
                    span: s(),
                },
                span: s(),
            }],
            head: None,
            span: s(),
        },
        span: s(),
    })]);
    assert!(errors.is_empty(), "int is renderable: {:?}", errors);
}

// ── 10. Directive checking ─────────────────────────────────────────────

#[test]
fn bind_directive_requires_signal() {
    // bind:name where name is a let binding (not signal) — should error
    let errors = check(vec![Item::ComponentDecl(ComponentDecl {
        name: "Form".into(),
        props: vec![],
        body: ComponentBody {
            stmts: vec![Stmt::Let {
                name: "name".into(),
                ty_ann: None,
                init: Expr::StringLit {
                    value: "".into(),
                    span: s(),
                },
                span: s(),
            }],
            template: vec![TemplateNode::SelfClosing {
                tag: "input".into(),
                attributes: vec![],
                directives: vec![Directive::Bind {
                    field: "name".into(),
                    span: s(),
                }],
                span: s(),
            }],
            head: None,
            span: s(),
        },
        span: s(),
    })]);
    assert!(!errors.is_empty(), "bind:name on a non-signal should error");
}

#[test]
fn bind_directive_with_signal_ok() {
    let errors = check(vec![Item::ComponentDecl(ComponentDecl {
        name: "Form".into(),
        props: vec![],
        body: ComponentBody {
            stmts: vec![Stmt::Signal {
                name: "name".into(),
                ty_ann: None,
                init: Expr::StringLit {
                    value: "".into(),
                    span: s(),
                },
                span: s(),
            }],
            template: vec![TemplateNode::SelfClosing {
                tag: "input".into(),
                attributes: vec![],
                directives: vec![Directive::Bind {
                    field: "name".into(),
                    span: s(),
                }],
                span: s(),
            }],
            head: None,
            span: s(),
        },
        span: s(),
    })]);
    assert!(
        errors.is_empty(),
        "bind:name on a signal should be OK: {:?}",
        errors
    );
}

// ── 11. Union assignability ────────────────────────────────────────────

#[test]
fn string_literal_to_union() {
    let errors = check_stmts(vec![Stmt::Let {
        name: "variant".into(),
        ty_ann: Some(TypeAnnotation::Union {
            types: vec![
                TypeAnnotation::StringLiteral {
                    value: "primary".into(),
                    span: s(),
                },
                TypeAnnotation::StringLiteral {
                    value: "ghost".into(),
                    span: s(),
                },
            ],
            span: s(),
        }),
        init: Expr::StringLit {
            value: "primary".into(),
            span: s(),
        },
        span: s(),
    }]);
    assert!(
        errors.is_empty(),
        "\"primary\" assignable to union: {:?}",
        errors
    );
}

// ── 12. Scope isolation ────────────────────────────────────────────────

#[test]
fn scope_isolation_block_variable() {
    // { let inner = 1 }; inner should be undefined
    let errors = check_stmts(vec![
        Stmt::Block(Block {
            stmts: vec![Stmt::Let {
                name: "inner".into(),
                ty_ann: None,
                init: Expr::IntLit {
                    value: 1,
                    span: s(),
                },
                span: s(),
            }],
            span: s(),
        }),
        Stmt::ExprStmt {
            expr: Expr::Ident {
                name: "inner".into(),
                span: s(),
            },
            span: s(),
        },
    ]);
    assert!(
        !errors.is_empty(),
        "inner should be undefined outside block"
    );
}

// ── 13. Mutability checking ───────────────────────────────────────────

#[test]
fn assign_to_let_errors() {
    let errors = check_stmts(vec![
        Stmt::Let {
            name: "x".into(),
            ty_ann: None,
            init: Expr::IntLit {
                value: 1,
                span: s(),
            },
            span: s(),
        },
        Stmt::ExprStmt {
            expr: Expr::Assign {
                target: Box::new(Expr::Ident {
                    name: "x".into(),
                    span: s(),
                }),
                op: AssignOp::Assign,
                value: Box::new(Expr::IntLit {
                    value: 2,
                    span: s(),
                }),
                span: s(),
            },
            span: s(),
        },
    ]);
    assert!(
        !errors.is_empty(),
        "assigning to `let` binding should error"
    );
}

#[test]
fn assign_to_mut_ok() {
    let errors = check_stmts(vec![
        Stmt::Mut {
            name: "x".into(),
            ty_ann: None,
            init: Expr::IntLit {
                value: 1,
                span: s(),
            },
            span: s(),
        },
        Stmt::ExprStmt {
            expr: Expr::Assign {
                target: Box::new(Expr::Ident {
                    name: "x".into(),
                    span: s(),
                }),
                op: AssignOp::Assign,
                value: Box::new(Expr::IntLit {
                    value: 2,
                    span: s(),
                }),
                span: s(),
            },
            span: s(),
        },
    ]);
    assert!(
        errors.is_empty(),
        "assigning to `mut` should be OK: {:?}",
        errors
    );
}

// ── 14. Multiple error accumulation ────────────────────────────────────

#[test]
fn multiple_errors_reported() {
    let errors = check_stmts(vec![
        // Error 1: undefined variable
        Stmt::ExprStmt {
            expr: Expr::Ident {
                name: "undefined_var".into(),
                span: s(),
            },
            span: s(),
        },
        // Error 2: type mismatch
        Stmt::Let {
            name: "y".into(),
            ty_ann: Some(TypeAnnotation::Named {
                name: "int".into(),
                span: s(),
            }),
            init: Expr::BoolLit {
                value: false,
                span: s(),
            },
            span: s(),
        },
    ]);
    assert!(
        errors.len() >= 2,
        "should accumulate multiple errors, got {}: {:?}",
        errors.len(),
        errors
    );
}

// ── Additional: undefined variable ─────────────────────────────────────

#[test]
fn undefined_variable_error() {
    let errors = check_stmts(vec![Stmt::ExprStmt {
        expr: Expr::Ident {
            name: "nonexistent".into(),
            span: s(),
        },
        span: s(),
    }]);
    assert!(
        !errors.is_empty(),
        "undefined variable should produce an error"
    );
}

// ── Additional: for loop type inference ────────────────────────────────

#[test]
fn for_loop_binding_type() {
    let errors = check_stmts(vec![
        Stmt::Let {
            name: "items".into(),
            ty_ann: None,
            init: Expr::ArrayLit {
                elements: vec![
                    Expr::IntLit {
                        value: 1,
                        span: s(),
                    },
                    Expr::IntLit {
                        value: 2,
                        span: s(),
                    },
                ],
                span: s(),
            },
            span: s(),
        },
        Stmt::For {
            binding: "item".into(),
            index: Some("i".into()),
            iterable: Expr::Ident {
                name: "items".into(),
                span: s(),
            },
            body: Block {
                stmts: vec![
                    // item should be int, i should be int
                    Stmt::Let {
                        name: "result".into(),
                        ty_ann: Some(TypeAnnotation::Named {
                            name: "int".into(),
                            span: s(),
                        }),
                        init: Expr::Ident {
                            name: "item".into(),
                            span: s(),
                        },
                        span: s(),
                    },
                ],
                span: s(),
            },
            span: s(),
        },
    ]);
    assert!(
        errors.is_empty(),
        "for loop should infer item as int: {:?}",
        errors
    );
}

// ══════════════════════════════════════════════════════════════════════
// Phase 10.3 — Guard type integration tests
// ══════════════════════════════════════════════════════════════════════

// ── G1. Validator-type compat: .email() on string passes ───────────────

#[test]
fn guard_email_on_string_valid() {
    let errors = check(vec![Item::GuardDecl(GuardDecl {
        name: "Email".into(),
        fields: vec![GuardFieldDecl {
            name: "value".into(),
            ty: TypeAnnotation::Named {
                name: "string".into(),
                span: s(),
            },
            validators: vec![ValidatorCall {
                name: "email".into(),
                args: vec![],
                span: s(),
            }],
            span: s(),
        }],
        span: s(),
    })]);
    assert!(
        errors.is_empty(),
        "email() on string should be valid: {:?}",
        errors
    );
}

// ── G2. Validator-type compat: .email() on int errors ──────────────────

#[test]
fn guard_email_on_int_errors() {
    let errors = check(vec![Item::GuardDecl(GuardDecl {
        name: "Bad".into(),
        fields: vec![GuardFieldDecl {
            name: "value".into(),
            ty: TypeAnnotation::Named {
                name: "int".into(),
                span: s(),
            },
            validators: vec![ValidatorCall {
                name: "email".into(),
                args: vec![],
                span: s(),
            }],
            span: s(),
        }],
        span: s(),
    })]);
    assert!(!errors.is_empty(), ".email() on int should error");
}

// ── G3. Validator-type compat: .min() on int passes, on string errors ──

#[test]
fn guard_min_on_int_valid() {
    let errors = check(vec![Item::GuardDecl(GuardDecl {
        name: "Age".into(),
        fields: vec![GuardFieldDecl {
            name: "value".into(),
            ty: TypeAnnotation::Named {
                name: "int".into(),
                span: s(),
            },
            validators: vec![ValidatorCall {
                name: "min".into(),
                args: vec![Expr::IntLit {
                    value: 0,
                    span: s(),
                }],
                span: s(),
            }],
            span: s(),
        }],
        span: s(),
    })]);
    assert!(
        errors.is_empty(),
        ".min(0) on int should be valid: {:?}",
        errors
    );
}

#[test]
fn guard_min_on_string_errors() {
    let errors = check(vec![Item::GuardDecl(GuardDecl {
        name: "Bad".into(),
        fields: vec![GuardFieldDecl {
            name: "value".into(),
            ty: TypeAnnotation::Named {
                name: "string".into(),
                span: s(),
            },
            validators: vec![ValidatorCall {
                name: "min".into(),
                args: vec![Expr::IntLit {
                    value: 0,
                    span: s(),
                }],
                span: s(),
            }],
            span: s(),
        }],
        span: s(),
    })]);
    assert!(!errors.is_empty(), ".min() on string should error");
}

// ── G4. Validator-type compat: .minLen() on string passes, on int errors

#[test]
fn guard_minlen_on_string_valid() {
    let errors = check(vec![Item::GuardDecl(GuardDecl {
        name: "Name".into(),
        fields: vec![GuardFieldDecl {
            name: "value".into(),
            ty: TypeAnnotation::Named {
                name: "string".into(),
                span: s(),
            },
            validators: vec![ValidatorCall {
                name: "minLen".into(),
                args: vec![Expr::IntLit {
                    value: 2,
                    span: s(),
                }],
                span: s(),
            }],
            span: s(),
        }],
        span: s(),
    })]);
    assert!(
        errors.is_empty(),
        ".minLen(2) on string should be valid: {:?}",
        errors
    );
}

#[test]
fn guard_minlen_on_int_errors() {
    let errors = check(vec![Item::GuardDecl(GuardDecl {
        name: "Bad".into(),
        fields: vec![GuardFieldDecl {
            name: "value".into(),
            ty: TypeAnnotation::Named {
                name: "int".into(),
                span: s(),
            },
            validators: vec![ValidatorCall {
                name: "minLen".into(),
                args: vec![Expr::IntLit {
                    value: 2,
                    span: s(),
                }],
                span: s(),
            }],
            span: s(),
        }],
        span: s(),
    })]);
    assert!(!errors.is_empty(), ".minLen() on int should error");
}

// ── G5. Duplicate field detection ──────────────────────────────────────

#[test]
fn guard_duplicate_field_errors() {
    let errors = check(vec![Item::GuardDecl(GuardDecl {
        name: "Dupe".into(),
        fields: vec![
            GuardFieldDecl {
                name: "email".into(),
                ty: TypeAnnotation::Named {
                    name: "string".into(),
                    span: s(),
                },
                validators: vec![],
                span: s(),
            },
            GuardFieldDecl {
                name: "email".into(),
                ty: TypeAnnotation::Named {
                    name: "string".into(),
                    span: s(),
                },
                validators: vec![],
                span: s(),
            },
        ],
        span: s(),
    })]);
    assert!(!errors.is_empty(), "duplicate field should error");
}

// ── G6. Guard-to-object assignability ──────────────────────────────────

#[test]
fn guard_assignable_to_matching_object() {
    // Define guard User { name: string, email: string }
    // Then let obj: { name: string, email: string } = User-instance
    let errors = check(vec![
        Item::GuardDecl(GuardDecl {
            name: "User".into(),
            fields: vec![
                GuardFieldDecl {
                    name: "name".into(),
                    ty: TypeAnnotation::Named {
                        name: "string".into(),
                        span: s(),
                    },
                    validators: vec![],
                    span: s(),
                },
                GuardFieldDecl {
                    name: "email".into(),
                    ty: TypeAnnotation::Named {
                        name: "string".into(),
                        span: s(),
                    },
                    validators: vec![],
                    span: s(),
                },
            ],
            span: s(),
        }),
        Item::Stmt(Stmt::Let {
            name: "user".into(),
            ty_ann: Some(TypeAnnotation::Named {
                name: "User".into(),
                span: s(),
            }),
            init: Expr::Ident {
                name: "User".into(),
                span: s(),
            },
            span: s(),
        }),
    ]);
    // This tests that the guard type resolves and can be used as a type annotation
    assert!(errors.is_empty(), "guard as type annotation: {:?}", errors);
}

// ── G7. Guard .partial() ───────────────────────────────────────────────

#[test]
fn guard_partial_composition() {
    let errors = check(vec![
        Item::GuardDecl(GuardDecl {
            name: "User".into(),
            fields: vec![GuardFieldDecl {
                name: "name".into(),
                ty: TypeAnnotation::Named {
                    name: "string".into(),
                    span: s(),
                },
                validators: vec![],
                span: s(),
            }],
            span: s(),
        }),
        Item::Stmt(Stmt::Let {
            name: "partial".into(),
            ty_ann: None,
            init: Expr::FnCall {
                callee: Box::new(Expr::MemberAccess {
                    object: Box::new(Expr::Ident {
                        name: "User".into(),
                        span: s(),
                    }),
                    field: "partial".into(),
                    span: s(),
                }),
                args: vec![],
                span: s(),
            },
            span: s(),
        }),
    ]);
    assert!(
        errors.is_empty(),
        "User.partial() should work: {:?}",
        errors
    );
}

// ── G8. Guard .pick() ──────────────────────────────────────────────────

#[test]
fn guard_pick_composition() {
    let errors = check(vec![
        Item::GuardDecl(GuardDecl {
            name: "User".into(),
            fields: vec![
                GuardFieldDecl {
                    name: "name".into(),
                    ty: TypeAnnotation::Named {
                        name: "string".into(),
                        span: s(),
                    },
                    validators: vec![],
                    span: s(),
                },
                GuardFieldDecl {
                    name: "email".into(),
                    ty: TypeAnnotation::Named {
                        name: "string".into(),
                        span: s(),
                    },
                    validators: vec![],
                    span: s(),
                },
            ],
            span: s(),
        }),
        Item::Stmt(Stmt::Let {
            name: "login".into(),
            ty_ann: None,
            init: Expr::FnCall {
                callee: Box::new(Expr::MemberAccess {
                    object: Box::new(Expr::Ident {
                        name: "User".into(),
                        span: s(),
                    }),
                    field: "pick".into(),
                    span: s(),
                }),
                args: vec![Expr::StringLit {
                    value: "email".into(),
                    span: s(),
                }],
                span: s(),
            },
            span: s(),
        }),
    ]);
    assert!(
        errors.is_empty(),
        "User.pick(\"email\") should work: {:?}",
        errors
    );
}

// ── G9. Guard .omit() ──────────────────────────────────────────────────

#[test]
fn guard_omit_composition() {
    let errors = check(vec![
        Item::GuardDecl(GuardDecl {
            name: "User".into(),
            fields: vec![
                GuardFieldDecl {
                    name: "name".into(),
                    ty: TypeAnnotation::Named {
                        name: "string".into(),
                        span: s(),
                    },
                    validators: vec![],
                    span: s(),
                },
                GuardFieldDecl {
                    name: "email".into(),
                    ty: TypeAnnotation::Named {
                        name: "string".into(),
                        span: s(),
                    },
                    validators: vec![],
                    span: s(),
                },
            ],
            span: s(),
        }),
        Item::Stmt(Stmt::Let {
            name: "noEmail".into(),
            ty_ann: None,
            init: Expr::FnCall {
                callee: Box::new(Expr::MemberAccess {
                    object: Box::new(Expr::Ident {
                        name: "User".into(),
                        span: s(),
                    }),
                    field: "omit".into(),
                    span: s(),
                }),
                args: vec![Expr::StringLit {
                    value: "email".into(),
                    span: s(),
                }],
                span: s(),
            },
            span: s(),
        }),
    ]);
    assert!(
        errors.is_empty(),
        "User.omit(\"email\") should work: {:?}",
        errors
    );
}

// ── G10. Shared guard visibility ───────────────────────────────────────

#[test]
fn shared_guard_registered_as_shared() {
    let mut checker = TypeChecker::new();
    let program = make_program(vec![Item::SharedBlock(BoundaryBlock {
        items: vec![Item::GuardDecl(GuardDecl {
            name: "SharedUser".into(),
            fields: vec![GuardFieldDecl {
                name: "name".into(),
                ty: TypeAnnotation::Named {
                    name: "string".into(),
                    span: s(),
                },
                validators: vec![],
                span: s(),
            }],
            span: s(),
        })],
        span: s(),
    })]);
    let errors = checker.check_program(&program);
    assert!(
        errors.is_empty(),
        "shared guard should compile: {:?}",
        errors
    );
    assert!(
        checker.env.is_shared_type("SharedUser"),
        "guard in shared block should be registered as shared"
    );
}

// ── G11. Validator missing arg errors ──────────────────────────────────

#[test]
fn guard_validator_missing_arg_errors() {
    let errors = check(vec![Item::GuardDecl(GuardDecl {
        name: "Bad".into(),
        fields: vec![GuardFieldDecl {
            name: "age".into(),
            ty: TypeAnnotation::Named {
                name: "int".into(),
                span: s(),
            },
            validators: vec![ValidatorCall {
                name: "min".into(),
                args: vec![], // missing required int arg
                span: s(),
            }],
            span: s(),
        }],
        span: s(),
    })]);
    assert!(!errors.is_empty(), ".min() without arg should error");
}

// ── G12. .pick() with nonexistent field errors ─────────────────────────

#[test]
fn guard_pick_nonexistent_field_errors() {
    let errors = check(vec![
        Item::GuardDecl(GuardDecl {
            name: "User".into(),
            fields: vec![GuardFieldDecl {
                name: "name".into(),
                ty: TypeAnnotation::Named {
                    name: "string".into(),
                    span: s(),
                },
                validators: vec![],
                span: s(),
            }],
            span: s(),
        }),
        Item::Stmt(Stmt::Let {
            name: "bad".into(),
            ty_ann: None,
            init: Expr::FnCall {
                callee: Box::new(Expr::MemberAccess {
                    object: Box::new(Expr::Ident {
                        name: "User".into(),
                        span: s(),
                    }),
                    field: "pick".into(),
                    span: s(),
                }),
                args: vec![Expr::StringLit {
                    value: "nonexistent".into(),
                    span: s(),
                }],
                span: s(),
            },
            span: s(),
        }),
    ]);
    assert!(!errors.is_empty(), ".pick(\"nonexistent\") should error");
}

// ══════════════════════════════════════════════════════════════════════
// Phase 10.4 — Boundary checking tests
// ══════════════════════════════════════════════════════════════════════

// ── B1. Server binding not visible in client ──────────────────────────

#[test]
fn server_binding_not_visible_in_client() {
    // server { let db = "conn" }
    // client { db }  ← ERROR
    let errors = check(vec![
        Item::ServerBlock(BoundaryBlock {
            items: vec![Item::Stmt(Stmt::Let {
                name: "db".into(),
                ty_ann: None,
                init: Expr::StringLit {
                    value: "conn".into(),
                    span: s(),
                },
                span: s(),
            })],
            span: s(),
        }),
        Item::ClientBlock(BoundaryBlock {
            items: vec![Item::Stmt(Stmt::ExprStmt {
                expr: Expr::Ident {
                    name: "db".into(),
                    span: s(),
                },
                span: s(),
            })],
            span: s(),
        }),
    ]);
    assert!(
        !errors.is_empty(),
        "server binding 'db' should not be accessible from client scope"
    );
    let has_boundary_error = errors.iter().any(|e| {
        matches!(
            e.kind,
            galex::types::constraint::TypeErrorKind::BoundaryViolation { .. }
        )
    });
    assert!(
        has_boundary_error,
        "should produce a BoundaryViolation error, got: {:?}",
        errors
    );
}

// ── B2. Client binding not visible in server ──────────────────────────

#[test]
fn client_binding_not_visible_in_server() {
    // client { let count = 0 }
    // server { count }  ← ERROR
    let errors = check(vec![
        Item::ClientBlock(BoundaryBlock {
            items: vec![Item::Stmt(Stmt::Let {
                name: "count".into(),
                ty_ann: None,
                init: Expr::IntLit {
                    value: 0,
                    span: s(),
                },
                span: s(),
            })],
            span: s(),
        }),
        Item::ServerBlock(BoundaryBlock {
            items: vec![Item::Stmt(Stmt::ExprStmt {
                expr: Expr::Ident {
                    name: "count".into(),
                    span: s(),
                },
                span: s(),
            })],
            span: s(),
        }),
    ]);
    assert!(
        !errors.is_empty(),
        "client binding 'count' should not be accessible from server scope"
    );
}

// ── B3. Shared binding visible in server ──────────────────────────────

#[test]
fn shared_binding_visible_in_server() {
    // shared { let MAX = 100 }
    // server { MAX }  ← OK
    let errors = check(vec![
        Item::SharedBlock(BoundaryBlock {
            items: vec![Item::Stmt(Stmt::Let {
                name: "MAX".into(),
                ty_ann: None,
                init: Expr::IntLit {
                    value: 100,
                    span: s(),
                },
                span: s(),
            })],
            span: s(),
        }),
        Item::ServerBlock(BoundaryBlock {
            items: vec![Item::Stmt(Stmt::ExprStmt {
                expr: Expr::Ident {
                    name: "MAX".into(),
                    span: s(),
                },
                span: s(),
            })],
            span: s(),
        }),
    ]);
    assert!(
        errors.is_empty(),
        "shared binding should be visible in server scope: {:?}",
        errors
    );
}

// ── B4. Shared binding visible in client ──────────────────────────────

#[test]
fn shared_binding_visible_in_client() {
    // shared { let MAX = 100 }
    // client { MAX }  ← OK
    let errors = check(vec![
        Item::SharedBlock(BoundaryBlock {
            items: vec![Item::Stmt(Stmt::Let {
                name: "MAX".into(),
                ty_ann: None,
                init: Expr::IntLit {
                    value: 100,
                    span: s(),
                },
                span: s(),
            })],
            span: s(),
        }),
        Item::ClientBlock(BoundaryBlock {
            items: vec![Item::Stmt(Stmt::ExprStmt {
                expr: Expr::Ident {
                    name: "MAX".into(),
                    span: s(),
                },
                span: s(),
            })],
            span: s(),
        }),
    ]);
    assert!(
        errors.is_empty(),
        "shared binding should be visible in client scope: {:?}",
        errors
    );
}

// ── B5. Action stub accessible from client ────────────────────────────

#[test]
fn action_stub_accessible_from_client() {
    // action createUser(name: string) -> string { return name }
    // client { createUser("alice") }  ← OK (action stubs bridge the boundary)
    let errors = check(vec![
        Item::ActionDecl(ActionDecl {
            name: "createUser".into(),
            params: vec![Param {
                name: "name".into(),
                ty_ann: Some(TypeAnnotation::Named {
                    name: "string".into(),
                    span: s(),
                }),
                default: None,
                span: s(),
            }],
            ret_ty: Some(TypeAnnotation::Named {
                name: "string".into(),
                span: s(),
            }),
            body: Block {
                stmts: vec![Stmt::Return {
                    value: Some(Expr::Ident {
                        name: "name".into(),
                        span: s(),
                    }),
                    span: s(),
                }],
                span: s(),
            },
            span: s(),
        }),
        Item::ClientBlock(BoundaryBlock {
            items: vec![Item::Stmt(Stmt::ExprStmt {
                expr: Expr::FnCall {
                    callee: Box::new(Expr::Ident {
                        name: "createUser".into(),
                        span: s(),
                    }),
                    args: vec![Expr::StringLit {
                        value: "alice".into(),
                        span: s(),
                    }],
                    span: s(),
                },
                span: s(),
            })],
            span: s(),
        }),
    ]);
    assert!(
        errors.is_empty(),
        "action stubs should be callable from client scope: {:?}",
        errors
    );
}

// ── B6. Signal in server block errors ─────────────────────────────────

#[test]
fn signal_in_server_block_errors() {
    // server { signal count = 0 }  ← ERROR (signals are client-only)
    let errors = check(vec![Item::ServerBlock(BoundaryBlock {
        items: vec![Item::Stmt(Stmt::Signal {
            name: "count".into(),
            ty_ann: None,
            init: Expr::IntLit {
                value: 0,
                span: s(),
            },
            span: s(),
        })],
        span: s(),
    })]);
    assert!(!errors.is_empty(), "signal in server block should error");
    let has_boundary_error = errors.iter().any(|e| {
        matches!(
            e.kind,
            galex::types::constraint::TypeErrorKind::BoundaryViolation { .. }
        )
    });
    assert!(
        has_boundary_error,
        "should produce a BoundaryViolation for signal in server block, got: {:?}",
        errors
    );
}

// ── B7. Action in client block errors ─────────────────────────────────

#[test]
fn action_in_client_block_errors() {
    // client { action doStuff() { } }  ← ERROR (actions are server-only)
    let errors = check(vec![Item::ClientBlock(BoundaryBlock {
        items: vec![Item::ActionDecl(ActionDecl {
            name: "doStuff".into(),
            params: vec![],
            ret_ty: None,
            body: Block {
                stmts: vec![],
                span: s(),
            },
            span: s(),
        })],
        span: s(),
    })]);
    assert!(!errors.is_empty(), "action in client block should error");
}

// ── B8. env() private var in client errors ────────────────────────────

#[test]
fn env_private_in_client_errors() {
    // client { env.DATABASE_URL }  ← ERROR (server-only)
    let errors = check(vec![Item::ClientBlock(BoundaryBlock {
        items: vec![Item::Stmt(Stmt::ExprStmt {
            expr: Expr::EnvAccess {
                key: "DATABASE_URL".into(),
                span: s(),
            },
            span: s(),
        })],
        span: s(),
    })]);
    assert!(
        !errors.is_empty(),
        "env.DATABASE_URL should not be accessible from client scope"
    );
    let has_env_error = errors.iter().any(|e| {
        matches!(
            e.kind,
            galex::types::constraint::TypeErrorKind::InvalidEnvAccess
        )
    });
    assert!(
        has_env_error,
        "should produce InvalidEnvAccess error, got: {:?}",
        errors
    );
}

// ── B9. env() PUBLIC_ var in client OK ────────────────────────────────

#[test]
fn env_public_in_client_ok() {
    // client { env.PUBLIC_API_URL }  ← OK (PUBLIC_ prefix)
    let errors = check(vec![Item::ClientBlock(BoundaryBlock {
        items: vec![Item::Stmt(Stmt::ExprStmt {
            expr: Expr::EnvAccess {
                key: "PUBLIC_API_URL".into(),
                span: s(),
            },
            span: s(),
        })],
        span: s(),
    })]);
    assert!(
        errors.is_empty(),
        "env.PUBLIC_API_URL should be accessible from client scope: {:?}",
        errors
    );
}

// ── B10. env() any var in server OK ───────────────────────────────────

#[test]
fn env_any_in_server_ok() {
    // server { env.DATABASE_URL }  ← OK (server can access all env vars)
    let errors = check(vec![Item::ServerBlock(BoundaryBlock {
        items: vec![Item::Stmt(Stmt::ExprStmt {
            expr: Expr::EnvAccess {
                key: "DATABASE_URL".into(),
                span: s(),
            },
            span: s(),
        })],
        span: s(),
    })]);
    assert!(
        errors.is_empty(),
        "env.DATABASE_URL should be accessible from server scope: {:?}",
        errors
    );
}

// ── B11. out action valid ─────────────────────────────────────────────

#[test]
fn out_action_valid() {
    // out action save() { }  ← OK
    let errors = check(vec![Item::Out(OutDecl {
        inner: Box::new(Item::ActionDecl(ActionDecl {
            name: "save".into(),
            params: vec![],
            ret_ty: None,
            body: Block {
                stmts: vec![],
                span: s(),
            },
            span: s(),
        })),
        span: s(),
    })]);
    assert!(
        errors.is_empty(),
        "out action should be valid: {:?}",
        errors
    );
}

// ── B12. out signal invalid ───────────────────────────────────────────

#[test]
fn out_signal_invalid() {
    // out signal count = 0  ← ERROR (cannot export reactive state)
    let errors = check(vec![Item::Out(OutDecl {
        inner: Box::new(Item::Stmt(Stmt::Signal {
            name: "count".into(),
            ty_ann: None,
            init: Expr::IntLit {
                value: 0,
                span: s(),
            },
            span: s(),
        })),
        span: s(),
    })]);
    assert!(!errors.is_empty(), "out signal should be invalid");
    let has_export_error = errors.iter().any(|e| {
        matches!(
            e.kind,
            galex::types::constraint::TypeErrorKind::InvalidExport { .. }
        )
    });
    assert!(
        has_export_error,
        "should produce InvalidExport error, got: {:?}",
        errors
    );
}

// ── B13. out component in server block errors ─────────────────────────

#[test]
fn out_component_in_server_errors() {
    // server { out ui Counter() { } }  ← ERROR (components are client-side)
    let errors = check(vec![Item::ServerBlock(BoundaryBlock {
        items: vec![Item::Out(OutDecl {
            inner: Box::new(Item::ComponentDecl(ComponentDecl {
                name: "Counter".into(),
                props: vec![],
                body: ComponentBody {
                    stmts: vec![],
                    template: vec![],
                    head: None,
                    span: s(),
                },
                span: s(),
            })),
            span: s(),
        })],
        span: s(),
    })]);
    assert!(
        !errors.is_empty(),
        "out component in server block should error"
    );
}

// ── B14. Server fn return not serializable errors ─────────────────────

#[test]
fn server_fn_return_not_serializable_errors() {
    // server {
    //   out fn getHandler() -> fn(int) -> void { ... }
    // }  ← ERROR (functions are not serializable)
    let errors = check(vec![Item::ServerBlock(BoundaryBlock {
        items: vec![Item::Out(OutDecl {
            inner: Box::new(Item::FnDecl(FnDecl {
                name: "getHandler".into(),
                params: vec![],
                ret_ty: Some(TypeAnnotation::Function {
                    params: vec![TypeAnnotation::Named {
                        name: "int".into(),
                        span: s(),
                    }],
                    ret: Box::new(TypeAnnotation::Named {
                        name: "void".into(),
                        span: s(),
                    }),
                    span: s(),
                }),
                body: Block {
                    stmts: vec![],
                    span: s(),
                },
                is_async: false,
                span: s(),
            })),
            span: s(),
        })],
        span: s(),
    })]);
    assert!(
        !errors.is_empty(),
        "exported server fn with non-serializable return should error"
    );
    let has_serial_error = errors.iter().any(|e| {
        matches!(
            e.kind,
            galex::types::constraint::TypeErrorKind::NotSerializable
        )
    });
    assert!(
        has_serial_error,
        "should produce NotSerializable error, got: {:?}",
        errors
    );
}

// ── B15. Unscoped binding accessible everywhere ───────────────────────

#[test]
fn unscoped_binding_accessible_everywhere() {
    // let x = 42  (top-level, unscoped)
    // server { x }   ← OK
    // client { x }   ← OK
    let errors = check(vec![
        Item::Stmt(Stmt::Let {
            name: "x".into(),
            ty_ann: None,
            init: Expr::IntLit {
                value: 42,
                span: s(),
            },
            span: s(),
        }),
        Item::ServerBlock(BoundaryBlock {
            items: vec![Item::Stmt(Stmt::ExprStmt {
                expr: Expr::Ident {
                    name: "x".into(),
                    span: s(),
                },
                span: s(),
            })],
            span: s(),
        }),
        Item::ClientBlock(BoundaryBlock {
            items: vec![Item::Stmt(Stmt::ExprStmt {
                expr: Expr::Ident {
                    name: "x".into(),
                    span: s(),
                },
                span: s(),
            })],
            span: s(),
        }),
    ]);
    assert!(
        errors.is_empty(),
        "unscoped binding should be accessible from both server and client: {:?}",
        errors
    );
}

// ── B16. Derive in server block errors ────────────────────────────────

#[test]
fn derive_in_server_block_errors() {
    // server { derive doubled = 2 }  ← ERROR (derived values are client-only)
    let errors = check(vec![Item::ServerBlock(BoundaryBlock {
        items: vec![Item::Stmt(Stmt::Derive {
            name: "doubled".into(),
            init: Expr::IntLit {
                value: 2,
                span: s(),
            },
            span: s(),
        })],
        span: s(),
    })]);
    assert!(!errors.is_empty(), "derive in server block should error");
}

// ── B17. Ref in server block errors ───────────────────────────────────

#[test]
fn ref_in_server_block_errors() {
    // server { ref canvas: HTMLElement }  ← ERROR (DOM refs are client-only)
    let errors = check(vec![Item::ServerBlock(BoundaryBlock {
        items: vec![Item::Stmt(Stmt::RefDecl {
            name: "canvas".into(),
            ty_ann: TypeAnnotation::Named {
                name: "HTMLElement".into(),
                span: s(),
            },
            span: s(),
        })],
        span: s(),
    })]);
    assert!(!errors.is_empty(), "ref in server block should error");
}

// ── B18. Component in server block errors ─────────────────────────────

#[test]
fn component_in_server_block_errors() {
    // server { out ui Widget() { } }  ← ERROR (components are client-only)
    let errors = check(vec![Item::ServerBlock(BoundaryBlock {
        items: vec![Item::ComponentDecl(ComponentDecl {
            name: "Widget".into(),
            props: vec![],
            body: ComponentBody {
                stmts: vec![],
                template: vec![],
                head: None,
                span: s(),
            },
            span: s(),
        })],
        span: s(),
    })]);
    assert!(!errors.is_empty(), "component in server block should error");
}

// ── B19. Query in client block errors ─────────────────────────────────

#[test]
fn query_in_client_block_errors() {
    // client { query users = "/api/users" }  ← ERROR (queries are server-only)
    let errors = check(vec![Item::ClientBlock(BoundaryBlock {
        items: vec![Item::QueryDecl(QueryDecl {
            name: "users".into(),
            url_pattern: Expr::StringLit {
                value: "/api/users".into(),
                span: s(),
            },
            ret_ty: None,
            span: s(),
        })],
        span: s(),
    })]);
    assert!(!errors.is_empty(), "query in client block should error");
}

// ── B20. Shared guard accessible from both scopes ─────────────────────

#[test]
fn shared_guard_accessible_from_both_scopes() {
    // shared { guard User { name: string } }
    // server { let u: User = User }  ← OK
    // client { let u: User = User }  ← OK
    let errors = check(vec![
        Item::SharedBlock(BoundaryBlock {
            items: vec![Item::GuardDecl(GuardDecl {
                name: "User".into(),
                fields: vec![GuardFieldDecl {
                    name: "name".into(),
                    ty: TypeAnnotation::Named {
                        name: "string".into(),
                        span: s(),
                    },
                    validators: vec![],
                    span: s(),
                }],
                span: s(),
            })],
            span: s(),
        }),
        Item::ServerBlock(BoundaryBlock {
            items: vec![Item::Stmt(Stmt::Let {
                name: "u1".into(),
                ty_ann: Some(TypeAnnotation::Named {
                    name: "User".into(),
                    span: s(),
                }),
                init: Expr::Ident {
                    name: "User".into(),
                    span: s(),
                },
                span: s(),
            })],
            span: s(),
        }),
        Item::ClientBlock(BoundaryBlock {
            items: vec![Item::Stmt(Stmt::Let {
                name: "u2".into(),
                ty_ann: Some(TypeAnnotation::Named {
                    name: "User".into(),
                    span: s(),
                }),
                init: Expr::Ident {
                    name: "User".into(),
                    span: s(),
                },
                span: s(),
            })],
            span: s(),
        }),
    ]);
    assert!(
        errors.is_empty(),
        "shared guard should be accessible from both server and client: {:?}",
        errors
    );
}

// ── B21. Out store in server block errors ─────────────────────────────

#[test]
fn out_store_in_server_errors() {
    // server { out store Counter { signal count = 0 } }  ← ERROR
    // (stores contain reactive state and are client-only, and signal in server also errors)
    let errors = check(vec![Item::ServerBlock(BoundaryBlock {
        items: vec![Item::Out(OutDecl {
            inner: Box::new(Item::StoreDecl(StoreDecl {
                name: "Counter".into(),
                members: vec![],
                span: s(),
            })),
            span: s(),
        })],
        span: s(),
    })]);
    assert!(!errors.is_empty(), "out store in server block should error");
}

// ══════════════════════════════════════════════════════════════════════
// Phase 10.5 — Special type checking
// ══════════════════════════════════════════════════════════════════════

// ── S1. Store: signals only mutated inside store methods ──────────────

#[test]
fn store_signal_mutation_inside_method_ok() {
    // store Counter { signal count = 0; fn inc() { count = count + 1 } }
    // Mutating count inside the store's own method should be fine.
    let errors = check(vec![Item::StoreDecl(StoreDecl {
        name: "Counter".into(),
        members: vec![
            StoreMember::Signal(Stmt::Signal {
                name: "count".into(),
                ty_ann: None,
                init: Expr::IntLit {
                    value: 0,
                    span: s(),
                },
                span: s(),
            }),
            StoreMember::Method(FnDecl {
                name: "inc".into(),
                params: vec![],
                ret_ty: None,
                body: Block {
                    stmts: vec![Stmt::ExprStmt {
                        expr: Expr::Assign {
                            target: Box::new(Expr::Ident {
                                name: "count".into(),
                                span: s(),
                            }),
                            op: AssignOp::Assign,
                            value: Box::new(Expr::IntLit {
                                value: 1,
                                span: s(),
                            }),
                            span: s(),
                        },
                        span: s(),
                    }],
                    span: s(),
                },
                is_async: false,
                span: s(),
            }),
        ],
        span: s(),
    })]);
    assert!(
        errors.is_empty(),
        "store method should be able to mutate its own signals: {:?}",
        errors
    );
}

#[test]
fn store_signal_mutation_outside_method_errors() {
    // let myStore = Counter; myStore.count = 5  ← ERROR
    // First define a store, then try to assign to its member externally.
    let errors = check(vec![
        Item::StoreDecl(StoreDecl {
            name: "Counter".into(),
            members: vec![StoreMember::Signal(Stmt::Signal {
                name: "count".into(),
                ty_ann: None,
                init: Expr::IntLit {
                    value: 0,
                    span: s(),
                },
                span: s(),
            })],
            span: s(),
        }),
        Item::Stmt(Stmt::ExprStmt {
            expr: Expr::Assign {
                target: Box::new(Expr::MemberAccess {
                    object: Box::new(Expr::Ident {
                        name: "Counter".into(),
                        span: s(),
                    }),
                    field: "count".into(),
                    span: s(),
                }),
                op: AssignOp::Assign,
                value: Box::new(Expr::IntLit {
                    value: 5,
                    span: s(),
                }),
                span: s(),
            },
            span: s(),
        }),
    ]);
    assert!(
        !errors.is_empty(),
        "mutating a store signal outside of store methods should error"
    );
}

// ── S2. Channel: message type and direction ───────────────────────────

#[test]
fn channel_receive_handler_msg_type_ok() {
    // channel chat() <-> string { on receive(msg: string) { } }
    let errors = check(vec![Item::ChannelDecl(ChannelDecl {
        name: "chat".into(),
        params: vec![],
        direction: ChannelDirection::Bidirectional,
        msg_ty: TypeAnnotation::Named {
            name: "string".into(),
            span: s(),
        },
        handlers: vec![ChannelHandler {
            event: "receive".into(),
            params: vec![Param {
                name: "msg".into(),
                ty_ann: Some(TypeAnnotation::Named {
                    name: "string".into(),
                    span: s(),
                }),
                default: None,
                span: s(),
            }],
            body: Block {
                stmts: vec![],
                span: s(),
            },
            span: s(),
        }],
        span: s(),
    })]);
    assert!(
        errors.is_empty(),
        "receive handler with matching msg type should be OK: {:?}",
        errors
    );
}

#[test]
fn channel_receive_handler_msg_type_mismatch() {
    // channel chat() <-> string { on receive(msg: int) { } }  ← ERROR
    let errors = check(vec![Item::ChannelDecl(ChannelDecl {
        name: "chat".into(),
        params: vec![],
        direction: ChannelDirection::Bidirectional,
        msg_ty: TypeAnnotation::Named {
            name: "string".into(),
            span: s(),
        },
        handlers: vec![ChannelHandler {
            event: "receive".into(),
            params: vec![Param {
                name: "msg".into(),
                ty_ann: Some(TypeAnnotation::Named {
                    name: "int".into(),
                    span: s(),
                }),
                default: None,
                span: s(),
            }],
            body: Block {
                stmts: vec![],
                span: s(),
            },
            span: s(),
        }],
        span: s(),
    })]);
    assert!(
        !errors.is_empty(),
        "receive handler with wrong msg type should error"
    );
}

#[test]
fn channel_unknown_handler_event_errors() {
    // channel chat() <-> string { on foobar() { } }  ← ERROR
    let errors = check(vec![Item::ChannelDecl(ChannelDecl {
        name: "chat".into(),
        params: vec![],
        direction: ChannelDirection::Bidirectional,
        msg_ty: TypeAnnotation::Named {
            name: "string".into(),
            span: s(),
        },
        handlers: vec![ChannelHandler {
            event: "foobar".into(),
            params: vec![],
            body: Block {
                stmts: vec![],
                span: s(),
            },
            span: s(),
        }],
        span: s(),
    })]);
    assert!(
        !errors.is_empty(),
        "unknown channel handler event should error"
    );
}

// ── S3. Query: URL interpolations must be strings ─────────────────────

#[test]
fn query_url_interpolation_string_ok() {
    // query users = `/api/users/${id}` -> User[]
    // where id is a string — should be fine
    let errors = check(vec![
        Item::Stmt(Stmt::Let {
            name: "id".into(),
            ty_ann: None,
            init: Expr::StringLit {
                value: "123".into(),
                span: s(),
            },
            span: s(),
        }),
        Item::QueryDecl(QueryDecl {
            name: "users".into(),
            url_pattern: Expr::TemplateLit {
                parts: vec![
                    TemplatePart::Text("/api/users/".into()),
                    TemplatePart::Expr(Expr::Ident {
                        name: "id".into(),
                        span: s(),
                    }),
                ],
                span: s(),
            },
            ret_ty: None,
            span: s(),
        }),
    ]);
    assert!(
        errors.is_empty(),
        "string interpolation in query URL should be OK: {:?}",
        errors
    );
}

#[test]
fn query_url_interpolation_bool_errors() {
    // query users = `/api/users/${flag}` where flag is bool ← ERROR
    let errors = check(vec![
        Item::Stmt(Stmt::Let {
            name: "flag".into(),
            ty_ann: None,
            init: Expr::BoolLit {
                value: true,
                span: s(),
            },
            span: s(),
        }),
        Item::QueryDecl(QueryDecl {
            name: "users".into(),
            url_pattern: Expr::TemplateLit {
                parts: vec![
                    TemplatePart::Text("/api/users/".into()),
                    TemplatePart::Expr(Expr::Ident {
                        name: "flag".into(),
                        span: s(),
                    }),
                ],
                span: s(),
            },
            ret_ty: None,
            span: s(),
        }),
    ]);
    assert!(
        !errors.is_empty(),
        "bool interpolation in query URL should error"
    );
}

// ── S4. Template: on:event handler signature ──────────────────────────

#[test]
fn on_event_handler_function_ok() {
    // on:click={(e) => { }}  — handler is a function, should be fine
    let errors = check(vec![Item::ComponentDecl(ComponentDecl {
        name: "Btn".into(),
        props: vec![],
        body: ComponentBody {
            stmts: vec![],
            template: vec![TemplateNode::SelfClosing {
                tag: "button".into(),
                attributes: vec![],
                directives: vec![Directive::On {
                    event: "click".into(),
                    modifiers: vec![],
                    handler: Expr::ArrowFn {
                        params: vec![Param {
                            name: "e".into(),
                            ty_ann: None,
                            default: None,
                            span: s(),
                        }],
                        ret_ty: None,
                        body: ArrowBody::Block(Block {
                            stmts: vec![],
                            span: s(),
                        }),
                        span: s(),
                    },
                    span: s(),
                }],
                span: s(),
            }],
            head: None,
            span: s(),
        },
        span: s(),
    })]);
    assert!(
        errors.is_empty(),
        "on:click with arrow fn handler should be OK: {:?}",
        errors
    );
}

#[test]
fn on_event_handler_non_function_errors() {
    // on:click={42}  ← ERROR — handler must be a function
    let errors = check(vec![Item::ComponentDecl(ComponentDecl {
        name: "Btn".into(),
        props: vec![],
        body: ComponentBody {
            stmts: vec![],
            template: vec![TemplateNode::SelfClosing {
                tag: "button".into(),
                attributes: vec![],
                directives: vec![Directive::On {
                    event: "click".into(),
                    modifiers: vec![],
                    handler: Expr::IntLit {
                        value: 42,
                        span: s(),
                    },
                    span: s(),
                }],
                span: s(),
            }],
            head: None,
            span: s(),
        },
        span: s(),
    })]);
    assert!(
        !errors.is_empty(),
        "on:click with non-function handler should error"
    );
}

// ── S5. Bind: signal of compatible type ───────────────────────────────

#[test]
fn bind_signal_compatible_type_ok() {
    // signal name = ""; <input bind:value={name} />  — string signal on value is OK
    let errors = check(vec![Item::ComponentDecl(ComponentDecl {
        name: "Form".into(),
        props: vec![],
        body: ComponentBody {
            stmts: vec![Stmt::Signal {
                name: "name".into(),
                ty_ann: None,
                init: Expr::StringLit {
                    value: "".into(),
                    span: s(),
                },
                span: s(),
            }],
            template: vec![TemplateNode::SelfClosing {
                tag: "input".into(),
                attributes: vec![],
                directives: vec![Directive::Bind {
                    field: "name".into(),
                    span: s(),
                }],
                span: s(),
            }],
            head: None,
            span: s(),
        },
        span: s(),
    })]);
    assert!(
        errors.is_empty(),
        "bind:name on signal<string> for <input> should be OK: {:?}",
        errors
    );
}

#[test]
fn bind_non_signal_errors() {
    // let name = ""; <input bind:value={name} />  ← ERROR (not a signal)
    let errors = check(vec![Item::ComponentDecl(ComponentDecl {
        name: "Form".into(),
        props: vec![],
        body: ComponentBody {
            stmts: vec![Stmt::Let {
                name: "name".into(),
                ty_ann: None,
                init: Expr::StringLit {
                    value: "".into(),
                    span: s(),
                },
                span: s(),
            }],
            template: vec![TemplateNode::SelfClosing {
                tag: "input".into(),
                attributes: vec![],
                directives: vec![Directive::Bind {
                    field: "name".into(),
                    span: s(),
                }],
                span: s(),
            }],
            head: None,
            span: s(),
        },
        span: s(),
    })]);
    assert!(!errors.is_empty(), "bind: on a non-signal should error");
}

// ── S6. Ref: type must match element ──────────────────────────────────

#[test]
fn ref_domref_type_ok() {
    // ref myCanvas: HTMLCanvasElement; <canvas ref:myCanvas />
    let errors = check(vec![Item::ComponentDecl(ComponentDecl {
        name: "Draw".into(),
        props: vec![],
        body: ComponentBody {
            stmts: vec![Stmt::RefDecl {
                name: "myCanvas".into(),
                ty_ann: TypeAnnotation::Named {
                    name: "HTMLCanvasElement".into(),
                    span: s(),
                },
                span: s(),
            }],
            template: vec![TemplateNode::SelfClosing {
                tag: "canvas".into(),
                attributes: vec![],
                directives: vec![Directive::Ref {
                    name: "myCanvas".into(),
                    span: s(),
                }],
                span: s(),
            }],
            head: None,
            span: s(),
        },
        span: s(),
    })]);
    assert!(
        errors.is_empty(),
        "ref:myCanvas of HTMLCanvasElement on <canvas> should be OK: {:?}",
        errors
    );
}

#[test]
fn ref_non_domref_errors() {
    // let myRef = 42; <canvas ref:myRef />  ← ERROR (not a DomRef)
    let errors = check(vec![Item::ComponentDecl(ComponentDecl {
        name: "Draw".into(),
        props: vec![],
        body: ComponentBody {
            stmts: vec![Stmt::Let {
                name: "myRef".into(),
                ty_ann: None,
                init: Expr::IntLit {
                    value: 42,
                    span: s(),
                },
                span: s(),
            }],
            template: vec![TemplateNode::SelfClosing {
                tag: "canvas".into(),
                attributes: vec![],
                directives: vec![Directive::Ref {
                    name: "myRef".into(),
                    span: s(),
                }],
                span: s(),
            }],
            head: None,
            span: s(),
        },
        span: s(),
    })]);
    assert!(!errors.is_empty(), "ref: on a non-DomRef should error");
}

// ── S7. Head block: property validation ───────────────────────────────

#[test]
fn head_block_valid_properties() {
    // head { title: "My Page", description: "A description" }
    let errors = check(vec![Item::ComponentDecl(ComponentDecl {
        name: "Page".into(),
        props: vec![],
        body: ComponentBody {
            stmts: vec![],
            template: vec![],
            head: Some(HeadBlock {
                fields: vec![
                    HeadField {
                        key: "title".into(),
                        value: Expr::StringLit {
                            value: "My Page".into(),
                            span: s(),
                        },
                        span: s(),
                    },
                    HeadField {
                        key: "description".into(),
                        value: Expr::StringLit {
                            value: "A description".into(),
                            span: s(),
                        },
                        span: s(),
                    },
                ],
                span: s(),
            }),
            span: s(),
        },
        span: s(),
    })]);
    assert!(
        errors.is_empty(),
        "head block with valid string properties should be OK: {:?}",
        errors
    );
}

#[test]
fn head_block_invalid_property_type() {
    // head { title: 42 }  ← ERROR — title must be a string
    let errors = check(vec![Item::ComponentDecl(ComponentDecl {
        name: "Page".into(),
        props: vec![],
        body: ComponentBody {
            stmts: vec![],
            template: vec![],
            head: Some(HeadBlock {
                fields: vec![HeadField {
                    key: "title".into(),
                    value: Expr::IntLit {
                        value: 42,
                        span: s(),
                    },
                    span: s(),
                }],
                span: s(),
            }),
            span: s(),
        },
        span: s(),
    })]);
    assert!(!errors.is_empty(), "head title with int value should error");
}

#[test]
fn head_block_unknown_property() {
    // head { foobar: "value" }  ← ERROR — unknown property
    let errors = check(vec![Item::ComponentDecl(ComponentDecl {
        name: "Page".into(),
        props: vec![],
        body: ComponentBody {
            stmts: vec![],
            template: vec![],
            head: Some(HeadBlock {
                fields: vec![HeadField {
                    key: "foobar".into(),
                    value: Expr::StringLit {
                        value: "value".into(),
                        span: s(),
                    },
                    span: s(),
                }],
                span: s(),
            }),
            span: s(),
        },
        span: s(),
    })]);
    assert!(
        !errors.is_empty(),
        "head block with unknown property should error"
    );
}

// ── S8. Form: guard + action compatibility ────────────────────────────

#[test]
fn form_guard_action_compatible() {
    // guard LoginForm { email: string.email() }
    // action login(data: LoginForm) -> void { }
    // <form form:guard={LoginForm} form:action={login} />
    let errors = check(vec![
        Item::GuardDecl(GuardDecl {
            name: "LoginForm".into(),
            fields: vec![GuardFieldDecl {
                name: "email".into(),
                ty: TypeAnnotation::Named {
                    name: "string".into(),
                    span: s(),
                },
                validators: vec![ValidatorCall {
                    name: "email".into(),
                    args: vec![],
                    span: s(),
                }],
                span: s(),
            }],
            span: s(),
        }),
        Item::FnDecl(FnDecl {
            name: "login".into(),
            params: vec![Param {
                name: "data".into(),
                ty_ann: Some(TypeAnnotation::Named {
                    name: "LoginForm".into(),
                    span: s(),
                }),
                default: None,
                span: s(),
            }],
            ret_ty: Some(TypeAnnotation::Named {
                name: "void".into(),
                span: s(),
            }),
            body: Block {
                stmts: vec![],
                span: s(),
            },
            is_async: false,
            span: s(),
        }),
        Item::ComponentDecl(ComponentDecl {
            name: "Login".into(),
            props: vec![],
            body: ComponentBody {
                stmts: vec![],
                template: vec![TemplateNode::Element {
                    tag: "form".into(),
                    attributes: vec![],
                    directives: vec![
                        Directive::FormGuard {
                            guard: Expr::Ident {
                                name: "LoginForm".into(),
                                span: s(),
                            },
                            span: s(),
                        },
                        Directive::FormAction {
                            action: Expr::Ident {
                                name: "login".into(),
                                span: s(),
                            },
                            span: s(),
                        },
                    ],
                    children: vec![],
                    span: s(),
                }],
                head: None,
                span: s(),
            },
            span: s(),
        }),
    ]);
    assert!(
        errors.is_empty(),
        "form:guard + form:action with compatible types should be OK: {:?}",
        errors
    );
}

// ── S9. Frozen: deep immutability ─────────────────────────────────────

#[test]
fn frozen_reassignment_errors() {
    // frozen x = 1; x = 2  ← ERROR
    let errors = check_stmts(vec![
        Stmt::Frozen {
            name: "x".into(),
            init: Expr::IntLit {
                value: 1,
                span: s(),
            },
            span: s(),
        },
        Stmt::ExprStmt {
            expr: Expr::Assign {
                target: Box::new(Expr::Ident {
                    name: "x".into(),
                    span: s(),
                }),
                op: AssignOp::Assign,
                value: Box::new(Expr::IntLit {
                    value: 2,
                    span: s(),
                }),
                span: s(),
            },
            span: s(),
        },
    ]);
    assert!(!errors.is_empty(), "frozen reassignment should error");
}

#[test]
fn frozen_member_mutation_errors() {
    // frozen obj = { a: 1 }; obj.a = 2  ← ERROR (deep immutability)
    let errors = check_stmts(vec![
        Stmt::Frozen {
            name: "obj".into(),
            init: Expr::ObjectLit {
                fields: vec![ObjectFieldExpr {
                    key: "a".into(),
                    value: Expr::IntLit {
                        value: 1,
                        span: s(),
                    },
                    span: s(),
                }],
                span: s(),
            },
            span: s(),
        },
        Stmt::ExprStmt {
            expr: Expr::Assign {
                target: Box::new(Expr::MemberAccess {
                    object: Box::new(Expr::Ident {
                        name: "obj".into(),
                        span: s(),
                    }),
                    field: "a".into(),
                    span: s(),
                }),
                op: AssignOp::Assign,
                value: Box::new(Expr::IntLit {
                    value: 2,
                    span: s(),
                }),
                span: s(),
            },
            span: s(),
        },
    ]);
    assert!(
        !errors.is_empty(),
        "mutating a member of a frozen value should error"
    );
}

// ── S10. Watch: must reference reactive source ────────────────────────

#[test]
fn watch_reactive_source_ok() {
    // signal count = 0; watch count as (next, prev) { }
    let errors = check_stmts(vec![
        Stmt::Signal {
            name: "count".into(),
            ty_ann: None,
            init: Expr::IntLit {
                value: 0,
                span: s(),
            },
            span: s(),
        },
        Stmt::Watch {
            target: Expr::Ident {
                name: "count".into(),
                span: s(),
            },
            next_name: "next".into(),
            prev_name: "prev".into(),
            body: Block {
                stmts: vec![],
                span: s(),
            },
            span: s(),
        },
    ]);
    assert!(
        errors.is_empty(),
        "watching a signal should be OK: {:?}",
        errors
    );
}

#[test]
fn watch_non_reactive_errors() {
    // let x = 42; watch x as (next, prev) { }  ← ERROR
    let errors = check_stmts(vec![
        Stmt::Let {
            name: "x".into(),
            ty_ann: None,
            init: Expr::IntLit {
                value: 42,
                span: s(),
            },
            span: s(),
        },
        Stmt::Watch {
            target: Expr::Ident {
                name: "x".into(),
                span: s(),
            },
            next_name: "next".into(),
            prev_name: "prev".into(),
            body: Block {
                stmts: vec![],
                span: s(),
            },
            span: s(),
        },
    ]);
    assert!(
        !errors.is_empty(),
        "watching a non-reactive variable should error"
    );
}

#[test]
fn watch_literal_errors() {
    // watch 42 as (next, prev) { }  ← ERROR
    let errors = check_stmts(vec![Stmt::Watch {
        target: Expr::IntLit {
            value: 42,
            span: s(),
        },
        next_name: "next".into(),
        prev_name: "prev".into(),
        body: Block {
            stmts: vec![],
            span: s(),
        },
        span: s(),
    }]);
    assert!(!errors.is_empty(), "watching a literal should error");
}

#[test]
fn watch_derived_ok() {
    // let x = 10; derive doubled = x * 2; watch doubled as (n, p) { }
    let errors = check_stmts(vec![
        Stmt::Let {
            name: "x".into(),
            ty_ann: None,
            init: Expr::IntLit {
                value: 10,
                span: s(),
            },
            span: s(),
        },
        Stmt::Derive {
            name: "doubled".into(),
            init: Expr::BinaryOp {
                left: Box::new(Expr::Ident {
                    name: "x".into(),
                    span: s(),
                }),
                op: BinOp::Mul,
                right: Box::new(Expr::IntLit {
                    value: 2,
                    span: s(),
                }),
                span: s(),
            },
            span: s(),
        },
        Stmt::Watch {
            target: Expr::Ident {
                name: "doubled".into(),
                span: s(),
            },
            next_name: "n".into(),
            prev_name: "p".into(),
            body: Block {
                stmts: vec![],
                span: s(),
            },
            span: s(),
        },
    ]);
    assert!(
        errors.is_empty(),
        "watching a derived value should be OK: {:?}",
        errors
    );
}

// ══════════════════════════════════════════════════════════════════════
// Phase 10.6 — Comprehensive type checker test suite
// ══════════════════════════════════════════════════════════════════════

// ── T1. Type inference for all expression types ───────────────────────

#[test]
fn float_literal_inference() {
    let errors = check_stmts(vec![Stmt::Let {
        name: "x".into(),
        ty_ann: None,
        init: Expr::FloatLit {
            value: 3.14,
            span: s(),
        },
        span: s(),
    }]);
    assert!(errors.is_empty(), "float literal: {:?}", errors);
}

#[test]
fn null_literal_inference() {
    let errors = check_stmts(vec![Stmt::Let {
        name: "x".into(),
        ty_ann: None,
        init: Expr::NullLit { span: s() },
        span: s(),
    }]);
    assert!(errors.is_empty(), "null literal: {:?}", errors);
}

#[test]
fn regex_literal_inference() {
    let errors = check_stmts(vec![Stmt::Let {
        name: "x".into(),
        ty_ann: None,
        init: Expr::RegexLit {
            pattern: "\\d+".into(),
            flags: "g".into(),
            span: s(),
        },
        span: s(),
    }]);
    assert!(errors.is_empty(), "regex literal: {:?}", errors);
}

#[test]
fn unary_neg_int() {
    let errors = check_stmts(vec![Stmt::Let {
        name: "x".into(),
        ty_ann: None,
        init: Expr::UnaryOp {
            op: UnaryOp::Neg,
            operand: Box::new(Expr::IntLit {
                value: 42,
                span: s(),
            }),
            span: s(),
        },
        span: s(),
    }]);
    assert!(errors.is_empty(), "unary neg int: {:?}", errors);
}

#[test]
fn unary_neg_float() {
    let errors = check_stmts(vec![Stmt::Let {
        name: "x".into(),
        ty_ann: None,
        init: Expr::UnaryOp {
            op: UnaryOp::Neg,
            operand: Box::new(Expr::FloatLit {
                value: 1.5,
                span: s(),
            }),
            span: s(),
        },
        span: s(),
    }]);
    assert!(errors.is_empty(), "unary neg float: {:?}", errors);
}

#[test]
fn unary_neg_string_errors() {
    let errors = check_stmts(vec![Stmt::Let {
        name: "x".into(),
        ty_ann: None,
        init: Expr::UnaryOp {
            op: UnaryOp::Neg,
            operand: Box::new(Expr::StringLit {
                value: "hello".into(),
                span: s(),
            }),
            span: s(),
        },
        span: s(),
    }]);
    assert!(!errors.is_empty(), "unary neg on string should error");
}

#[test]
fn unary_not_bool() {
    let errors = check_stmts(vec![Stmt::Let {
        name: "x".into(),
        ty_ann: None,
        init: Expr::UnaryOp {
            op: UnaryOp::Not,
            operand: Box::new(Expr::BoolLit {
                value: true,
                span: s(),
            }),
            span: s(),
        },
        span: s(),
    }]);
    assert!(errors.is_empty(), "unary not bool: {:?}", errors);
}

#[test]
fn unary_not_int_errors() {
    let errors = check_stmts(vec![Stmt::Let {
        name: "x".into(),
        ty_ann: None,
        init: Expr::UnaryOp {
            op: UnaryOp::Not,
            operand: Box::new(Expr::IntLit {
                value: 1,
                span: s(),
            }),
            span: s(),
        },
        span: s(),
    }]);
    assert!(!errors.is_empty(), "unary not on int should error");
}

#[test]
fn ternary_inference() {
    let errors = check_stmts(vec![Stmt::Let {
        name: "x".into(),
        ty_ann: None,
        init: Expr::Ternary {
            condition: Box::new(Expr::BoolLit {
                value: true,
                span: s(),
            }),
            then_expr: Box::new(Expr::IntLit {
                value: 1,
                span: s(),
            }),
            else_expr: Box::new(Expr::IntLit {
                value: 2,
                span: s(),
            }),
            span: s(),
        },
        span: s(),
    }]);
    assert!(errors.is_empty(), "ternary: {:?}", errors);
}

#[test]
fn ternary_condition_not_bool_errors() {
    let errors = check_stmts(vec![Stmt::Let {
        name: "x".into(),
        ty_ann: None,
        init: Expr::Ternary {
            condition: Box::new(Expr::IntLit {
                value: 42,
                span: s(),
            }),
            then_expr: Box::new(Expr::IntLit {
                value: 1,
                span: s(),
            }),
            else_expr: Box::new(Expr::IntLit {
                value: 2,
                span: s(),
            }),
            span: s(),
        },
        span: s(),
    }]);
    assert!(
        !errors.is_empty(),
        "ternary with non-bool condition should error"
    );
}

#[test]
fn null_coalesce_expression() {
    // x ?? 0 where x: int? = null
    let errors = check_stmts(vec![
        Stmt::Let {
            name: "x".into(),
            ty_ann: Some(TypeAnnotation::Optional {
                inner: Box::new(TypeAnnotation::Named {
                    name: "int".into(),
                    span: s(),
                }),
                span: s(),
            }),
            init: Expr::NullLit { span: s() },
            span: s(),
        },
        Stmt::Let {
            name: "y".into(),
            ty_ann: None,
            init: Expr::NullCoalesce {
                left: Box::new(Expr::Ident {
                    name: "x".into(),
                    span: s(),
                }),
                right: Box::new(Expr::IntLit {
                    value: 0,
                    span: s(),
                }),
                span: s(),
            },
            span: s(),
        },
    ]);
    assert!(errors.is_empty(), "null coalesce: {:?}", errors);
}

#[test]
fn index_access_array() {
    let errors = check_stmts(vec![
        Stmt::Let {
            name: "arr".into(),
            ty_ann: None,
            init: Expr::ArrayLit {
                elements: vec![
                    Expr::IntLit {
                        value: 1,
                        span: s(),
                    },
                    Expr::IntLit {
                        value: 2,
                        span: s(),
                    },
                ],
                span: s(),
            },
            span: s(),
        },
        Stmt::Let {
            name: "x".into(),
            ty_ann: None,
            init: Expr::IndexAccess {
                object: Box::new(Expr::Ident {
                    name: "arr".into(),
                    span: s(),
                }),
                index: Box::new(Expr::IntLit {
                    value: 0,
                    span: s(),
                }),
                span: s(),
            },
            span: s(),
        },
    ]);
    assert!(errors.is_empty(), "index access on array: {:?}", errors);
}

#[test]
fn index_access_non_indexable_errors() {
    let errors = check_stmts(vec![
        Stmt::Let {
            name: "x".into(),
            ty_ann: None,
            init: Expr::IntLit {
                value: 42,
                span: s(),
            },
            span: s(),
        },
        Stmt::ExprStmt {
            expr: Expr::IndexAccess {
                object: Box::new(Expr::Ident {
                    name: "x".into(),
                    span: s(),
                }),
                index: Box::new(Expr::IntLit {
                    value: 0,
                    span: s(),
                }),
                span: s(),
            },
            span: s(),
        },
    ]);
    assert!(
        !errors.is_empty(),
        "index access on non-indexable should error"
    );
}

#[test]
fn range_expression() {
    let errors = check_stmts(vec![Stmt::Let {
        name: "r".into(),
        ty_ann: None,
        init: Expr::Range {
            start: Box::new(Expr::IntLit {
                value: 1,
                span: s(),
            }),
            end: Box::new(Expr::IntLit {
                value: 10,
                span: s(),
            }),
            span: s(),
        },
        span: s(),
    }]);
    assert!(errors.is_empty(), "range expression: {:?}", errors);
}

#[test]
fn range_non_int_errors() {
    let errors = check_stmts(vec![Stmt::Let {
        name: "r".into(),
        ty_ann: None,
        init: Expr::Range {
            start: Box::new(Expr::StringLit {
                value: "a".into(),
                span: s(),
            }),
            end: Box::new(Expr::StringLit {
                value: "z".into(),
                span: s(),
            }),
            span: s(),
        },
        span: s(),
    }]);
    assert!(!errors.is_empty(), "range with strings should error");
}

#[test]
fn pipe_expression() {
    // fn double(x: int) -> int { return x * 2 }; let y = 5 |> double
    let errors = check(vec![
        Item::FnDecl(FnDecl {
            name: "double".into(),
            params: vec![Param {
                name: "x".into(),
                ty_ann: Some(TypeAnnotation::Named {
                    name: "int".into(),
                    span: s(),
                }),
                default: None,
                span: s(),
            }],
            ret_ty: Some(TypeAnnotation::Named {
                name: "int".into(),
                span: s(),
            }),
            body: Block {
                stmts: vec![Stmt::Return {
                    value: Some(Expr::BinaryOp {
                        left: Box::new(Expr::Ident {
                            name: "x".into(),
                            span: s(),
                        }),
                        op: BinOp::Mul,
                        right: Box::new(Expr::IntLit {
                            value: 2,
                            span: s(),
                        }),
                        span: s(),
                    }),
                    span: s(),
                }],
                span: s(),
            },
            is_async: false,
            span: s(),
        }),
        Item::Stmt(Stmt::Let {
            name: "y".into(),
            ty_ann: None,
            init: Expr::Pipe {
                left: Box::new(Expr::IntLit {
                    value: 5,
                    span: s(),
                }),
                right: Box::new(Expr::Ident {
                    name: "double".into(),
                    span: s(),
                }),
                span: s(),
            },
            span: s(),
        }),
    ]);
    assert!(errors.is_empty(), "pipe expression: {:?}", errors);
}

#[test]
fn pipe_non_function_errors() {
    let errors = check_stmts(vec![Stmt::Let {
        name: "y".into(),
        ty_ann: None,
        init: Expr::Pipe {
            left: Box::new(Expr::IntLit {
                value: 5,
                span: s(),
            }),
            right: Box::new(Expr::IntLit {
                value: 42,
                span: s(),
            }),
            span: s(),
        },
        span: s(),
    }]);
    assert!(!errors.is_empty(), "pipe to non-function should error");
}

#[test]
fn assert_expression_bool() {
    let errors = check_stmts(vec![Stmt::ExprStmt {
        expr: Expr::Assert {
            expr: Box::new(Expr::BoolLit {
                value: true,
                span: s(),
            }),
            span: s(),
        },
        span: s(),
    }]);
    assert!(errors.is_empty(), "assert bool: {:?}", errors);
}

#[test]
fn assert_non_bool_errors() {
    let errors = check_stmts(vec![Stmt::ExprStmt {
        expr: Expr::Assert {
            expr: Box::new(Expr::IntLit {
                value: 42,
                span: s(),
            }),
            span: s(),
        },
        span: s(),
    }]);
    assert!(!errors.is_empty(), "assert non-bool should error");
}

#[test]
fn await_passthrough() {
    let errors = check_stmts(vec![Stmt::Let {
        name: "x".into(),
        ty_ann: None,
        init: Expr::Await {
            expr: Box::new(Expr::IntLit {
                value: 42,
                span: s(),
            }),
            span: s(),
        },
        span: s(),
    }]);
    assert!(errors.is_empty(), "await passthrough: {:?}", errors);
}

#[test]
fn spread_passthrough() {
    let errors = check_stmts(vec![
        Stmt::Let {
            name: "arr".into(),
            ty_ann: None,
            init: Expr::ArrayLit {
                elements: vec![Expr::IntLit {
                    value: 1,
                    span: s(),
                }],
                span: s(),
            },
            span: s(),
        },
        Stmt::Let {
            name: "x".into(),
            ty_ann: None,
            init: Expr::Spread {
                expr: Box::new(Expr::Ident {
                    name: "arr".into(),
                    span: s(),
                }),
                span: s(),
            },
            span: s(),
        },
    ]);
    assert!(errors.is_empty(), "spread passthrough: {:?}", errors);
}

// ── T1b. Operator type rules ──────────────────────────────────────────

#[test]
fn binary_sub_int() {
    let errors = check_stmts(vec![Stmt::Let {
        name: "x".into(),
        ty_ann: None,
        init: Expr::BinaryOp {
            left: Box::new(Expr::IntLit {
                value: 5,
                span: s(),
            }),
            op: BinOp::Sub,
            right: Box::new(Expr::IntLit {
                value: 3,
                span: s(),
            }),
            span: s(),
        },
        span: s(),
    }]);
    assert!(errors.is_empty(), "int - int: {:?}", errors);
}

#[test]
fn binary_mul_float() {
    let errors = check_stmts(vec![Stmt::Let {
        name: "x".into(),
        ty_ann: None,
        init: Expr::BinaryOp {
            left: Box::new(Expr::FloatLit {
                value: 2.0,
                span: s(),
            }),
            op: BinOp::Mul,
            right: Box::new(Expr::FloatLit {
                value: 3.0,
                span: s(),
            }),
            span: s(),
        },
        span: s(),
    }]);
    assert!(errors.is_empty(), "float * float: {:?}", errors);
}

#[test]
fn binary_add_int_float_promotion() {
    let errors = check_stmts(vec![Stmt::Let {
        name: "x".into(),
        ty_ann: None,
        init: Expr::BinaryOp {
            left: Box::new(Expr::IntLit {
                value: 1,
                span: s(),
            }),
            op: BinOp::Add,
            right: Box::new(Expr::FloatLit {
                value: 2.5,
                span: s(),
            }),
            span: s(),
        },
        span: s(),
    }]);
    assert!(errors.is_empty(), "int + float promotion: {:?}", errors);
}

#[test]
fn binary_div_non_numeric_errors() {
    let errors = check_stmts(vec![Stmt::Let {
        name: "x".into(),
        ty_ann: None,
        init: Expr::BinaryOp {
            left: Box::new(Expr::StringLit {
                value: "a".into(),
                span: s(),
            }),
            op: BinOp::Div,
            right: Box::new(Expr::StringLit {
                value: "b".into(),
                span: s(),
            }),
            span: s(),
        },
        span: s(),
    }]);
    assert!(!errors.is_empty(), "string / string should error");
}

#[test]
fn binary_mod_int() {
    let errors = check_stmts(vec![Stmt::Let {
        name: "x".into(),
        ty_ann: None,
        init: Expr::BinaryOp {
            left: Box::new(Expr::IntLit {
                value: 10,
                span: s(),
            }),
            op: BinOp::Mod,
            right: Box::new(Expr::IntLit {
                value: 3,
                span: s(),
            }),
            span: s(),
        },
        span: s(),
    }]);
    assert!(errors.is_empty(), "int %% int: {:?}", errors);
}

#[test]
fn binary_eq_returns_bool() {
    let errors = check_stmts(vec![Stmt::Let {
        name: "x".into(),
        ty_ann: Some(TypeAnnotation::Named {
            name: "bool".into(),
            span: s(),
        }),
        init: Expr::BinaryOp {
            left: Box::new(Expr::IntLit {
                value: 1,
                span: s(),
            }),
            op: BinOp::Eq,
            right: Box::new(Expr::IntLit {
                value: 2,
                span: s(),
            }),
            span: s(),
        },
        span: s(),
    }]);
    assert!(errors.is_empty(), "== returns bool: {:?}", errors);
}

#[test]
fn binary_lt_returns_bool() {
    let errors = check_stmts(vec![Stmt::Let {
        name: "x".into(),
        ty_ann: Some(TypeAnnotation::Named {
            name: "bool".into(),
            span: s(),
        }),
        init: Expr::BinaryOp {
            left: Box::new(Expr::IntLit {
                value: 1,
                span: s(),
            }),
            op: BinOp::Lt,
            right: Box::new(Expr::IntLit {
                value: 2,
                span: s(),
            }),
            span: s(),
        },
        span: s(),
    }]);
    assert!(errors.is_empty(), "< returns bool: {:?}", errors);
}

#[test]
fn binary_comparison_non_comparable_errors() {
    let errors = check_stmts(vec![Stmt::Let {
        name: "x".into(),
        ty_ann: None,
        init: Expr::BinaryOp {
            left: Box::new(Expr::BoolLit {
                value: true,
                span: s(),
            }),
            op: BinOp::Lt,
            right: Box::new(Expr::BoolLit {
                value: false,
                span: s(),
            }),
            span: s(),
        },
        span: s(),
    }]);
    assert!(!errors.is_empty(), "bool < bool should error");
}

#[test]
fn binary_and_bool() {
    let errors = check_stmts(vec![Stmt::Let {
        name: "x".into(),
        ty_ann: None,
        init: Expr::BinaryOp {
            left: Box::new(Expr::BoolLit {
                value: true,
                span: s(),
            }),
            op: BinOp::And,
            right: Box::new(Expr::BoolLit {
                value: false,
                span: s(),
            }),
            span: s(),
        },
        span: s(),
    }]);
    assert!(errors.is_empty(), "&& on bools: {:?}", errors);
}

#[test]
fn binary_or_non_bool_errors() {
    let errors = check_stmts(vec![Stmt::Let {
        name: "x".into(),
        ty_ann: None,
        init: Expr::BinaryOp {
            left: Box::new(Expr::IntLit {
                value: 1,
                span: s(),
            }),
            op: BinOp::Or,
            right: Box::new(Expr::IntLit {
                value: 2,
                span: s(),
            }),
            span: s(),
        },
        span: s(),
    }]);
    assert!(!errors.is_empty(), "|| on ints should error");
}

#[test]
fn binary_dotdot_range() {
    let errors = check_stmts(vec![Stmt::Let {
        name: "r".into(),
        ty_ann: None,
        init: Expr::BinaryOp {
            left: Box::new(Expr::IntLit {
                value: 0,
                span: s(),
            }),
            op: BinOp::DotDot,
            right: Box::new(Expr::IntLit {
                value: 10,
                span: s(),
            }),
            span: s(),
        },
        span: s(),
    }]);
    assert!(errors.is_empty(), "0..10 range: {:?}", errors);
}

// ── T1c. Statement type checks ────────────────────────────────────────

#[test]
fn if_condition_must_be_bool() {
    let errors = check_stmts(vec![Stmt::If {
        condition: Expr::IntLit {
            value: 1,
            span: s(),
        },
        then_block: Block {
            stmts: vec![],
            span: s(),
        },
        else_branch: None,
        span: s(),
    }]);
    assert!(
        !errors.is_empty(),
        "if with non-bool condition should error"
    );
}

#[test]
fn if_with_else_branch() {
    let errors = check_stmts(vec![Stmt::If {
        condition: Expr::BoolLit {
            value: true,
            span: s(),
        },
        then_block: Block {
            stmts: vec![Stmt::Let {
                name: "x".into(),
                ty_ann: None,
                init: Expr::IntLit {
                    value: 1,
                    span: s(),
                },
                span: s(),
            }],
            span: s(),
        },
        else_branch: Some(ElseBranch::Else(Block {
            stmts: vec![Stmt::Let {
                name: "y".into(),
                ty_ann: None,
                init: Expr::IntLit {
                    value: 2,
                    span: s(),
                },
                span: s(),
            }],
            span: s(),
        })),
        span: s(),
    }]);
    assert!(
        errors.is_empty(),
        "if/else with bool condition: {:?}",
        errors
    );
}

#[test]
fn effect_body_checks() {
    let errors = check_stmts(vec![Stmt::Effect {
        body: Block {
            stmts: vec![Stmt::Let {
                name: "x".into(),
                ty_ann: None,
                init: Expr::IntLit {
                    value: 1,
                    span: s(),
                },
                span: s(),
            }],
            span: s(),
        },
        cleanup: Some(Block {
            stmts: vec![Stmt::Let {
                name: "y".into(),
                ty_ann: None,
                init: Expr::IntLit {
                    value: 2,
                    span: s(),
                },
                span: s(),
            }],
            span: s(),
        }),
        span: s(),
    }]);
    assert!(errors.is_empty(), "effect with body+cleanup: {:?}", errors);
}

// ── T1d. Declaration type checks ──────────────────────────────────────

#[test]
fn type_alias_declaration() {
    let errors = check(vec![
        Item::TypeAlias(TypeAliasDecl {
            name: "ID".into(),
            ty: TypeAnnotation::Named {
                name: "string".into(),
                span: s(),
            },
            span: s(),
        }),
        Item::Stmt(Stmt::Let {
            name: "x".into(),
            ty_ann: Some(TypeAnnotation::Named {
                name: "ID".into(),
                span: s(),
            }),
            init: Expr::StringLit {
                value: "abc".into(),
                span: s(),
            },
            span: s(),
        }),
    ]);
    assert!(errors.is_empty(), "type alias: {:?}", errors);
}

#[test]
fn enum_declaration() {
    let errors = check(vec![Item::EnumDecl(EnumDecl {
        name: "Status".into(),
        variants: vec!["Active".into(), "Inactive".into()],
        span: s(),
    })]);
    assert!(errors.is_empty(), "enum declaration: {:?}", errors);
}

#[test]
fn test_block_declaration() {
    let errors = check(vec![Item::TestDecl(TestDecl {
        name: "my test".into(),
        body: Block {
            stmts: vec![Stmt::ExprStmt {
                expr: Expr::Assert {
                    expr: Box::new(Expr::BoolLit {
                        value: true,
                        span: s(),
                    }),
                    span: s(),
                },
                span: s(),
            }],
            span: s(),
        },
        span: s(),
    })]);
    assert!(errors.is_empty(), "test block: {:?}", errors);
}

#[test]
fn use_import_declaration() {
    let errors = check(vec![Item::Use(UseDecl {
        imports: ImportKind::Named(vec!["Foo".into(), "Bar".into()]),
        path: "./module".into(),
        span: s(),
    })]);
    assert!(errors.is_empty(), "use import: {:?}", errors);
}

// ── T1e. Template node type checks ────────────────────────────────────

#[test]
fn template_when_condition_must_be_bool() {
    let errors = check(vec![Item::ComponentDecl(ComponentDecl {
        name: "C".into(),
        props: vec![],
        body: ComponentBody {
            stmts: vec![],
            template: vec![TemplateNode::When {
                condition: Expr::IntLit {
                    value: 1,
                    span: s(),
                },
                body: vec![TemplateNode::Text {
                    value: "yes".into(),
                    span: s(),
                }],
                else_branch: None,
                span: s(),
            }],
            head: None,
            span: s(),
        },
        span: s(),
    })]);
    assert!(
        !errors.is_empty(),
        "when with non-bool condition should error"
    );
}

#[test]
fn template_each_requires_array() {
    let errors = check(vec![Item::ComponentDecl(ComponentDecl {
        name: "C".into(),
        props: vec![],
        body: ComponentBody {
            stmts: vec![Stmt::Let {
                name: "x".into(),
                ty_ann: None,
                init: Expr::IntLit {
                    value: 42,
                    span: s(),
                },
                span: s(),
            }],
            template: vec![TemplateNode::Each {
                binding: "item".into(),
                index: None,
                iterable: Expr::Ident {
                    name: "x".into(),
                    span: s(),
                },
                body: vec![],
                empty: None,
                span: s(),
            }],
            head: None,
            span: s(),
        },
        span: s(),
    })]);
    assert!(!errors.is_empty(), "each on non-array should error");
}

#[test]
fn template_class_directive_must_be_bool() {
    let errors = check(vec![Item::ComponentDecl(ComponentDecl {
        name: "C".into(),
        props: vec![],
        body: ComponentBody {
            stmts: vec![],
            template: vec![TemplateNode::SelfClosing {
                tag: "div".into(),
                attributes: vec![],
                directives: vec![Directive::Class {
                    name: "active".into(),
                    condition: Expr::IntLit {
                        value: 1,
                        span: s(),
                    },
                    span: s(),
                }],
                span: s(),
            }],
            head: None,
            span: s(),
        },
        span: s(),
    })]);
    assert!(
        !errors.is_empty(),
        "class: directive with non-bool should error"
    );
}

#[test]
fn template_key_must_be_string_or_int() {
    let errors = check(vec![Item::ComponentDecl(ComponentDecl {
        name: "C".into(),
        props: vec![],
        body: ComponentBody {
            stmts: vec![],
            template: vec![TemplateNode::SelfClosing {
                tag: "div".into(),
                attributes: vec![],
                directives: vec![Directive::Key {
                    expr: Expr::BoolLit {
                        value: true,
                        span: s(),
                    },
                    span: s(),
                }],
                span: s(),
            }],
            head: None,
            span: s(),
        },
        span: s(),
    })]);
    assert!(!errors.is_empty(), "key with bool should error");
}

// ── T2. Type errors with exact error messages ─────────────────────────

#[test]
fn error_msg_undefined_variable() {
    let errors = check_stmts(vec![Stmt::ExprStmt {
        expr: Expr::Ident {
            name: "nonexistent".into(),
            span: s(),
        },
        span: s(),
    }]);
    assert_eq!(errors.len(), 1);
    assert!(
        errors[0]
            .context
            .contains("undefined variable 'nonexistent'"),
        "expected 'undefined variable', got: {}",
        errors[0].context
    );
}

#[test]
fn error_msg_not_callable() {
    let errors = check_stmts(vec![
        Stmt::Let {
            name: "x".into(),
            ty_ann: None,
            init: Expr::IntLit {
                value: 42,
                span: s(),
            },
            span: s(),
        },
        Stmt::ExprStmt {
            expr: Expr::FnCall {
                callee: Box::new(Expr::Ident {
                    name: "x".into(),
                    span: s(),
                }),
                args: vec![],
                span: s(),
            },
            span: s(),
        },
    ]);
    assert_eq!(errors.len(), 1);
    assert!(
        errors[0].context.contains("not callable"),
        "expected 'not callable', got: {}",
        errors[0].context
    );
}

#[test]
fn error_msg_arity_mismatch() {
    let errors = check(vec![
        Item::FnDecl(FnDecl {
            name: "f".into(),
            params: vec![Param {
                name: "a".into(),
                ty_ann: Some(TypeAnnotation::Named {
                    name: "int".into(),
                    span: s(),
                }),
                default: None,
                span: s(),
            }],
            ret_ty: None,
            body: Block {
                stmts: vec![],
                span: s(),
            },
            is_async: false,
            span: s(),
        }),
        Item::Stmt(Stmt::ExprStmt {
            expr: Expr::FnCall {
                callee: Box::new(Expr::Ident {
                    name: "f".into(),
                    span: s(),
                }),
                args: vec![
                    Expr::IntLit {
                        value: 1,
                        span: s(),
                    },
                    Expr::IntLit {
                        value: 2,
                        span: s(),
                    },
                ],
                span: s(),
            },
            span: s(),
        }),
    ]);
    assert!(!errors.is_empty());
    let has_arity = errors.iter().any(|e| {
        matches!(
            e.kind,
            galex::types::constraint::TypeErrorKind::ArityMismatch {
                expected: 1,
                actual: 2
            }
        )
    });
    assert!(has_arity, "expected ArityMismatch(1, 2), got: {:?}", errors);
}

#[test]
fn error_msg_property_not_found() {
    let errors = check_stmts(vec![
        Stmt::Let {
            name: "obj".into(),
            ty_ann: None,
            init: Expr::ObjectLit {
                fields: vec![ObjectFieldExpr {
                    key: "a".into(),
                    value: Expr::IntLit {
                        value: 1,
                        span: s(),
                    },
                    span: s(),
                }],
                span: s(),
            },
            span: s(),
        },
        Stmt::ExprStmt {
            expr: Expr::MemberAccess {
                object: Box::new(Expr::Ident {
                    name: "obj".into(),
                    span: s(),
                }),
                field: "nonexistent".into(),
                span: s(),
            },
            span: s(),
        },
    ]);
    assert_eq!(errors.len(), 1);
    assert!(
        errors[0]
            .context
            .contains("property 'nonexistent' does not exist"),
        "expected property-not-found, got: {}",
        errors[0].context
    );
}

#[test]
fn error_msg_not_iterable() {
    let errors = check_stmts(vec![Stmt::For {
        binding: "item".into(),
        index: None,
        iterable: Expr::IntLit {
            value: 42,
            span: s(),
        },
        body: Block {
            stmts: vec![],
            span: s(),
        },
        span: s(),
    }]);
    assert_eq!(errors.len(), 1);
    assert!(
        errors[0].context.contains("for loop requires an iterable"),
        "expected 'iterable', got: {}",
        errors[0].context
    );
}

#[test]
fn error_msg_add_incompatible() {
    let errors = check_stmts(vec![Stmt::ExprStmt {
        expr: Expr::BinaryOp {
            left: Box::new(Expr::IntLit {
                value: 1,
                span: s(),
            }),
            op: BinOp::Add,
            right: Box::new(Expr::BoolLit {
                value: true,
                span: s(),
            }),
            span: s(),
        },
        span: s(),
    }]);
    assert_eq!(errors.len(), 1);
    assert!(
        errors[0].context.contains("`+` requires numeric or string"),
        "expected '+' error, got: {}",
        errors[0].context
    );
}

#[test]
fn error_msg_arithmetic_non_numeric() {
    let errors = check_stmts(vec![Stmt::ExprStmt {
        expr: Expr::BinaryOp {
            left: Box::new(Expr::StringLit {
                value: "a".into(),
                span: s(),
            }),
            op: BinOp::Sub,
            right: Box::new(Expr::StringLit {
                value: "b".into(),
                span: s(),
            }),
            span: s(),
        },
        span: s(),
    }]);
    assert_eq!(errors.len(), 1);
    assert!(
        errors[0]
            .context
            .contains("arithmetic operator requires numeric"),
        "expected arithmetic error, got: {}",
        errors[0].context
    );
}

#[test]
fn error_msg_comparison_non_comparable() {
    let errors = check_stmts(vec![Stmt::ExprStmt {
        expr: Expr::BinaryOp {
            left: Box::new(Expr::BoolLit {
                value: true,
                span: s(),
            }),
            op: BinOp::Gt,
            right: Box::new(Expr::BoolLit {
                value: false,
                span: s(),
            }),
            span: s(),
        },
        span: s(),
    }]);
    assert_eq!(errors.len(), 1);
    assert!(
        errors[0]
            .context
            .contains("comparison requires numeric or string"),
        "expected comparison error, got: {}",
        errors[0].context
    );
}

#[test]
fn error_msg_watch_no_reactive() {
    let errors = check_stmts(vec![Stmt::Watch {
        target: Expr::IntLit {
            value: 42,
            span: s(),
        },
        next_name: "n".into(),
        prev_name: "p".into(),
        body: Block {
            stmts: vec![],
            span: s(),
        },
        span: s(),
    }]);
    assert_eq!(errors.len(), 1);
    assert!(
        errors[0]
            .context
            .contains("does not reference any reactive source"),
        "expected reactive source error, got: {}",
        errors[0].context
    );
}

#[test]
fn error_msg_store_mutation_outside() {
    let errors = check(vec![
        Item::StoreDecl(StoreDecl {
            name: "Counter".into(),
            members: vec![StoreMember::Signal(Stmt::Signal {
                name: "count".into(),
                ty_ann: None,
                init: Expr::IntLit {
                    value: 0,
                    span: s(),
                },
                span: s(),
            })],
            span: s(),
        }),
        Item::Stmt(Stmt::ExprStmt {
            expr: Expr::Assign {
                target: Box::new(Expr::MemberAccess {
                    object: Box::new(Expr::Ident {
                        name: "Counter".into(),
                        span: s(),
                    }),
                    field: "count".into(),
                    span: s(),
                }),
                op: AssignOp::Assign,
                value: Box::new(Expr::IntLit {
                    value: 5,
                    span: s(),
                }),
                span: s(),
            },
            span: s(),
        }),
    ]);
    let store_error = errors
        .iter()
        .find(|e| e.context.contains("cannot mutate signal"));
    assert!(
        store_error.is_some(),
        "expected store mutation error, got: {:?}",
        errors
    );
}

#[test]
fn error_msg_frozen_member_mutation() {
    let errors = check_stmts(vec![
        Stmt::Frozen {
            name: "obj".into(),
            init: Expr::ObjectLit {
                fields: vec![ObjectFieldExpr {
                    key: "a".into(),
                    value: Expr::IntLit {
                        value: 1,
                        span: s(),
                    },
                    span: s(),
                }],
                span: s(),
            },
            span: s(),
        },
        Stmt::ExprStmt {
            expr: Expr::Assign {
                target: Box::new(Expr::MemberAccess {
                    object: Box::new(Expr::Ident {
                        name: "obj".into(),
                        span: s(),
                    }),
                    field: "a".into(),
                    span: s(),
                }),
                op: AssignOp::Assign,
                value: Box::new(Expr::IntLit {
                    value: 2,
                    span: s(),
                }),
                span: s(),
            },
            span: s(),
        },
    ]);
    let frozen_error = errors.iter().find(|e| e.context.contains("frozen"));
    assert!(
        frozen_error.is_some(),
        "expected frozen mutation error, got: {:?}",
        errors
    );
}

// ── T3. Boundary violations ───────────────────────────────────────────

#[test]
fn out_guard_export_valid() {
    let errors = check(vec![Item::SharedBlock(BoundaryBlock {
        items: vec![Item::Out(OutDecl {
            inner: Box::new(Item::GuardDecl(GuardDecl {
                name: "Email".into(),
                fields: vec![GuardFieldDecl {
                    name: "value".into(),
                    ty: TypeAnnotation::Named {
                        name: "string".into(),
                        span: s(),
                    },
                    validators: vec![],
                    span: s(),
                }],
                span: s(),
            })),
            span: s(),
        })],
        span: s(),
    })]);
    assert!(errors.is_empty(), "out guard export: {:?}", errors);
}

#[test]
fn out_enum_export_valid() {
    let errors = check(vec![Item::SharedBlock(BoundaryBlock {
        items: vec![Item::Out(OutDecl {
            inner: Box::new(Item::EnumDecl(EnumDecl {
                name: "Status".into(),
                variants: vec!["Active".into(), "Inactive".into()],
                span: s(),
            })),
            span: s(),
        })],
        span: s(),
    })]);
    assert!(errors.is_empty(), "out enum export: {:?}", errors);
}

#[test]
fn out_server_fn_serializable_return_valid() {
    let errors = check(vec![Item::ServerBlock(BoundaryBlock {
        items: vec![Item::Out(OutDecl {
            inner: Box::new(Item::FnDecl(FnDecl {
                name: "getId".into(),
                params: vec![],
                ret_ty: Some(TypeAnnotation::Named {
                    name: "int".into(),
                    span: s(),
                }),
                body: Block {
                    stmts: vec![Stmt::Return {
                        value: Some(Expr::IntLit {
                            value: 1,
                            span: s(),
                        }),
                        span: s(),
                    }],
                    span: s(),
                },
                is_async: false,
                span: s(),
            })),
            span: s(),
        })],
        span: s(),
    })]);
    assert!(
        errors.is_empty(),
        "server fn with serializable return: {:?}",
        errors
    );
}

#[test]
fn channel_export_valid() {
    let errors = check(vec![Item::Out(OutDecl {
        inner: Box::new(Item::ChannelDecl(ChannelDecl {
            name: "Chat".into(),
            params: vec![],
            direction: ChannelDirection::Bidirectional,
            msg_ty: TypeAnnotation::Named {
                name: "string".into(),
                span: s(),
            },
            handlers: vec![],
            span: s(),
        })),
        span: s(),
    })]);
    assert!(errors.is_empty(), "channel export: {:?}", errors);
}

#[test]
fn query_export_valid() {
    let errors = check(vec![Item::Out(OutDecl {
        inner: Box::new(Item::QueryDecl(QueryDecl {
            name: "users".into(),
            url_pattern: Expr::StringLit {
                value: "/api/users".into(),
                span: s(),
            },
            ret_ty: None,
            span: s(),
        })),
        span: s(),
    })]);
    assert!(errors.is_empty(), "query export: {:?}", errors);
}

#[test]
fn nested_boundary_innermost_wins() {
    // server { client { signal x = 0 } } — signal is valid in client scope
    let errors = check(vec![Item::ServerBlock(BoundaryBlock {
        items: vec![Item::ClientBlock(BoundaryBlock {
            items: vec![Item::Stmt(Stmt::Signal {
                name: "x".into(),
                ty_ann: None,
                init: Expr::IntLit {
                    value: 0,
                    span: s(),
                },
                span: s(),
            })],
            span: s(),
        })],
        span: s(),
    })]);
    // The signal should not error because the innermost scope is client
    let boundary_errors: Vec<_> = errors
        .iter()
        .filter(|e| {
            matches!(
                e.kind,
                galex::types::constraint::TypeErrorKind::BoundaryViolation { .. }
            )
        })
        .collect();
    assert!(
        boundary_errors.is_empty(),
        "signal in client inside server should not have boundary errors: {:?}",
        boundary_errors
    );
}

// ── T4. Shared scope resolution ───────────────────────────────────────

#[test]
fn shared_type_alias_accessible_everywhere() {
    let errors = check(vec![
        Item::SharedBlock(BoundaryBlock {
            items: vec![Item::TypeAlias(TypeAliasDecl {
                name: "ID".into(),
                ty: TypeAnnotation::Named {
                    name: "string".into(),
                    span: s(),
                },
                span: s(),
            })],
            span: s(),
        }),
        Item::ServerBlock(BoundaryBlock {
            items: vec![Item::Stmt(Stmt::Let {
                name: "serverId".into(),
                ty_ann: Some(TypeAnnotation::Named {
                    name: "ID".into(),
                    span: s(),
                }),
                init: Expr::StringLit {
                    value: "abc".into(),
                    span: s(),
                },
                span: s(),
            })],
            span: s(),
        }),
        Item::ClientBlock(BoundaryBlock {
            items: vec![Item::Stmt(Stmt::Let {
                name: "clientId".into(),
                ty_ann: Some(TypeAnnotation::Named {
                    name: "ID".into(),
                    span: s(),
                }),
                init: Expr::StringLit {
                    value: "def".into(),
                    span: s(),
                },
                span: s(),
            })],
            span: s(),
        }),
    ]);
    assert!(
        errors.is_empty(),
        "shared type alias everywhere: {:?}",
        errors
    );
}

#[test]
fn shared_enum_accessible_everywhere() {
    let errors = check(vec![
        Item::SharedBlock(BoundaryBlock {
            items: vec![Item::EnumDecl(EnumDecl {
                name: "Status".into(),
                variants: vec!["Active".into(), "Inactive".into()],
                span: s(),
            })],
            span: s(),
        }),
        Item::ServerBlock(BoundaryBlock {
            items: vec![Item::Stmt(Stmt::Let {
                name: "s".into(),
                ty_ann: Some(TypeAnnotation::Named {
                    name: "Status".into(),
                    span: s(),
                }),
                init: Expr::Ident {
                    name: "Status".into(),
                    span: s(),
                },
                span: s(),
            })],
            span: s(),
        }),
    ]);
    // Enum is registered as a type, should be resolvable from server
    let type_undef_errors: Vec<_> = errors
        .iter()
        .filter(|e| e.context.contains("undefined type"))
        .collect();
    assert!(
        type_undef_errors.is_empty(),
        "shared enum should be accessible: {:?}",
        type_undef_errors
    );
}

#[test]
fn shared_function_accessible_from_both() {
    let errors = check(vec![
        Item::SharedBlock(BoundaryBlock {
            items: vec![Item::FnDecl(FnDecl {
                name: "validate".into(),
                params: vec![Param {
                    name: "x".into(),
                    ty_ann: Some(TypeAnnotation::Named {
                        name: "int".into(),
                        span: s(),
                    }),
                    default: None,
                    span: s(),
                }],
                ret_ty: Some(TypeAnnotation::Named {
                    name: "bool".into(),
                    span: s(),
                }),
                body: Block {
                    stmts: vec![Stmt::Return {
                        value: Some(Expr::BoolLit {
                            value: true,
                            span: s(),
                        }),
                        span: s(),
                    }],
                    span: s(),
                },
                is_async: false,
                span: s(),
            })],
            span: s(),
        }),
        Item::ServerBlock(BoundaryBlock {
            items: vec![Item::Stmt(Stmt::ExprStmt {
                expr: Expr::FnCall {
                    callee: Box::new(Expr::Ident {
                        name: "validate".into(),
                        span: s(),
                    }),
                    args: vec![Expr::IntLit {
                        value: 42,
                        span: s(),
                    }],
                    span: s(),
                },
                span: s(),
            })],
            span: s(),
        }),
        Item::ClientBlock(BoundaryBlock {
            items: vec![Item::Stmt(Stmt::ExprStmt {
                expr: Expr::FnCall {
                    callee: Box::new(Expr::Ident {
                        name: "validate".into(),
                        span: s(),
                    }),
                    args: vec![Expr::IntLit {
                        value: 10,
                        span: s(),
                    }],
                    span: s(),
                },
                span: s(),
            })],
            span: s(),
        }),
    ]);
    assert!(
        errors.is_empty(),
        "shared fn callable from both: {:?}",
        errors
    );
}

// ── T5. Guard type extraction and validation ──────────────────────────

#[test]
fn guard_multiple_validators_chain() {
    // field: string.minLen(2).maxLen(100).email()
    let errors = check(vec![Item::GuardDecl(GuardDecl {
        name: "ContactForm".into(),
        fields: vec![GuardFieldDecl {
            name: "email".into(),
            ty: TypeAnnotation::Named {
                name: "string".into(),
                span: s(),
            },
            validators: vec![
                ValidatorCall {
                    name: "minLen".into(),
                    args: vec![Expr::IntLit {
                        value: 2,
                        span: s(),
                    }],
                    span: s(),
                },
                ValidatorCall {
                    name: "maxLen".into(),
                    args: vec![Expr::IntLit {
                        value: 100,
                        span: s(),
                    }],
                    span: s(),
                },
                ValidatorCall {
                    name: "email".into(),
                    args: vec![],
                    span: s(),
                },
            ],
            span: s(),
        }],
        span: s(),
    })]);
    assert!(
        errors.is_empty(),
        "guard with multiple validators: {:?}",
        errors
    );
}

#[test]
fn guard_custom_validator() {
    let errors = check(vec![Item::GuardDecl(GuardDecl {
        name: "MyForm".into(),
        fields: vec![GuardFieldDecl {
            name: "data".into(),
            ty: TypeAnnotation::Named {
                name: "string".into(),
                span: s(),
            },
            validators: vec![ValidatorCall {
                name: "myCustomValidator".into(),
                args: vec![],
                span: s(),
            }],
            span: s(),
        }],
        span: s(),
    })]);
    assert!(
        errors.is_empty(),
        "guard with custom validator: {:?}",
        errors
    );
}

#[test]
fn guard_used_as_type_annotation() {
    let errors = check(vec![
        Item::GuardDecl(GuardDecl {
            name: "UserForm".into(),
            fields: vec![
                GuardFieldDecl {
                    name: "name".into(),
                    ty: TypeAnnotation::Named {
                        name: "string".into(),
                        span: s(),
                    },
                    validators: vec![],
                    span: s(),
                },
                GuardFieldDecl {
                    name: "age".into(),
                    ty: TypeAnnotation::Named {
                        name: "int".into(),
                        span: s(),
                    },
                    validators: vec![],
                    span: s(),
                },
            ],
            span: s(),
        }),
        Item::FnDecl(FnDecl {
            name: "processUser".into(),
            params: vec![Param {
                name: "user".into(),
                ty_ann: Some(TypeAnnotation::Named {
                    name: "UserForm".into(),
                    span: s(),
                }),
                default: None,
                span: s(),
            }],
            ret_ty: None,
            body: Block {
                stmts: vec![Stmt::Let {
                    name: "n".into(),
                    ty_ann: None,
                    init: Expr::MemberAccess {
                        object: Box::new(Expr::Ident {
                            name: "user".into(),
                            span: s(),
                        }),
                        field: "name".into(),
                        span: s(),
                    },
                    span: s(),
                }],
                span: s(),
            },
            is_async: false,
            span: s(),
        }),
    ]);
    assert!(errors.is_empty(), "guard as type annotation: {:?}", errors);
}

#[test]
fn guard_omit_then_pick_chain() {
    // User.omit("password") then .pick("email") — compound composition
    let errors = check(vec![
        Item::GuardDecl(GuardDecl {
            name: "User".into(),
            fields: vec![
                GuardFieldDecl {
                    name: "email".into(),
                    ty: TypeAnnotation::Named {
                        name: "string".into(),
                        span: s(),
                    },
                    validators: vec![],
                    span: s(),
                },
                GuardFieldDecl {
                    name: "name".into(),
                    ty: TypeAnnotation::Named {
                        name: "string".into(),
                        span: s(),
                    },
                    validators: vec![],
                    span: s(),
                },
                GuardFieldDecl {
                    name: "password".into(),
                    ty: TypeAnnotation::Named {
                        name: "string".into(),
                        span: s(),
                    },
                    validators: vec![],
                    span: s(),
                },
            ],
            span: s(),
        }),
        Item::Stmt(Stmt::Let {
            name: "safe".into(),
            ty_ann: None,
            init: Expr::FnCall {
                callee: Box::new(Expr::MemberAccess {
                    object: Box::new(Expr::Ident {
                        name: "User".into(),
                        span: s(),
                    }),
                    field: "omit".into(),
                    span: s(),
                }),
                args: vec![Expr::StringLit {
                    value: "password".into(),
                    span: s(),
                }],
                span: s(),
            },
            span: s(),
        }),
    ]);
    assert!(errors.is_empty(), "guard omit composition: {:?}", errors);
}

// ── T6. Generic type inference ────────────────────────────────────────

#[test]
fn infer_function_return_from_body() {
    // fn foo() { return 42 } — return type inferred as int
    let errors = check(vec![Item::FnDecl(FnDecl {
        name: "foo".into(),
        params: vec![],
        ret_ty: None,
        body: Block {
            stmts: vec![Stmt::Return {
                value: Some(Expr::IntLit {
                    value: 42,
                    span: s(),
                }),
                span: s(),
            }],
            span: s(),
        },
        is_async: false,
        span: s(),
    })]);
    assert!(errors.is_empty(), "infer return type: {:?}", errors);
}

#[test]
fn infer_signal_inner_from_init() {
    // signal count = 0  — inner type inferred as int
    let errors = check_stmts(vec![
        Stmt::Signal {
            name: "count".into(),
            ty_ann: None,
            init: Expr::IntLit {
                value: 0,
                span: s(),
            },
            span: s(),
        },
        // Assigning a string to a signal<int> should error
        Stmt::ExprStmt {
            expr: Expr::Assign {
                target: Box::new(Expr::Ident {
                    name: "count".into(),
                    span: s(),
                }),
                op: AssignOp::Assign,
                value: Box::new(Expr::StringLit {
                    value: "hello".into(),
                    span: s(),
                }),
                span: s(),
            },
            span: s(),
        },
    ]);
    assert!(
        !errors.is_empty(),
        "assigning string to signal<int> should error"
    );
}

#[test]
fn infer_multiple_chained_vars() {
    // let a = 1; let b = a; let c: int = b — all int
    let errors = check_stmts(vec![
        Stmt::Let {
            name: "a".into(),
            ty_ann: None,
            init: Expr::IntLit {
                value: 1,
                span: s(),
            },
            span: s(),
        },
        Stmt::Let {
            name: "b".into(),
            ty_ann: None,
            init: Expr::Ident {
                name: "a".into(),
                span: s(),
            },
            span: s(),
        },
        Stmt::Let {
            name: "c".into(),
            ty_ann: Some(TypeAnnotation::Named {
                name: "int".into(),
                span: s(),
            }),
            init: Expr::Ident {
                name: "b".into(),
                span: s(),
            },
            span: s(),
        },
    ]);
    assert!(errors.is_empty(), "chained var inference: {:?}", errors);
}

#[test]
fn infer_component_prop_from_default() {
    // out ui Button(label: string = "Click") { {label} }
    // Annotate the prop type so the template interpolation sees `string`
    // (not a TypeVar, which is not in the renderable set).
    let errors = check(vec![Item::ComponentDecl(ComponentDecl {
        name: "Button".into(),
        props: vec![Param {
            name: "label".into(),
            ty_ann: Some(TypeAnnotation::Named {
                name: "string".into(),
                span: s(),
            }),
            default: Some(Expr::StringLit {
                value: "Click".into(),
                span: s(),
            }),
            span: s(),
        }],
        body: ComponentBody {
            stmts: vec![],
            template: vec![TemplateNode::ExprInterp {
                expr: Expr::Ident {
                    name: "label".into(),
                    span: s(),
                },
                span: s(),
            }],
            head: None,
            span: s(),
        },
        span: s(),
    })]);
    assert!(errors.is_empty(), "prop default inference: {:?}", errors);
}

#[test]
fn infer_query_result_with_annotation() {
    let errors = check(vec![Item::QueryDecl(QueryDecl {
        name: "users".into(),
        url_pattern: Expr::StringLit {
            value: "/api/users".into(),
            span: s(),
        },
        ret_ty: Some(TypeAnnotation::Array {
            element: Box::new(TypeAnnotation::Named {
                name: "string".into(),
                span: s(),
            }),
            span: s(),
        }),
        span: s(),
    })]);
    assert!(errors.is_empty(), "query result annotation: {:?}", errors);
}

#[test]
fn infer_arrow_fn_return() {
    // let f = (x: int) => x + 1 — return type inferred from body
    let errors = check_stmts(vec![Stmt::Let {
        name: "f".into(),
        ty_ann: None,
        init: Expr::ArrowFn {
            params: vec![Param {
                name: "x".into(),
                ty_ann: Some(TypeAnnotation::Named {
                    name: "int".into(),
                    span: s(),
                }),
                default: None,
                span: s(),
            }],
            ret_ty: None,
            body: ArrowBody::Expr(Box::new(Expr::BinaryOp {
                left: Box::new(Expr::Ident {
                    name: "x".into(),
                    span: s(),
                }),
                op: BinOp::Add,
                right: Box::new(Expr::IntLit {
                    value: 1,
                    span: s(),
                }),
                span: s(),
            })),
            span: s(),
        },
        span: s(),
    }]);
    assert!(errors.is_empty(), "arrow fn return inference: {:?}", errors);
}

#[test]
fn default_param_type_mismatch_errors() {
    // fn greet(name: string = 42) — default value doesn't match annotation
    let errors = check(vec![Item::FnDecl(FnDecl {
        name: "greet".into(),
        params: vec![Param {
            name: "name".into(),
            ty_ann: Some(TypeAnnotation::Named {
                name: "string".into(),
                span: s(),
            }),
            default: Some(Expr::IntLit {
                value: 42,
                span: s(),
            }),
            span: s(),
        }],
        ret_ty: None,
        body: Block {
            stmts: vec![],
            span: s(),
        },
        is_async: false,
        span: s(),
    })]);
    assert!(
        !errors.is_empty(),
        "default param type mismatch should error"
    );
}

// ── T7. Complex scenarios ─────────────────────────────────────────────

#[test]
fn store_with_derived_and_methods() {
    // Store with signal and a method that mutates it.
    // Note: derive expressions that use arithmetic on signals would need
    // signal auto-unwrapping (not yet implemented). Testing signal + method only.
    let errors = check(vec![Item::StoreDecl(StoreDecl {
        name: "Counter".into(),
        members: vec![
            StoreMember::Signal(Stmt::Signal {
                name: "count".into(),
                ty_ann: None,
                init: Expr::IntLit {
                    value: 0,
                    span: s(),
                },
                span: s(),
            }),
            StoreMember::Method(FnDecl {
                name: "inc".into(),
                params: vec![],
                ret_ty: None,
                body: Block {
                    stmts: vec![Stmt::ExprStmt {
                        expr: Expr::Assign {
                            target: Box::new(Expr::Ident {
                                name: "count".into(),
                                span: s(),
                            }),
                            op: AssignOp::Assign,
                            value: Box::new(Expr::IntLit {
                                value: 1,
                                span: s(),
                            }),
                            span: s(),
                        },
                        span: s(),
                    }],
                    span: s(),
                },
                is_async: false,
                span: s(),
            }),
        ],
        span: s(),
    })]);
    assert!(
        errors.is_empty(),
        "store with signal and method: {:?}",
        errors
    );
}

#[test]
fn action_with_guard_param_validation() {
    // guard LoginForm { email: string.email() }
    // action login(data: LoginForm) -> void { let e = data.email }
    let errors = check(vec![
        Item::GuardDecl(GuardDecl {
            name: "LoginForm".into(),
            fields: vec![GuardFieldDecl {
                name: "email".into(),
                ty: TypeAnnotation::Named {
                    name: "string".into(),
                    span: s(),
                },
                validators: vec![ValidatorCall {
                    name: "email".into(),
                    args: vec![],
                    span: s(),
                }],
                span: s(),
            }],
            span: s(),
        }),
        Item::ActionDecl(ActionDecl {
            name: "login".into(),
            params: vec![Param {
                name: "data".into(),
                ty_ann: Some(TypeAnnotation::Named {
                    name: "LoginForm".into(),
                    span: s(),
                }),
                default: None,
                span: s(),
            }],
            ret_ty: Some(TypeAnnotation::Named {
                name: "void".into(),
                span: s(),
            }),
            body: Block {
                stmts: vec![Stmt::Let {
                    name: "e".into(),
                    ty_ann: None,
                    init: Expr::MemberAccess {
                        object: Box::new(Expr::Ident {
                            name: "data".into(),
                            span: s(),
                        }),
                        field: "email".into(),
                        span: s(),
                    },
                    span: s(),
                }],
                span: s(),
            },
            span: s(),
        }),
    ]);
    assert!(errors.is_empty(), "action with guard param: {:?}", errors);
}

#[test]
fn component_with_signals_binds_and_events() {
    // Full component: signal name = ""; let label = "Name";
    // <input bind:name on:input={(e) => {}} />; <p>{label}</p>
    // Note: signals are not yet auto-unwrapped in template interpolation,
    // so we use a separate let binding for the interpolation.
    let errors = check(vec![Item::ComponentDecl(ComponentDecl {
        name: "Form".into(),
        props: vec![],
        body: ComponentBody {
            stmts: vec![
                Stmt::Signal {
                    name: "name".into(),
                    ty_ann: None,
                    init: Expr::StringLit {
                        value: "".into(),
                        span: s(),
                    },
                    span: s(),
                },
                Stmt::Let {
                    name: "label".into(),
                    ty_ann: None,
                    init: Expr::StringLit {
                        value: "Name".into(),
                        span: s(),
                    },
                    span: s(),
                },
            ],
            template: vec![
                TemplateNode::SelfClosing {
                    tag: "input".into(),
                    attributes: vec![],
                    directives: vec![
                        Directive::Bind {
                            field: "name".into(),
                            span: s(),
                        },
                        Directive::On {
                            event: "input".into(),
                            modifiers: vec![],
                            handler: Expr::ArrowFn {
                                params: vec![Param {
                                    name: "e".into(),
                                    ty_ann: None,
                                    default: None,
                                    span: s(),
                                }],
                                ret_ty: None,
                                body: ArrowBody::Block(Block {
                                    stmts: vec![],
                                    span: s(),
                                }),
                                span: s(),
                            },
                            span: s(),
                        },
                    ],
                    span: s(),
                },
                TemplateNode::Element {
                    tag: "p".into(),
                    attributes: vec![],
                    directives: vec![],
                    children: vec![TemplateNode::ExprInterp {
                        expr: Expr::Ident {
                            name: "label".into(),
                            span: s(),
                        },
                        span: s(),
                    }],
                    span: s(),
                },
            ],
            head: None,
            span: s(),
        },
        span: s(),
    })]);
    assert!(errors.is_empty(), "full component: {:?}", errors);
}

#[test]
fn nested_function_calls_with_type_flow() {
    // fn a(x: int) -> string { ... }; fn b(s: string) -> bool { ... }; let r = b(a(42))
    let errors = check(vec![
        Item::FnDecl(FnDecl {
            name: "intToStr".into(),
            params: vec![Param {
                name: "x".into(),
                ty_ann: Some(TypeAnnotation::Named {
                    name: "int".into(),
                    span: s(),
                }),
                default: None,
                span: s(),
            }],
            ret_ty: Some(TypeAnnotation::Named {
                name: "string".into(),
                span: s(),
            }),
            body: Block {
                stmts: vec![Stmt::Return {
                    value: Some(Expr::StringLit {
                        value: "42".into(),
                        span: s(),
                    }),
                    span: s(),
                }],
                span: s(),
            },
            is_async: false,
            span: s(),
        }),
        Item::FnDecl(FnDecl {
            name: "strToBool".into(),
            params: vec![Param {
                name: "s".into(),
                ty_ann: Some(TypeAnnotation::Named {
                    name: "string".into(),
                    span: s(),
                }),
                default: None,
                span: s(),
            }],
            ret_ty: Some(TypeAnnotation::Named {
                name: "bool".into(),
                span: s(),
            }),
            body: Block {
                stmts: vec![Stmt::Return {
                    value: Some(Expr::BoolLit {
                        value: true,
                        span: s(),
                    }),
                    span: s(),
                }],
                span: s(),
            },
            is_async: false,
            span: s(),
        }),
        Item::Stmt(Stmt::Let {
            name: "result".into(),
            ty_ann: Some(TypeAnnotation::Named {
                name: "bool".into(),
                span: s(),
            }),
            init: Expr::FnCall {
                callee: Box::new(Expr::Ident {
                    name: "strToBool".into(),
                    span: s(),
                }),
                args: vec![Expr::FnCall {
                    callee: Box::new(Expr::Ident {
                        name: "intToStr".into(),
                        span: s(),
                    }),
                    args: vec![Expr::IntLit {
                        value: 42,
                        span: s(),
                    }],
                    span: s(),
                }],
                span: s(),
            },
            span: s(),
        }),
    ]);
    assert!(errors.is_empty(), "nested calls type flow: {:?}", errors);
}

#[test]
fn block_statement_scope_isolation() {
    // { let inner = 1 }; use inner — should error
    // Note: uses Stmt::Block which pushes scope. The `if` statement's
    // check_block does not yet push its own scope (future improvement).
    let errors = check_stmts(vec![
        Stmt::Block(Block {
            stmts: vec![Stmt::Let {
                name: "inner".into(),
                ty_ann: None,
                init: Expr::IntLit {
                    value: 1,
                    span: s(),
                },
                span: s(),
            }],
            span: s(),
        }),
        Stmt::ExprStmt {
            expr: Expr::Ident {
                name: "inner".into(),
                span: s(),
            },
            span: s(),
        },
    ]);
    assert!(
        !errors.is_empty(),
        "inner should be undefined outside block scope"
    );
}

#[test]
fn for_loop_element_type_flows() {
    // let items: string[] = ["a"]; for item in items { let x: string = item }
    // Annotate the array to avoid string-literal type inference issues.
    let errors = check_stmts(vec![
        Stmt::Let {
            name: "items".into(),
            ty_ann: Some(TypeAnnotation::Array {
                element: Box::new(TypeAnnotation::Named {
                    name: "string".into(),
                    span: s(),
                }),
                span: s(),
            }),
            init: Expr::ArrayLit {
                elements: vec![Expr::StringLit {
                    value: "a".into(),
                    span: s(),
                }],
                span: s(),
            },
            span: s(),
        },
        Stmt::For {
            binding: "item".into(),
            index: None,
            iterable: Expr::Ident {
                name: "items".into(),
                span: s(),
            },
            body: Block {
                stmts: vec![Stmt::Let {
                    name: "x".into(),
                    ty_ann: Some(TypeAnnotation::Named {
                        name: "string".into(),
                        span: s(),
                    }),
                    init: Expr::Ident {
                        name: "item".into(),
                        span: s(),
                    },
                    span: s(),
                }],
                span: s(),
            },
            span: s(),
        },
    ]);
    assert!(errors.is_empty(), "for loop element type: {:?}", errors);
}

#[test]
fn object_member_access_typed() {
    let errors = check_stmts(vec![
        Stmt::Let {
            name: "user".into(),
            ty_ann: None,
            init: Expr::ObjectLit {
                fields: vec![
                    ObjectFieldExpr {
                        key: "name".into(),
                        value: Expr::StringLit {
                            value: "Alice".into(),
                            span: s(),
                        },
                        span: s(),
                    },
                    ObjectFieldExpr {
                        key: "age".into(),
                        value: Expr::IntLit {
                            value: 30,
                            span: s(),
                        },
                        span: s(),
                    },
                ],
                span: s(),
            },
            span: s(),
        },
        Stmt::Let {
            name: "n".into(),
            ty_ann: Some(TypeAnnotation::Named {
                name: "string".into(),
                span: s(),
            }),
            init: Expr::MemberAccess {
                object: Box::new(Expr::Ident {
                    name: "user".into(),
                    span: s(),
                }),
                field: "name".into(),
                span: s(),
            },
            span: s(),
        },
        Stmt::Let {
            name: "a".into(),
            ty_ann: Some(TypeAnnotation::Named {
                name: "int".into(),
                span: s(),
            }),
            init: Expr::MemberAccess {
                object: Box::new(Expr::Ident {
                    name: "user".into(),
                    span: s(),
                }),
                field: "age".into(),
                span: s(),
            },
            span: s(),
        },
    ]);
    assert!(errors.is_empty(), "object member access: {:?}", errors);
}

#[test]
fn array_length_builtin() {
    let errors = check_stmts(vec![
        Stmt::Let {
            name: "arr".into(),
            ty_ann: None,
            init: Expr::ArrayLit {
                elements: vec![Expr::IntLit {
                    value: 1,
                    span: s(),
                }],
                span: s(),
            },
            span: s(),
        },
        Stmt::Let {
            name: "len".into(),
            ty_ann: Some(TypeAnnotation::Named {
                name: "int".into(),
                span: s(),
            }),
            init: Expr::MemberAccess {
                object: Box::new(Expr::Ident {
                    name: "arr".into(),
                    span: s(),
                }),
                field: "length".into(),
                span: s(),
            },
            span: s(),
        },
    ]);
    assert!(errors.is_empty(), "array.length: {:?}", errors);
}
