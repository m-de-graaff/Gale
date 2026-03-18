//! Code actions and quick fixes.

use lsp_types::{CodeAction, CodeActionKind, Range, TextEdit, Url, WorkspaceEdit};

/// Provide code actions for diagnostics in the given range.
pub fn provide_code_actions(uri: &Url, diagnostics: &[lsp_types::Diagnostic]) -> Vec<CodeAction> {
    let mut actions = Vec::new();

    for diag in diagnostics {
        // Missing alt on img → add alt=""
        if diag.message.contains("alt") && diag.message.contains("img") {
            actions.push(CodeAction {
                title: "Add alt=\"\" attribute".into(),
                kind: Some(CodeActionKind::QUICKFIX),
                diagnostics: Some(vec![diag.clone()]),
                edit: Some(WorkspaceEdit {
                    changes: Some(
                        [(
                            uri.clone(),
                            vec![TextEdit {
                                range: Range {
                                    start: diag.range.end,
                                    end: diag.range.end,
                                },
                                new_text: " alt=\"\"".into(),
                            }],
                        )]
                        .into(),
                    ),
                    ..Default::default()
                }),
                ..Default::default()
            });
        }

        // Unused signal → prefix with _
        if diag.message.contains("unused") && diag.message.contains("signal") {
            if let Some(name) = extract_name_from_message(&diag.message) {
                actions.push(CodeAction {
                    title: format!("Prefix with underscore: _{name}"),
                    kind: Some(CodeActionKind::QUICKFIX),
                    diagnostics: Some(vec![diag.clone()]),
                    edit: Some(WorkspaceEdit {
                        changes: Some(
                            [(
                                uri.clone(),
                                vec![TextEdit {
                                    range: diag.range,
                                    new_text: format!("_{name}"),
                                }],
                            )]
                            .into(),
                        ),
                        ..Default::default()
                    }),
                    ..Default::default()
                });
            }
        }

        // Missing key on each → add key directive
        if diag.message.contains("key") && diag.message.contains("each") {
            actions.push(CodeAction {
                title: "Add key={item.id} directive".into(),
                kind: Some(CodeActionKind::QUICKFIX),
                diagnostics: Some(vec![diag.clone()]),
                ..Default::default()
            });
        }
    }

    actions
}

/// Extract a quoted name from an error message like "signal `foo` is unused".
fn extract_name_from_message(msg: &str) -> Option<String> {
    let start = msg.find('`')? + 1;
    let end = msg[start..].find('`')? + start;
    Some(msg[start..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_name() {
        assert_eq!(
            extract_name_from_message("signal `count` is declared but never read"),
            Some("count".into())
        );
        assert_eq!(extract_name_from_message("no backticks here"), None);
    }
}
