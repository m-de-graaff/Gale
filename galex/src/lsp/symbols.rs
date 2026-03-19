//! Document symbols, folding ranges, semantic tokens, and formatting.

use lsp_types::{
    DocumentSymbol, FoldingRange, FoldingRangeKind, Position, Range, SymbolKind, TextEdit,
};

use crate::ast::*;

/// Extract document symbols (outline) from a program.
///
/// `source` is needed to compute proper end positions for spans.
pub fn document_symbols(program: &Program, source: &str) -> Vec<DocumentSymbol> {
    let mut symbols = Vec::new();
    for item in &program.items {
        if let Some(sym) = symbol_for_item(item, source) {
            symbols.push(sym);
        }
    }
    symbols
}

fn symbol_for_item(item: &Item, source: &str) -> Option<DocumentSymbol> {
    match item {
        Item::ComponentDecl(d) => Some(make_symbol(&d.name, SymbolKind::CLASS, &d.span, source)),
        Item::LayoutDecl(d) => Some(make_symbol(&d.name, SymbolKind::CLASS, &d.span, source)),
        Item::FnDecl(d) => Some(make_symbol(&d.name, SymbolKind::FUNCTION, &d.span, source)),
        Item::GuardDecl(d) => Some(make_symbol(&d.name, SymbolKind::STRUCT, &d.span, source)),
        Item::StoreDecl(d) => Some(make_symbol(&d.name, SymbolKind::MODULE, &d.span, source)),
        Item::ActionDecl(d) => Some(make_symbol(&d.name, SymbolKind::FUNCTION, &d.span, source)),
        Item::QueryDecl(d) => Some(make_symbol(&d.name, SymbolKind::INTERFACE, &d.span, source)),
        Item::ChannelDecl(d) => Some(make_symbol(&d.name, SymbolKind::INTERFACE, &d.span, source)),
        Item::EnumDecl(d) => Some(make_symbol(&d.name, SymbolKind::ENUM, &d.span, source)),
        Item::TypeAlias(d) => Some(make_symbol(
            &d.name,
            SymbolKind::TYPE_PARAMETER,
            &d.span,
            source,
        )),
        Item::ApiDecl(d) => Some(make_symbol(&d.name, SymbolKind::NAMESPACE, &d.span, source)),
        Item::MiddlewareDecl(d) => {
            Some(make_symbol(&d.name, SymbolKind::FUNCTION, &d.span, source))
        }
        Item::TestDecl(d) => Some(make_symbol(&d.name, SymbolKind::METHOD, &d.span, source)),
        Item::Out(out) => symbol_for_item(&out.inner, source),
        _ => None,
    }
}

#[allow(deprecated)]
fn make_symbol(
    name: &str,
    kind: SymbolKind,
    span: &crate::span::Span,
    source: &str,
) -> DocumentSymbol {
    let range = span_to_range(span, source);
    DocumentSymbol {
        name: name.to_string(),
        detail: None,
        kind,
        tags: None,
        deprecated: None,
        range,
        // selectionRange must be contained in range — use the full range
        // for both since the AST does not store a separate name span.
        selection_range: range,
        children: None,
    }
}

/// Extract folding ranges from a program.
pub fn folding_ranges(program: &Program, source: &str) -> Vec<FoldingRange> {
    let mut ranges = Vec::new();
    for item in &program.items {
        fold_item(item, source, &mut ranges);
    }
    ranges
}

fn fold_item(item: &Item, source: &str, ranges: &mut Vec<FoldingRange>) {
    match item {
        Item::ComponentDecl(d) => {
            add_folding(&d.span, source, FoldingRangeKind::Region, ranges);
        }
        Item::LayoutDecl(d) => {
            add_folding(&d.span, source, FoldingRangeKind::Region, ranges);
        }
        Item::GuardDecl(d) => {
            add_folding(&d.span, source, FoldingRangeKind::Region, ranges);
        }
        Item::StoreDecl(d) => {
            add_folding(&d.span, source, FoldingRangeKind::Region, ranges);
        }
        Item::ServerBlock(b) | Item::ClientBlock(b) | Item::SharedBlock(b) => {
            add_folding(&b.span, source, FoldingRangeKind::Region, ranges);
            for inner in &b.items {
                fold_item(inner, source, ranges);
            }
        }
        _ => {}
    }
}

fn add_folding(
    span: &crate::span::Span,
    source: &str,
    kind: FoldingRangeKind,
    ranges: &mut Vec<FoldingRange>,
) {
    let (end_line, _) = span.end_position(source);
    if end_line > span.line {
        ranges.push(FoldingRange {
            start_line: span.line.saturating_sub(1),
            start_character: None,
            end_line: end_line.saturating_sub(1),
            end_character: None,
            kind: Some(kind),
            collapsed_text: None,
        });
    }
}

/// Format a document using the GaleX formatter.
pub fn format_document(source: &str) -> Option<Vec<TextEdit>> {
    let formatted = crate::fmt::format_source(source, 0).ok()?;
    if formatted == source {
        return Some(vec![]); // Already formatted
    }
    // Replace the entire document
    let line_count = source.lines().count() as u32;
    Some(vec![TextEdit {
        range: Range {
            start: Position {
                line: 0,
                character: 0,
            },
            end: Position {
                line: line_count + 1,
                character: 0,
            },
        },
        new_text: formatted,
    }])
}

fn span_to_range(span: &crate::span::Span, source: &str) -> Range {
    let start = Position {
        line: span.line.saturating_sub(1),
        character: span.col.saturating_sub(1),
    };
    let (end_line, end_col) = span.end_position(source);
    let end = Position {
        line: end_line.saturating_sub(1),
        character: end_col.saturating_sub(1),
    };
    Range { start, end }
}
