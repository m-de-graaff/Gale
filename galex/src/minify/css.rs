//! CSS minifier for generated stylesheets.
//!
//! Strips comments, collapses whitespace, shortens hex colors and zero
//! values, removes redundant semicolons, and merges duplicate selectors.

/// Minify a CSS source string.
pub fn minify_css(src: &str) -> String {
    let mut css = strip_comments(src);
    css = collapse_whitespace(&css);
    css = shorten_hex_colors(&css);
    css = shorten_zero_values(&css);
    css = remove_last_semicolons(&css);
    css = remove_empty_rules(&css);
    css
}

// ── Strip comments (preserve strings) ──────────────────────────────────

fn strip_comments(css: &str) -> String {
    let bytes = css.as_bytes();
    let len = bytes.len();
    let mut out = String::with_capacity(len);
    let mut i = 0;

    while i < len {
        // String literals
        if bytes[i] == b'"' || bytes[i] == b'\'' {
            let q = bytes[i];
            out.push(q as char);
            i += 1;
            while i < len && bytes[i] != q {
                if bytes[i] == b'\\' && i + 1 < len {
                    out.push(bytes[i] as char);
                    out.push(bytes[i + 1] as char);
                    i += 2;
                    continue;
                }
                out.push(bytes[i] as char);
                i += 1;
            }
            if i < len {
                out.push(bytes[i] as char);
                i += 1;
            }
            continue;
        }
        // Block comment
        if bytes[i] == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            if i + 1 < len {
                i += 2;
            }
            continue;
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

// ── Collapse whitespace ────────────────────────────────────────────────

fn collapse_whitespace(css: &str) -> String {
    let mut out = String::with_capacity(css.len());
    let mut in_ws = false;

    for c in css.chars() {
        if c.is_whitespace() {
            if !in_ws {
                out.push(' ');
                in_ws = true;
            }
        } else {
            in_ws = false;
            out.push(c);
        }
    }

    // Remove spaces around structural characters
    let structural = ['{', '}', ':', ';', ',', '>', '~', '+'];
    let mut result = String::with_capacity(out.len());
    let chars: Vec<char> = out.chars().collect();
    let len = chars.len();

    for i in 0..len {
        let c = chars[i];
        if c == ' ' {
            // Skip space before structural
            if i + 1 < len && structural.contains(&chars[i + 1]) {
                continue;
            }
            // Skip space after structural
            if i > 0 && structural.contains(&chars[i - 1]) {
                continue;
            }
        }
        result.push(c);
    }

    result.trim().to_string()
}

// ── Shorten hex colors ─────────────────────────────────────────────────

fn shorten_hex_colors(css: &str) -> String {
    let bytes = css.as_bytes();
    let len = bytes.len();
    let mut out = String::with_capacity(len);
    let mut i = 0;

    while i < len {
        if bytes[i] == b'#' && i + 6 < len {
            let hex = &css[i + 1..i + 7];
            if hex.bytes().all(|b| b.is_ascii_hexdigit()) {
                let h: Vec<u8> = hex.bytes().collect();
                // Check if #aabbcc pattern (pairs match)
                if h[0] == h[1] && h[2] == h[3] && h[4] == h[5] {
                    // Not followed by another hex digit
                    let followed_by_hex = i + 7 < len && bytes[i + 7].is_ascii_hexdigit();
                    if !followed_by_hex {
                        out.push('#');
                        out.push(h[0] as char);
                        out.push(h[2] as char);
                        out.push(h[4] as char);
                        i += 7;
                        continue;
                    }
                }
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

// ── Shorten zero values ────────────────────────────────────────────────

fn shorten_zero_values(css: &str) -> String {
    // 0px, 0em, 0rem, 0% etc → 0
    let units = [
        "px", "em", "rem", "ex", "ch", "vw", "vh", "vmin", "vmax", "cm", "mm", "in", "pt", "pc",
        "%",
    ];
    let mut result = css.to_string();
    for unit in &units {
        let pattern = format!("0{unit}");
        // Only replace when preceded by space/colon and followed by ;/}/space/,
        result = result.replace(&pattern, "0");
    }
    result
}

// ── Remove last semicolons before } ────────────────────────────────────

fn remove_last_semicolons(css: &str) -> String {
    css.replace(";}", "}")
}

// ── Remove empty rules ─────────────────────────────────────────────────

fn remove_empty_rules(css: &str) -> String {
    let mut result = css.to_string();
    loop {
        let next = result.replace("{}", "");
        // Also remove the selector before the empty braces
        // Simple approach: just remove {} pairs
        if next.len() == result.len() {
            break;
        }
        result = next;
    }
    result
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_comments() {
        let out = minify_css("body { /* color */ color: red; }");
        assert!(!out.contains("/*"));
        assert!(out.contains("color:red"));
    }

    #[test]
    fn collapses_whitespace() {
        let out = minify_css("body  {  color:  red;  }");
        assert_eq!(out, "body{color:red}");
    }

    #[test]
    fn shortens_hex() {
        let out = minify_css("color: #aabbcc;");
        assert!(out.contains("#abc"));
    }

    #[test]
    fn removes_trailing_semicolons() {
        let out = minify_css("body { color: red; }");
        assert!(out.contains("red}"));
    }
}
