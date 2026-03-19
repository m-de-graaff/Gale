//! Rust source minifier for generated code.
//!
//! Strips doc-comments, file headers, and blank lines from the generated
//! `.rs` files.  This has no effect on the compiled binary but reduces
//! the on-disk footprint of `.gale_dev/src/`.

/// Minify generated Rust source by stripping comments and blank lines.
pub fn minify_rs(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut prev_blank = false;

    for line in src.lines() {
        let trimmed = line.trim();

        // Skip doc-comment headers  (//! ...)
        if trimmed.starts_with("//!") {
            continue;
        }

        // Skip regular comments  (// ...)
        if trimmed.starts_with("//") {
            continue;
        }

        // Collapse multiple blank lines into zero
        if trimmed.is_empty() {
            if prev_blank {
                continue;
            }
            prev_blank = true;
            // Skip blank lines entirely (don't even emit one)
            continue;
        }

        prev_blank = false;
        out.push_str(line);
        out.push('\n');
    }

    // Remove trailing whitespace
    while out.ends_with('\n') {
        out.pop();
    }
    out.push('\n');
    out
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_doc_comments() {
        let src = "//! File header.\n//! Generated.\n\nfn main() {}\n";
        let out = minify_rs(src);
        assert!(!out.contains("//!"));
        assert!(out.contains("fn main()"));
    }

    #[test]
    fn strips_line_comments() {
        let src = "// This is a comment\nfn foo() {}\n";
        let out = minify_rs(src);
        assert!(!out.contains("// This"));
        assert!(out.contains("fn foo()"));
    }

    #[test]
    fn collapses_blank_lines() {
        let src = "fn a() {}\n\n\n\nfn b() {}\n";
        let out = minify_rs(src);
        assert!(!out.contains("\n\n"));
    }
}
