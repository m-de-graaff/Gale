//! Benchmarks for the GaleX type checker.
//!
//! Measures cold-start TypeChecker::new() + check_program() on a
//! synthetic 20-file project. Target: < 200ms.

use criterion::{criterion_group, criterion_main, Criterion};
use galex::ast::*;
use galex::checker::TypeChecker;
use galex::span::Span;

fn s() -> Span {
    Span::dummy()
}

/// Generate a synthetic project with `file_count` "files" worth of items.
///
/// Each file contributes ~15 items: functions, guards, signals, derives,
/// a store, a component, and assorted statements.
fn generate_synthetic_project(file_count: usize) -> Program {
    let mut items = Vec::new();

    for i in 0..file_count {
        // A guard with validated fields
        items.push(Item::GuardDecl(GuardDecl {
            name: format!("Form{i}").into(),
            fields: vec![
                GuardFieldDecl {
                    name: "name".into(),
                    ty: TypeAnnotation::Named {
                        name: "string".into(),
                        span: s(),
                    },
                    validators: vec![ValidatorCall {
                        name: "minLen".into(),
                        args: vec![Expr::IntLit {
                            value: 1,
                            span: s(),
                        }],
                        span: s(),
                    }],
                    span: s(),
                },
                GuardFieldDecl {
                    name: "age".into(),
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
                },
            ],
            span: s(),
        }));

        // A function
        items.push(Item::FnDecl(FnDecl {
            name: format!("process{i}").into(),
            params: vec![
                Param {
                    name: "x".into(),
                    ty_ann: Some(TypeAnnotation::Named {
                        name: "int".into(),
                        span: s(),
                    }),
                    default: None,
                    span: s(),
                },
                Param {
                    name: "y".into(),
                    ty_ann: Some(TypeAnnotation::Named {
                        name: "string".into(),
                        span: s(),
                    }),
                    default: None,
                    span: s(),
                },
            ],
            ret_ty: Some(TypeAnnotation::Named {
                name: "bool".into(),
                span: s(),
            }),
            body: Block {
                stmts: vec![
                    Stmt::Let {
                        name: "result".into(),
                        ty_ann: None,
                        init: Expr::BinaryOp {
                            left: Box::new(Expr::Ident {
                                name: "x".into(),
                                span: s(),
                            }),
                            op: BinOp::Gt,
                            right: Box::new(Expr::IntLit {
                                value: 0,
                                span: s(),
                            }),
                            span: s(),
                        },
                        span: s(),
                    },
                    Stmt::Return {
                        value: Some(Expr::Ident {
                            name: "result".into(),
                            span: s(),
                        }),
                        span: s(),
                    },
                ],
                span: s(),
            },
            is_async: false,
            span: s(),
        }));

        // An enum
        items.push(Item::EnumDecl(EnumDecl {
            name: format!("Status{i}").into(),
            variants: vec!["Active".into(), "Inactive".into(), "Pending".into()],
            span: s(),
        }));

        // A component with template
        items.push(Item::ComponentDecl(ComponentDecl {
            name: format!("Page{i}").into(),
            props: vec![Param {
                name: "title".into(),
                ty_ann: Some(TypeAnnotation::Named {
                    name: "string".into(),
                    span: s(),
                }),
                default: Some(Expr::StringLit {
                    value: "Default".into(),
                    span: s(),
                }),
                span: s(),
            }],
            body: ComponentBody {
                stmts: vec![
                    Stmt::Signal {
                        name: "count".into(),
                        ty_ann: None,
                        init: Expr::IntLit {
                            value: 0,
                            span: s(),
                        },
                        span: s(),
                    },
                    Stmt::Derive {
                        name: "doubled".into(),
                        init: Expr::BinaryOp {
                            left: Box::new(Expr::Ident {
                                name: "count".into(),
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
                ],
                template: vec![TemplateNode::Element {
                    tag: "div".into(),
                    attributes: vec![],
                    directives: vec![],
                    children: vec![
                        TemplateNode::ExprInterp {
                            expr: Expr::Ident {
                                name: "title".into(),
                                span: s(),
                            },
                            span: s(),
                        },
                        TemplateNode::ExprInterp {
                            expr: Expr::Ident {
                                name: "count".into(),
                                span: s(),
                            },
                            span: s(),
                        },
                    ],
                    span: s(),
                }],
                head: None,
                span: s(),
            },
            span: s(),
        }));

        // A store
        items.push(Item::StoreDecl(StoreDecl {
            name: format!("Store{i}").into(),
            members: vec![
                StoreMember::Signal(Stmt::Signal {
                    name: "value".into(),
                    ty_ann: None,
                    init: Expr::IntLit {
                        value: 0,
                        span: s(),
                    },
                    span: s(),
                }),
                StoreMember::Method(FnDecl {
                    name: "set".into(),
                    params: vec![Param {
                        name: "v".into(),
                        ty_ann: Some(TypeAnnotation::Named {
                            name: "int".into(),
                            span: s(),
                        }),
                        default: None,
                        span: s(),
                    }],
                    ret_ty: None,
                    body: Block {
                        stmts: vec![Stmt::ExprStmt {
                            expr: Expr::Assign {
                                target: Box::new(Expr::Ident {
                                    name: "value".into(),
                                    span: s(),
                                }),
                                op: AssignOp::Assign,
                                value: Box::new(Expr::Ident {
                                    name: "v".into(),
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
        }));
    }

    Program { items, span: s() }
}

fn bench_typecheck_20_files(c: &mut Criterion) {
    let program = generate_synthetic_project(20);

    c.bench_function("typecheck_20_files", |b| {
        b.iter(|| {
            let mut checker = TypeChecker::new();
            let errors = checker.check_program(&program);
            let _ = errors.len();
        });
    });
}

fn bench_typecheck_100_files(c: &mut Criterion) {
    let program = generate_synthetic_project(100);

    c.bench_function("typecheck_100_files", |b| {
        b.iter(|| {
            let mut checker = TypeChecker::new();
            let errors = checker.check_program(&program);
            let _ = errors.len();
        });
    });
}

criterion_group!(benches, bench_typecheck_20_files, bench_typecheck_100_files);
criterion_main!(benches);
