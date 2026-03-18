//! Head/SEO validation (GX1400-GX1408).
//!
//! Validates head blocks in components for SEO best practices,
//! including missing title/description and length warnings.

use crate::ast::*;
use crate::errors::{codes, Diagnostic};

/// Known valid head property names.
const KNOWN_HEAD_PROPS: &[&str] = &[
    "title",
    "description",
    "charset",
    "viewport",
    "canonical",
    "og",
    "twitter",
    "robots",
    "favicon",
    "css",
    "script",
];

/// Maximum recommended title length for SEO.
const MAX_TITLE_LEN: usize = 60;

/// Maximum recommended description length for SEO.
const MAX_DESCRIPTION_LEN: usize = 160;

/// Validate head blocks in components.
///
/// Checks:
/// - GX1400: Unknown head properties
/// - GX1403: Missing title (warning)
/// - GX1404: Missing description (warning)
/// - GX1405: Title too long (warning)
/// - GX1406: Description too long (warning)
pub fn validate_head(head: &HeadBlock) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    let has_title = head.fields.iter().any(|f| f.key == "title");
    let has_description = head.fields.iter().any(|f| f.key == "description");

    // GX1403: Missing title
    if !has_title {
        diagnostics.push(Diagnostic::new(&codes::GX1403, head.span));
    }

    // GX1404: Missing description
    if !has_description {
        diagnostics.push(Diagnostic::new(&codes::GX1404, head.span));
    }

    for field in &head.fields {
        // GX1400: Unknown head property
        if !KNOWN_HEAD_PROPS.contains(&field.key.as_str()) {
            diagnostics.push(Diagnostic::with_message(
                &codes::GX1400,
                format!("Unknown head property `{}`", field.key),
                field.span,
            ));
        }

        // GX1405: Title too long
        if field.key == "title" {
            if let Expr::StringLit { value, .. } = &field.value {
                if value.len() > MAX_TITLE_LEN {
                    diagnostics.push(Diagnostic::with_message(
                        &codes::GX1405,
                        format!(
                            "`head.title` is {} characters (recommended max {})",
                            value.len(),
                            MAX_TITLE_LEN
                        ),
                        field.span,
                    ));
                }
            }
        }

        // GX1406: Description too long
        if field.key == "description" {
            if let Expr::StringLit { value, .. } = &field.value {
                if value.len() > MAX_DESCRIPTION_LEN {
                    diagnostics.push(Diagnostic::with_message(
                        &codes::GX1406,
                        format!(
                            "`head.description` is {} characters (recommended max {})",
                            value.len(),
                            MAX_DESCRIPTION_LEN
                        ),
                        field.span,
                    ));
                }
            }
        }
    }

    diagnostics
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::span::Span;

    fn s() -> Span {
        Span::dummy()
    }

    #[test]
    fn missing_title_and_description() {
        let head = HeadBlock {
            fields: vec![],
            span: s(),
        };
        let diags = validate_head(&head);
        assert!(diags.iter().any(|d| d.code.code == 1403));
        assert!(diags.iter().any(|d| d.code.code == 1404));
    }

    #[test]
    fn unknown_property() {
        let head = HeadBlock {
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
                        value: "A page".into(),
                        span: s(),
                    },
                    span: s(),
                },
                HeadField {
                    key: "bogus".into(),
                    value: Expr::StringLit {
                        value: "x".into(),
                        span: s(),
                    },
                    span: s(),
                },
            ],
            span: s(),
        };
        let diags = validate_head(&head);
        assert!(diags.iter().any(|d| d.code.code == 1400));
    }

    #[test]
    fn title_too_long() {
        let long_title = "A".repeat(80);
        let head = HeadBlock {
            fields: vec![
                HeadField {
                    key: "title".into(),
                    value: Expr::StringLit {
                        value: long_title.into(),
                        span: s(),
                    },
                    span: s(),
                },
                HeadField {
                    key: "description".into(),
                    value: Expr::StringLit {
                        value: "ok".into(),
                        span: s(),
                    },
                    span: s(),
                },
            ],
            span: s(),
        };
        let diags = validate_head(&head);
        assert!(diags.iter().any(|d| d.code.code == 1405));
    }

    #[test]
    fn description_too_long() {
        let long_desc = "B".repeat(200);
        let head = HeadBlock {
            fields: vec![
                HeadField {
                    key: "title".into(),
                    value: Expr::StringLit {
                        value: "T".into(),
                        span: s(),
                    },
                    span: s(),
                },
                HeadField {
                    key: "description".into(),
                    value: Expr::StringLit {
                        value: long_desc.into(),
                        span: s(),
                    },
                    span: s(),
                },
            ],
            span: s(),
        };
        let diags = validate_head(&head);
        assert!(diags.iter().any(|d| d.code.code == 1406));
    }

    #[test]
    fn valid_head_no_warnings() {
        let head = HeadBlock {
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
                        value: "A good page".into(),
                        span: s(),
                    },
                    span: s(),
                },
            ],
            span: s(),
        };
        let diags = validate_head(&head);
        assert!(diags.is_empty());
    }
}
