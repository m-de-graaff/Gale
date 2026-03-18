//! Compile test: generates a full project and runs `cargo check` on it.
//!
//! This test is `#[ignore]` by default because it's slow (~15s).
//! Run explicitly with: `cargo test --test compile_test -- --ignored`

use galex::ast::*;
use galex::codegen;
use galex::span::Span;
use galex::types::ty::TypeInterner;
use std::path::PathBuf;

fn s() -> Span {
    Span::dummy()
}

/// Build a realistic program with all feature types: guard, action,
/// components (static + dynamic), channel, API, enum.
fn build_kitchen_sink_program() -> Program {
    let mut items = Vec::new();

    // Guard with validators
    items.push(Item::GuardDecl(GuardDecl {
        name: "UserForm".into(),
        fields: vec![
            GuardFieldDecl {
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
            },
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

    // Action with guard param
    items.push(Item::ActionDecl(ActionDecl {
        name: "createUser".into(),
        params: vec![Param {
            name: "data".into(),
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
                name: "result".into(),
                ty_ann: None,
                init: Expr::ObjectLit {
                    fields: vec![ObjectFieldExpr {
                        key: "id".into(),
                        value: Expr::IntLit {
                            value: 1,
                            span: s(),
                        },
                        span: s(),
                    }],
                    span: s(),
                },
                span: s(),
            }],
            span: s(),
        },
        span: s(),
    }));

    // Static page component
    items.push(Item::ComponentDecl(ComponentDecl {
        name: "HomePage".into(),
        props: vec![],
        body: ComponentBody {
            stmts: vec![],
            template: vec![TemplateNode::Element {
                tag: "h1".into(),
                attributes: vec![],
                directives: vec![],
                children: vec![TemplateNode::Text {
                    value: "Welcome".into(),
                    span: s(),
                }],
                span: s(),
            }],
            head: None,
            span: s(),
        },
        span: s(),
    }));

    // Dynamic route component
    items.push(Item::ComponentDecl(ComponentDecl {
        name: "UserById[id]".into(),
        props: vec![],
        body: ComponentBody {
            stmts: vec![],
            template: vec![TemplateNode::Text {
                value: "User page".into(),
                span: s(),
            }],
            head: None,
            span: s(),
        },
        span: s(),
    }));

    // Channel (bidirectional)
    items.push(Item::ChannelDecl(ChannelDecl {
        name: "Chat".into(),
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
                ty_ann: None,
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
    }));

    // API resource (empty body — avoids return type issues)
    items.push(Item::ApiDecl(ApiDecl {
        name: "Users".into(),
        handlers: vec![ApiHandler {
            method: HttpMethod::Get,
            path_params: vec![],
            params: vec![],
            ret_ty: None,
            body: Block {
                stmts: vec![],
                span: s(),
            },
            span: s(),
        }],
        span: s(),
    }));

    // Enum
    items.push(Item::EnumDecl(EnumDecl {
        name: "Status".into(),
        variants: vec!["Active".into(), "Inactive".into()],
        span: s(),
    }));

    Program { items, span: s() }
}

/// Resolve the path to the Gale workspace root (parent of galex/).
fn gale_workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .expect("galex crate should be inside the Gale workspace")
        .to_path_buf()
}

#[test]
#[ignore] // Slow test — run with: cargo test --test compile_test -- --ignored
fn generated_project_compiles() {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let output_dir = dir.path().join("gale_build");

    let program = build_kitchen_sink_program();
    let interner = TypeInterner::new();

    // Generate the full project
    codegen::generate(&program, &interner, "compile_test_app", &output_dir, None)
        .expect("codegen::generate failed");

    // Fix the `gale` dependency path to point to the real workspace
    let cargo_toml_path = output_dir.join("Cargo.toml");
    let content =
        std::fs::read_to_string(&cargo_toml_path).expect("failed to read generated Cargo.toml");

    let gale_root = gale_workspace_root();
    let gale_path_str = gale_root.display().to_string().replace('\\', "/");
    let fixed = content.replace(
        "gale = { path = \"../\" }",
        &format!("gale = {{ path = \"{gale_path_str}\", package = \"gale\" }}"),
    );
    // Add [workspace] to prevent cargo from looking for a parent workspace
    let fixed = format!("{fixed}\n[workspace]\n");
    std::fs::write(&cargo_toml_path, fixed).expect("failed to write patched Cargo.toml");

    // Run cargo check on the generated project
    let output = std::process::Command::new("cargo")
        .args(["check", "--message-format=short"])
        .current_dir(&output_dir)
        .output()
        .expect("failed to invoke cargo check");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Debug: show the generated Cargo.toml
    let final_toml = std::fs::read_to_string(&cargo_toml_path).unwrap_or_default();
    assert!(
        output.status.success(),
        "Generated project failed `cargo check`:\n\n--- Cargo.toml ---\n{final_toml}\n--- stderr ---\n{stderr}\n--- stdout ---\n{stdout}"
    );
}

#[test]
#[ignore]
fn generated_project_files_exist() {
    let dir = tempfile::tempdir().expect("failed to create temp dir");
    let output_dir = dir.path().join("gale_build");

    let program = build_kitchen_sink_program();
    let interner = TypeInterner::new();

    codegen::generate(&program, &interner, "file_check_app", &output_dir, None)
        .expect("codegen::generate failed");

    // Verify expected files exist
    let expected_files = [
        "Cargo.toml",
        "src/main.rs",
        "src/layout.rs",
        "src/gale_ssr.rs",
        "src/actions/create_user.rs",
        "src/actions/mod.rs",
        "src/guards/user_form.rs",
        "src/guards/mod.rs",
        "src/channels/chat.rs",
        "src/channels/mod.rs",
        "src/api/users.rs",
        "src/api/mod.rs",
        "src/routes/home_page.rs",
        "src/routes/user_by_id.rs",
        "src/routes/mod.rs",
        "src/shared/status.rs",
        "src/shared/validation.rs",
        "src/shared/mod.rs",
    ];

    for file in &expected_files {
        let path = output_dir.join(file);
        assert!(
            path.exists(),
            "Expected generated file missing: {file} (checked: {})",
            path.display()
        );
    }
}
