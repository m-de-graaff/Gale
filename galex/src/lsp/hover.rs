//! Hover information provider.

use lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind};

use super::document::DocumentManager;
use super::position::{self, NodeInfo};

/// Provide hover information at the given byte offset.
pub fn provide_hover(docs: &DocumentManager, file_id: u32, offset: u32) -> Option<Hover> {
    let program = docs.merged_program()?;
    let node = position::node_at_offset(program, file_id, offset)?;
    let checker = docs.cached_checker.as_ref();

    let content = match node {
        NodeInfo::Ident { ref name, .. } => {
            // Look up the binding type
            if let Some(checker) = checker {
                if let Some(binding) = checker.env.lookup(name) {
                    let type_str = checker.interner.display(binding.ty);
                    let kind_str = format!("{:?}", binding.kind).to_lowercase();
                    format!("```gale\n{kind_str} {name}: {type_str}\n```")
                } else {
                    format!("`{name}` — unresolved")
                }
            } else {
                format!("`{name}`")
            }
        }
        NodeInfo::Decl {
            ref name, ref kind, ..
        } => {
            let kind_str = format!("{kind:?}").to_lowercase();
            if let Some(checker) = checker {
                if let Some(binding) = checker.env.lookup(name) {
                    let type_str = checker.interner.display(binding.ty);
                    format!("```gale\n{kind_str} {name}: {type_str}\n```")
                } else {
                    format!("```gale\n{kind_str} {name}\n```")
                }
            } else {
                format!("```gale\n{kind_str} {name}\n```")
            }
        }
        NodeInfo::TypeRef { ref name, .. } => {
            if let Some(checker) = checker {
                if let Some(ty_id) = checker.env.resolve_type(name) {
                    let type_str = checker.interner.display(ty_id);
                    format!("```gale\ntype {name} = {type_str}\n```")
                } else {
                    format!("`{name}` — type")
                }
            } else {
                format!("`{name}` — type")
            }
        }
        NodeInfo::HtmlTag { ref tag, .. } => {
            format!("`<{tag}>` — HTML element")
        }
        NodeInfo::DirectiveRef { ref kind, .. } => {
            let desc = match kind.as_str() {
                "bind" => "Two-way binding to a signal",
                "on" => "Event handler",
                "class" => "Conditional CSS class toggle",
                "ref" => "DOM element reference",
                "transition" => "CSS transition animation",
                "key" => "Keyed list item identifier",
                "form:action" => "Form submission action",
                "form:guard" => "Form validation guard",
                _ => "Directive",
            };
            format!("**{kind}:** {desc}")
        }
        NodeInfo::ExprNode { .. } => {
            // Re-infer on demand would go here
            "expression".to_string()
        }
    };

    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: content,
        }),
        range: None,
    })
}
