//! Class extraction from GaleX AST and source files.
//!
//! Extracts CSS class names that Tailwind's content scanner might miss:
//! - `class:bg-blue-500={active}` — the `class:` prefix confuses Tailwind
//! - `transition:fade` — GaleX-specific transition classes
//! - Dynamic class expressions (best-effort)
//!
//! The extracted classes form a "safelist" passed to Tailwind's config
//! to ensure they are always included in the output CSS.

use std::collections::BTreeSet;

use crate::ast::*;

/// Extract a deduplicated safelist of CSS classes from parsed programs.
///
/// These are classes that Tailwind's own content scanner would miss because
/// they appear in GaleX-specific syntax (`class:` directives, `transition:`
/// directives) rather than plain `class="..."` strings.
pub fn generate_safelist(programs: &[(u32, Program)]) -> Vec<String> {
    let mut classes = BTreeSet::new();

    for (_, program) in programs {
        for item in &program.items {
            extract_from_item(item, &mut classes);
        }
    }

    // Always include built-in transition classes
    for transition_type in &["fade", "slide", "scale", "blur"] {
        classes.insert(format!("gale-{transition_type}-enter"));
        classes.insert(format!("gale-{transition_type}-enter-active"));
        classes.insert(format!("gale-{transition_type}-exit-active"));
    }

    classes.into_iter().collect()
}

/// Extract all class names from a single program (for testing).
pub fn extract_classes_from_program(program: &Program) -> Vec<String> {
    let mut classes = BTreeSet::new();
    for item in &program.items {
        extract_from_item(item, &mut classes);
    }
    classes.into_iter().collect()
}

// ── Item/template walking ──────────────────────────────────────────────

fn extract_from_item(item: &Item, classes: &mut BTreeSet<String>) {
    match item {
        Item::ComponentDecl(decl) => {
            extract_from_template(&decl.body.template, classes);
        }
        Item::LayoutDecl(decl) => {
            extract_from_template(&decl.body.template, classes);
        }
        Item::Out(out) => {
            extract_from_item(&out.inner, classes);
        }
        Item::ServerBlock(block) | Item::ClientBlock(block) | Item::SharedBlock(block) => {
            for inner in &block.items {
                extract_from_item(inner, classes);
            }
        }
        _ => {}
    }
}

fn extract_from_template(nodes: &[TemplateNode], classes: &mut BTreeSet<String>) {
    for node in nodes {
        match node {
            TemplateNode::Element {
                attributes,
                directives,
                children,
                ..
            } => {
                extract_from_attrs_directives(attributes, directives, classes);
                extract_from_template(children, classes);
            }
            TemplateNode::SelfClosing {
                attributes,
                directives,
                ..
            } => {
                extract_from_attrs_directives(attributes, directives, classes);
            }
            TemplateNode::When {
                body, else_branch, ..
            } => {
                extract_from_template(body, classes);
                if let Some(WhenElse::Else(nodes)) = else_branch {
                    extract_from_template(nodes, classes);
                }
                if let Some(WhenElse::ElseWhen(node)) = else_branch {
                    extract_from_template(&[*node.clone()], classes);
                }
            }
            TemplateNode::Each { body, empty, .. } => {
                extract_from_template(body, classes);
                if let Some(nodes) = empty {
                    extract_from_template(nodes, classes);
                }
            }
            TemplateNode::Suspend { body, .. } => {
                extract_from_template(body, classes);
            }
            _ => {}
        }
    }
}

fn extract_from_attrs_directives(
    attributes: &[Attribute],
    directives: &[Directive],
    classes: &mut BTreeSet<String>,
) {
    // Extract from static class="..." attributes
    for attr in attributes {
        if attr.name == "class" {
            if let AttrValue::String(value) = &attr.value {
                for class in value.split_whitespace() {
                    classes.insert(class.to_string());
                }
            }
        }
    }

    // Extract from class: directives (Tailwind scanner misses these)
    for directive in directives {
        match directive {
            Directive::Class { name, .. } => {
                classes.insert(name.to_string());
            }
            Directive::Transition { kind, .. } => {
                // Add the GaleX transition class names
                classes.insert(format!("gale-{kind}-enter"));
                classes.insert(format!("gale-{kind}-enter-active"));
                classes.insert(format!("gale-{kind}-exit-active"));
            }
            _ => {}
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::span::Span;

    fn s() -> Span {
        Span::dummy()
    }

    fn make_component(template: Vec<TemplateNode>) -> Program {
        Program {
            items: vec![Item::ComponentDecl(ComponentDecl {
                name: "Test".into(),
                props: vec![],
                body: ComponentBody {
                    stmts: vec![],
                    template,
                    head: None,
                    span: s(),
                },
                span: s(),
            })],
            span: s(),
        }
    }

    #[test]
    fn extracts_static_classes() {
        let program = make_component(vec![TemplateNode::Element {
            tag: "div".into(),
            attributes: vec![Attribute {
                name: "class".into(),
                value: AttrValue::String("flex items-center gap-4".into()),
                span: s(),
            }],
            directives: vec![],
            children: vec![],
            span: s(),
        }]);
        let classes = extract_classes_from_program(&program);
        assert!(classes.contains(&"flex".to_string()));
        assert!(classes.contains(&"items-center".to_string()));
        assert!(classes.contains(&"gap-4".to_string()));
    }

    #[test]
    fn extracts_class_directives() {
        let program = make_component(vec![TemplateNode::Element {
            tag: "div".into(),
            attributes: vec![],
            directives: vec![
                Directive::Class {
                    name: "bg-blue-500".into(),
                    condition: Expr::BoolLit {
                        value: true,
                        span: s(),
                    },
                    span: s(),
                },
                Directive::Class {
                    name: "text-white".into(),
                    condition: Expr::BoolLit {
                        value: true,
                        span: s(),
                    },
                    span: s(),
                },
            ],
            children: vec![],
            span: s(),
        }]);
        let classes = extract_classes_from_program(&program);
        assert!(classes.contains(&"bg-blue-500".to_string()));
        assert!(classes.contains(&"text-white".to_string()));
    }

    #[test]
    fn extracts_transition_classes() {
        let program = make_component(vec![TemplateNode::Element {
            tag: "div".into(),
            attributes: vec![],
            directives: vec![Directive::Transition {
                kind: "fade".into(),
                config: None,
                span: s(),
            }],
            children: vec![],
            span: s(),
        }]);
        let classes = extract_classes_from_program(&program);
        assert!(classes.contains(&"gale-fade-enter".to_string()));
        assert!(classes.contains(&"gale-fade-enter-active".to_string()));
        assert!(classes.contains(&"gale-fade-exit-active".to_string()));
    }

    #[test]
    fn deduplicates_classes() {
        let program = make_component(vec![TemplateNode::Element {
            tag: "div".into(),
            attributes: vec![Attribute {
                name: "class".into(),
                value: AttrValue::String("flex flex".into()),
                span: s(),
            }],
            directives: vec![],
            children: vec![],
            span: s(),
        }]);
        let classes = extract_classes_from_program(&program);
        let flex_count = classes.iter().filter(|c| *c == "flex").count();
        assert_eq!(flex_count, 1, "should deduplicate");
    }

    #[test]
    fn walks_nested_children() {
        let program = make_component(vec![TemplateNode::Element {
            tag: "div".into(),
            attributes: vec![Attribute {
                name: "class".into(),
                value: AttrValue::String("parent".into()),
                span: s(),
            }],
            directives: vec![],
            children: vec![TemplateNode::Element {
                tag: "span".into(),
                attributes: vec![Attribute {
                    name: "class".into(),
                    value: AttrValue::String("child".into()),
                    span: s(),
                }],
                directives: vec![],
                children: vec![],
                span: s(),
            }],
            span: s(),
        }]);
        let classes = extract_classes_from_program(&program);
        assert!(classes.contains(&"parent".to_string()));
        assert!(classes.contains(&"child".to_string()));
    }

    #[test]
    fn safelist_includes_all_builtin_transitions() {
        let programs = vec![];
        let safelist = generate_safelist(&programs);
        for t in &["fade", "slide", "scale", "blur"] {
            assert!(
                safelist.contains(&format!("gale-{t}-enter")),
                "missing {t}-enter"
            );
            assert!(
                safelist.contains(&format!("gale-{t}-enter-active")),
                "missing {t}-enter-active"
            );
            assert!(
                safelist.contains(&format!("gale-{t}-exit-active")),
                "missing {t}-exit-active"
            );
        }
    }

    #[test]
    fn extracts_from_when_blocks() {
        let program = make_component(vec![TemplateNode::When {
            condition: Expr::BoolLit {
                value: true,
                span: s(),
            },
            body: vec![TemplateNode::Element {
                tag: "div".into(),
                attributes: vec![Attribute {
                    name: "class".into(),
                    value: AttrValue::String("shown".into()),
                    span: s(),
                }],
                directives: vec![],
                children: vec![],
                span: s(),
            }],
            else_branch: Some(WhenElse::Else(vec![TemplateNode::Element {
                tag: "div".into(),
                attributes: vec![Attribute {
                    name: "class".into(),
                    value: AttrValue::String("hidden".into()),
                    span: s(),
                }],
                directives: vec![],
                children: vec![],
                span: s(),
            }])),
            span: s(),
        }]);
        let classes = extract_classes_from_program(&program);
        assert!(classes.contains(&"shown".to_string()));
        assert!(classes.contains(&"hidden".to_string()));
    }

    #[test]
    fn extracts_from_self_closing() {
        let program = make_component(vec![TemplateNode::SelfClosing {
            tag: "img".into(),
            attributes: vec![Attribute {
                name: "class".into(),
                value: AttrValue::String("w-full rounded".into()),
                span: s(),
            }],
            directives: vec![],
            span: s(),
        }]);
        let classes = extract_classes_from_program(&program);
        assert!(classes.contains(&"w-full".to_string()));
        assert!(classes.contains(&"rounded".to_string()));
    }
}
