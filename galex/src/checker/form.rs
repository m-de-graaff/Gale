//! Form system validation (GX1500-GX1507).
//!
//! Validates form directives in templates, ensuring proper combinations
//! of `form:action` and `form:guard` directives.

use crate::ast::*;
use crate::errors::{codes, Diagnostic};

/// Validate form directives in templates.
///
/// Checks:
/// - GX1505: `<form>` with `form:action` but no `form:guard`
pub fn validate_forms(template: &[TemplateNode]) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    validate_template_forms(template, &mut diagnostics);
    diagnostics
}

fn validate_template_forms(nodes: &[TemplateNode], diagnostics: &mut Vec<Diagnostic>) {
    for node in nodes {
        match node {
            TemplateNode::Element {
                tag,
                directives,
                children,
                span,
                ..
            } => {
                if tag == "form" {
                    check_form_directives(directives, *span, diagnostics);
                }
                // Recurse into children
                validate_template_forms(children, diagnostics);
            }
            TemplateNode::When {
                body, else_branch, ..
            } => {
                validate_template_forms(body, diagnostics);
                match else_branch {
                    Some(WhenElse::Else(nodes)) => {
                        validate_template_forms(nodes, diagnostics);
                    }
                    Some(WhenElse::ElseWhen(node)) => {
                        validate_template_forms(&[*node.clone()], diagnostics);
                    }
                    None => {}
                }
            }
            TemplateNode::Each { body, empty, .. } => {
                validate_template_forms(body, diagnostics);
                if let Some(empty_nodes) = empty {
                    validate_template_forms(empty_nodes, diagnostics);
                }
            }
            TemplateNode::Suspend { body, .. } => {
                validate_template_forms(body, diagnostics);
            }
            _ => {}
        }
    }
}

/// Check form-specific directive combinations on a `<form>` element.
fn check_form_directives(
    directives: &[Directive],
    span: crate::span::Span,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let has_action = directives
        .iter()
        .any(|d| matches!(d, Directive::FormAction { .. }));
    let has_guard = directives
        .iter()
        .any(|d| matches!(d, Directive::FormGuard { .. }));

    // GX1505: form with form:action but no form:guard
    if has_action && !has_guard {
        diagnostics.push(Diagnostic::new(&codes::GX1505, span));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::span::Span;

    fn s() -> Span {
        Span::dummy()
    }

    #[test]
    fn form_with_action_no_guard() {
        let template = vec![TemplateNode::Element {
            tag: "form".into(),
            attributes: vec![],
            directives: vec![Directive::FormAction {
                action: Expr::Ident {
                    name: "submit".into(),
                    span: s(),
                },
                span: s(),
            }],
            children: vec![],
            span: s(),
        }];
        let diags = validate_forms(&template);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code.code, 1505);
    }

    #[test]
    fn form_with_action_and_guard_ok() {
        let template = vec![TemplateNode::Element {
            tag: "form".into(),
            attributes: vec![],
            directives: vec![
                Directive::FormAction {
                    action: Expr::Ident {
                        name: "submit".into(),
                        span: s(),
                    },
                    span: s(),
                },
                Directive::FormGuard {
                    guard: Expr::Ident {
                        name: "LoginForm".into(),
                        span: s(),
                    },
                    span: s(),
                },
            ],
            children: vec![],
            span: s(),
        }];
        let diags = validate_forms(&template);
        assert!(diags.is_empty());
    }

    #[test]
    fn nested_form_checked() {
        let template = vec![TemplateNode::Element {
            tag: "div".into(),
            attributes: vec![],
            directives: vec![],
            children: vec![TemplateNode::Element {
                tag: "form".into(),
                attributes: vec![],
                directives: vec![Directive::FormAction {
                    action: Expr::Ident {
                        name: "doSomething".into(),
                        span: s(),
                    },
                    span: s(),
                }],
                children: vec![],
                span: s(),
            }],
            span: s(),
        }];
        let diags = validate_forms(&template);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code.code, 1505);
    }
}
