//! Integration tests for the type system: interning, scoping, unification.

use galex::span::Span;
use galex::types::constraint::{ConstraintSolver, TypeErrorKind};
use galex::types::env::{Binding, BindingKind, ScopeKind, TypeEnv};
use galex::types::ty::*;
use galex::types::validation::Validation;

// ── TypeInterner ───────────────────────────────────────────────────────

#[test]
fn interner_primitives_have_stable_ids() {
    let i1 = TypeInterner::new();
    let i2 = TypeInterner::new();
    assert_eq!(i1.string.raw(), i2.string.raw());
    assert_eq!(i1.int.raw(), i2.int.raw());
    assert_eq!(i1.float.raw(), i2.float.raw());
}

#[test]
fn interner_compound_dedup() {
    let mut i = TypeInterner::new();
    let f1 = i.make_function(FunctionSig {
        params: vec![FnParam {
            name: "x".into(),
            ty: i.int,
            has_default: false,
        }],
        ret: i.string,
        is_async: false,
    });
    let f2 = i.make_function(FunctionSig {
        params: vec![FnParam {
            name: "x".into(),
            ty: i.int,
            has_default: false,
        }],
        ret: i.string,
        is_async: false,
    });
    assert_eq!(
        f1, f2,
        "identical function types should be interned to same ID"
    );
}

#[test]
fn interner_nested_array() {
    let mut i = TypeInterner::new();
    let inner = i.make_array(i.int);
    let outer = i.make_array(inner);
    assert_ne!(inner, outer);
    assert_eq!(i.display(outer), "int[][]");
}

#[test]
fn interner_complex_union() {
    let mut i = TypeInterner::new();
    let lit_a = i.make_string_literal("primary");
    let lit_b = i.make_string_literal("ghost");
    let lit_c = i.make_string_literal("danger");
    let union = i.make_union(vec![lit_a, lit_b, lit_c]);
    let display = i.display(union);
    assert!(display.contains("\"primary\""));
    assert!(display.contains("\"ghost\""));
    assert!(display.contains("\"danger\""));
}

#[test]
fn interner_optional_display() {
    let mut i = TypeInterner::new();
    let opt = i.make_optional(i.string);
    assert_eq!(i.display(opt), "string?");
}

#[test]
fn interner_signal_and_derived() {
    let mut i = TypeInterner::new();
    let sig = i.make_signal(i.int);
    let der = i.make_derived(i.int);
    assert_ne!(sig, der);
    assert_eq!(i.display(sig), "signal<int>");
    assert_eq!(i.display(der), "derived<int>");
}

#[test]
fn interner_guard_type() {
    let mut i = TypeInterner::new();
    let guard = i.make_guard(GuardDef {
        name: "User".into(),
        fields: vec![
            GuardField {
                name: "name".into(),
                ty: i.string,
                validations: vec![Validation::MinLen(2), Validation::MaxLen(100)],
            },
            GuardField {
                name: "email".into(),
                ty: i.string,
                validations: vec![Validation::Email],
            },
        ],
        extends: None,
        has_validators: true,
    });
    assert_eq!(i.display(guard), "guard User");
}

#[test]
fn interner_enum_type() {
    let mut i = TypeInterner::new();
    let e = i.intern(TypeData::Enum(EnumDef {
        name: "Status".into(),
        variants: vec!["Active".into(), "Inactive".into(), "Banned".into()],
    }));
    assert_eq!(i.display(e), "enum Status");
}

// ── TypeEnv ────────────────────────────────────────────────────────────

fn make_binding(ty: TypeId, kind: BindingKind) -> Binding {
    Binding {
        ty,
        kind,
        span: Span::dummy(),
        boundary: galex::types::env::BoundaryScope::Unscoped,
    }
}

#[test]
fn env_deep_nesting() {
    let mut env = TypeEnv::new();
    let ty = TypeId::from_raw(0);

    env.define("global".into(), make_binding(ty, BindingKind::Let))
        .unwrap();

    env.push_scope(ScopeKind::Function);
    env.define("fn_local".into(), make_binding(ty, BindingKind::Parameter))
        .unwrap();

    env.push_scope(ScopeKind::Block);
    env.define("block_local".into(), make_binding(ty, BindingKind::Let))
        .unwrap();

    assert!(env.lookup("global").is_some());
    assert!(env.lookup("fn_local").is_some());
    assert!(env.lookup("block_local").is_some());

    env.pop_scope();
    assert!(env.lookup("global").is_some());
    assert!(env.lookup("fn_local").is_some());
    assert!(env.lookup("block_local").is_none());

    env.pop_scope();
    assert!(env.lookup("global").is_some());
    assert!(env.lookup("fn_local").is_none());
}

#[test]
fn env_shadowing_restores() {
    let mut env = TypeEnv::new();
    let outer_ty = TypeId::from_raw(10);
    let inner_ty = TypeId::from_raw(20);

    env.define("x".into(), make_binding(outer_ty, BindingKind::Let))
        .unwrap();

    env.push_scope(ScopeKind::Block);
    env.define("x".into(), make_binding(inner_ty, BindingKind::Mut))
        .unwrap();

    assert_eq!(env.lookup("x").unwrap().ty, inner_ty);
    assert_eq!(env.lookup("x").unwrap().kind, BindingKind::Mut);

    env.pop_scope();
    assert_eq!(env.lookup("x").unwrap().ty, outer_ty);
    assert_eq!(env.lookup("x").unwrap().kind, BindingKind::Let);
}

#[test]
fn env_server_scope_detection() {
    let mut env = TypeEnv::new();
    assert!(!env.is_inside(ScopeKind::ServerBlock));

    env.push_scope(ScopeKind::ServerBlock);
    env.push_scope(ScopeKind::Function);
    assert!(env.is_inside(ScopeKind::ServerBlock));
    assert!(env.is_inside(ScopeKind::Function));
    assert!(env.is_inside(ScopeKind::Global));
}

// ── ConstraintSolver ───────────────────────────────────────────────────

#[test]
fn solver_multiple_constraints() {
    let mut i = TypeInterner::new();
    let v1 = i.fresh_type_var();
    let v2 = i.fresh_type_var();
    let v3 = i.fresh_type_var();
    let (int, string) = (i.int, i.string);

    let mut s = ConstraintSolver::new(&mut i);
    s.constrain_equal(v1, int, Span::dummy(), "v1 = int");
    s.constrain_equal(v2, string, Span::dummy(), "v2 = string");
    s.constrain_equal(v3, v1, Span::dummy(), "v3 = v1");

    let errors = s.solve();
    assert!(errors.is_empty(), "errors: {:?}", errors);
    assert_eq!(s.resolve(v1), int);
    assert_eq!(s.resolve(v2), string);
    assert_eq!(s.resolve(v3), int);
}

#[test]
fn solver_array_element_inference() {
    let mut i = TypeInterner::new();
    let int = i.int;
    let elem_var = i.fresh_type_var();
    let arr_var = i.make_array(elem_var);
    let arr_int = i.make_array(int);

    let mut s = ConstraintSolver::new(&mut i);
    s.constrain_equal(arr_var, arr_int, Span::dummy(), "array type");

    let errors = s.solve();
    assert!(errors.is_empty());
    assert_eq!(s.resolve(elem_var), int);
}

#[test]
fn solver_function_param_inference() {
    let mut i = TypeInterner::new();
    let (int, string) = (i.int, i.string);
    let param_var = i.fresh_type_var();
    let ret_var = i.fresh_type_var();

    let fn_unknown = i.make_function(FunctionSig {
        params: vec![FnParam {
            name: "x".into(),
            ty: param_var,
            has_default: false,
        }],
        ret: ret_var,
        is_async: false,
    });
    let fn_known = i.make_function(FunctionSig {
        params: vec![FnParam {
            name: "x".into(),
            ty: int,
            has_default: false,
        }],
        ret: string,
        is_async: false,
    });

    let mut s = ConstraintSolver::new(&mut i);
    s.constrain_equal(fn_unknown, fn_known, Span::dummy(), "fn type");

    let errors = s.solve();
    assert!(errors.is_empty());
    assert_eq!(s.resolve(param_var), int);
    assert_eq!(s.resolve(ret_var), string);
}

#[test]
fn solver_string_literal_union_assignability() {
    let mut i = TypeInterner::new();
    let primary = i.make_string_literal("primary");
    let ghost = i.make_string_literal("ghost");
    let danger = i.make_string_literal("danger");
    let variant_union = i.make_union(vec![primary, ghost, danger]);

    let mut s = ConstraintSolver::new(&mut i);
    s.constrain_assignable(primary, variant_union, Span::dummy(), "variant check");

    let errors = s.solve();
    assert!(errors.is_empty());
}

#[test]
fn solver_string_literal_not_in_union() {
    let mut i = TypeInterner::new();
    let primary = i.make_string_literal("primary");
    let ghost = i.make_string_literal("ghost");
    let union = i.make_union(vec![primary, ghost]);
    let danger = i.make_string_literal("danger");

    let mut s = ConstraintSolver::new(&mut i);
    s.constrain_assignable(danger, union, Span::dummy(), "variant check");

    let errors = s.solve();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].kind, TypeErrorKind::NotAssignable);
}

#[test]
fn solver_optional_accepts_null_and_value() {
    let mut i = TypeInterner::new();
    let (null, string) = (i.null, i.string);
    let opt_string = i.make_optional(string);

    let mut s = ConstraintSolver::new(&mut i);
    s.constrain_assignable(null, opt_string, Span::dummy(), "null to optional");
    s.constrain_assignable(string, opt_string, Span::dummy(), "string to optional");

    let errors = s.solve();
    assert!(errors.is_empty());
}

#[test]
fn solver_optional_rejects_wrong_type() {
    let mut i = TypeInterner::new();
    let int = i.int;
    let opt_string = i.make_optional(i.string);

    let mut s = ConstraintSolver::new(&mut i);
    s.constrain_assignable(int, opt_string, Span::dummy(), "int to string?");

    let errors = s.solve();
    assert_eq!(errors.len(), 1);
}

#[test]
fn solver_never_assignable_to_anything() {
    let mut i = TypeInterner::new();
    let (never, string) = (i.never, i.string);
    let arr = i.make_array(string);
    let union = i.make_union(vec![i.int, i.bool_]);

    let mut s = ConstraintSolver::new(&mut i);
    s.constrain_assignable(never, string, Span::dummy(), "1");
    s.constrain_assignable(never, arr, Span::dummy(), "2");
    s.constrain_assignable(never, union, Span::dummy(), "3");

    let errors = s.solve();
    assert!(errors.is_empty());
}

#[test]
fn solver_signal_unification() {
    let mut i = TypeInterner::new();
    let int = i.int;
    let sig_int = i.make_signal(int);
    let var = i.fresh_type_var();
    let sig_var = i.make_signal(var);

    let mut s = ConstraintSolver::new(&mut i);
    s.constrain_equal(sig_var, sig_int, Span::dummy(), "signal type");

    let errors = s.solve();
    assert!(errors.is_empty());
    assert_eq!(s.resolve(var), int);
}

// ── Validation ─────────────────────────────────────────────────────────

#[test]
fn validation_descriptions() {
    assert!(Validation::Email.description().contains("email"));
    assert!(Validation::Min(5).description().contains("5"));
    assert!(Validation::MaxLen(100).description().contains("100"));
    assert!(Validation::OneOf(vec!["a".into(), "b".into()])
        .description()
        .contains("a"));
}

// ── Display formatting ─────────────────────────────────────────────────

#[test]
fn display_complex_types() {
    let mut i = TypeInterner::new();

    let chan = i.intern(TypeData::Channel(ChannelDef {
        param_ty: i.string,
        msg_ty: i.string,
        direction: ChannelDirection::Bidirectional,
    }));
    assert_eq!(i.display(chan), "channel <-> string");

    let comp = i.intern(TypeData::Component(ComponentDef {
        name: "Button".into(),
        props: vec![PropDef {
            name: "label".into(),
            ty: i.string,
            has_default: false,
        }],
        slots: vec!["default".into()],
    }));
    assert_eq!(i.display(comp), "component Button");

    let named = i.make_named("HTMLCanvasElement");
    let dom_ref = i.make_dom_ref(named);
    assert_eq!(i.display(dom_ref), "ref<HTMLCanvasElement>");
}

// ── AST smoke test ─────────────────────────────────────────────────────

#[test]
fn ast_nodes_construct() {
    use galex::ast::*;

    let span = Span::dummy();

    let _expr = Expr::BinaryOp {
        left: Box::new(Expr::IntLit { value: 1, span }),
        op: BinOp::Add,
        right: Box::new(Expr::IntLit { value: 2, span }),
        span,
    };

    let _stmt = Stmt::Let {
        name: "x".into(),
        ty_ann: Some(TypeAnnotation::Named {
            name: "int".into(),
            span,
        }),
        init: Expr::IntLit { value: 42, span },
        span,
    };

    let _comp = ComponentDecl {
        name: "Button".into(),
        props: vec![Param {
            name: "label".into(),
            ty_ann: Some(TypeAnnotation::Named {
                name: "string".into(),
                span,
            }),
            default: None,
            span,
        }],
        body: ComponentBody {
            stmts: vec![],
            template: vec![TemplateNode::Text {
                value: "Click".into(),
                span,
            }],
            head: None,
            span,
        },
        span,
    };

    let _template = TemplateNode::Element {
        tag: "button".into(),
        attributes: vec![Attribute {
            name: "class".into(),
            value: AttrValue::String("btn".into()),
            span,
        }],
        directives: vec![Directive::On {
            event: "click".into(),
            modifiers: vec!["prevent".into()],
            handler: Expr::Ident {
                name: "handler".into(),
                span,
            },
            span,
        }],
        children: vec![],
        span,
    };
}
