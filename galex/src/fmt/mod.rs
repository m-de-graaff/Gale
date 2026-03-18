//! GaleX code formatter.
//!
//! Parses source → AST → pretty-prints back to formatted source.

pub mod printer;

/// Format a GaleX source file.
///
/// Returns the formatted source text, or errors if parsing fails.
pub fn format_source(source: &str, file_id: u32) -> Result<String, Vec<String>> {
    let result = crate::parser::parse(source, file_id);
    if !result.is_ok() {
        return Err(result.errors());
    }
    Ok(printer::print_program(&result.program))
}
