//! Document symbols, folding ranges, semantic tokens, and formatting.

use lsp_types::{
    DocumentSymbol, FoldingRange, FoldingRangeKind, Position, Range, SymbolKind, TextEdit,
};

use crate::ast::*;

/// Extract document symbols (outline) from a program.
pub fn document_symbols(program: &Program) -> Vec<DocumentSymbol> {
    let mut symbols = Vec::new();
    for item in &program.items {
        if let Some(sym) = symbol_for_item(item) {
            symbols.push(sym);
        }
    }
    symbols
}

fn symbol_for_item(item: &Item) -> Option<DocumentSymbol> {
    match item {
        Item::ComponentDecl(d) => Some(make_symbol(
            &d.name,
            SymbolKind::CLASS,
            &d.span,
            &d.body.span,
        )),
        Item::LayoutDecl(d) => Some(make_symbol(
            &d.name,
            SymbolKind::CLASS,
            &d.span,
            &d.body.span,
        )),
        Item::FnDecl(d) => Some(make_symbol(&d.name, SymbolKind::FUNCTION, &d.span, &d.span)),
        Item::GuardDecl(d) => Some(make_symbol(&d.name, SymbolKind::STRUCT, &d.span, &d.span)),
        Item::StoreDecl(d) => Some(make_symbol(&d.name, SymbolKind::MODULE, &d.span, &d.span)),
        Item::ActionDecl(d) => Some(make_symbol(&d.name, SymbolKind::FUNCTION, &d.span, &d.span)),
        Item::QueryDecl(d) => Some(make_symbol(
            &d.name,
            SymbolKind::INTERFACE,
            &d.span,
            &d.span,
        )),
        Item::ChannelDecl(d) => Some(make_symbol(
            &d.name,
            SymbolKind::INTERFACE,
            &d.span,
            &d.span,
        )),
        Item::EnumDecl(d) => Some(make_symbol(&d.name, SymbolKind::ENUM, &d.span, &d.span)),
        Item::TypeAlias(d) => Some(make_symbol(
            &d.name,
            SymbolKind::TYPE_PARAMETER,
            &d.span,
            &d.span,
        )),
        Item::ApiDecl(d) => Some(make_symbol(
            &d.name,
            SymbolKind::NAMESPACE,
            &d.span,
            &d.span,
        )),
        Item::MiddlewareDecl(d) => {
            Some(make_symbol(&d.name, SymbolKind::FUNCTION, &d.span, &d.span))
        }
        Item::TestDecl(d) => Some(make_symbol(&d.name, SymbolKind::METHOD, &d.span, &d.span)),
        Item::Out(out) => symbol_for_item(&out.inner),
        _ => None,
    }
}

#[allow(deprecated)]
fn make_symbol(
    name: &str,
    kind: SymbolKind,
    full_span: &crate::span::Span,
    sel_span: &crate::span::Span,
) -> DocumentSymbol {
    DocumentSymbol {
        name: name.to_string(),
        detail: None,
        kind,
        tags: None,
        deprecated: None,
        range: span_to_range(full_span),
        selection_range: span_to_range(sel_span),
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

fn span_to_range(span: &crate::span::Span) -> Range {
    let start = Position {
        line: span.line.saturating_sub(1),
        character: span.col.saturating_sub(1),
    };
    // Use start as end for selection range (approximate)
    Range { start, end: start }
}
