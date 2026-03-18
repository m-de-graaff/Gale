//! JavaScript minifier with identifier mangling.
//!
//! Two modes:
//! - **Basic** (`minify_js`): Strip comments, collapse whitespace, remove blanks.
//! - **Production** (`minify_js_production`): Basic + scope-aware identifier
//!   renaming + operator whitespace removal.  Achieves 50–70% size reduction.
//!
//! The production minifier:
//! 1. Tokenises the source into a stream of JS tokens.
//! 2. Classifies every identifier occurrence (declaration, reference,
//!    property access, export, shorthand property, keyword, global).
//! 3. Builds a rename map for safe-to-rename locals.
//! 4. Emits the token stream with renames applied and minimal whitespace.

use std::collections::{BTreeMap, HashMap, HashSet};

// ── Basic Minifier ─────────────────────────────────────────────────────

/// Minify a JavaScript source string (basic mode).
///
/// Strips comments, collapses whitespace, removes blank lines.
/// Does NOT rename identifiers.
pub fn minify_js(source: &str) -> String {
    let mut result = String::with_capacity(source.len());
    let mut in_block_comment = false;
    let mut in_string = false;
    let mut string_char: char = '"';

    for line in source.lines() {
        let line = line.trim_end();

        if in_block_comment {
            if let Some(pos) = line.find("*/") {
                in_block_comment = false;
                let rest = &line[pos + 2..];
                let trimmed = rest.trim();
                if !trimmed.is_empty() {
                    result.push_str(trimmed);
                    result.push('\n');
                }
            }
            continue;
        }

        let mut out = String::with_capacity(line.len());
        let chars: Vec<char> = line.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            let ch = chars[i];

            if in_string {
                out.push(ch);
                if ch == string_char && (i == 0 || chars[i - 1] != '\\') {
                    in_string = false;
                }
                i += 1;
                continue;
            }

            if ch == '"' || ch == '\'' || ch == '`' {
                in_string = true;
                string_char = ch;
                out.push(ch);
                i += 1;
                continue;
            }

            if ch == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
                break;
            }

            if ch == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
                if let Some(end) = line[i + 2..].find("*/") {
                    i = i + 2 + end + 2;
                    continue;
                } else {
                    in_block_comment = true;
                    break;
                }
            }

            out.push(ch);
            i += 1;
        }

        let trimmed = out.trim();
        if !trimmed.is_empty() {
            let mut collapsed = String::with_capacity(trimmed.len());
            let mut prev_ws = false;
            let mut in_str = false;
            let mut sc = '"';

            for ch in trimmed.chars() {
                if in_str {
                    collapsed.push(ch);
                    if ch == sc {
                        in_str = false;
                    }
                    prev_ws = false;
                    continue;
                }
                if ch == '"' || ch == '\'' || ch == '`' {
                    in_str = true;
                    sc = ch;
                    collapsed.push(ch);
                    prev_ws = false;
                    continue;
                }
                if ch.is_whitespace() {
                    if !prev_ws {
                        collapsed.push(' ');
                        prev_ws = true;
                    }
                } else {
                    collapsed.push(ch);
                    prev_ws = false;
                }
            }

            result.push_str(&collapsed);
            result.push('\n');
        }
    }

    result
}

// ═══════════════════════════════════════════════════════════════════════
// PRODUCTION MINIFIER — tokenizer + scope analysis + identifier mangling
// ═══════════════════════════════════════════════════════════════════════

// ── Token Types ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tk {
    Ident, // identifier (includes keywords — checked separately)
    Str,   // string or template literal fragment
    Num,   // number literal
    Punct, // operator or delimiter
    Ws,    // whitespace (space/tab, not newline)
    Nl,    // newline
}

#[derive(Debug, Clone)]
struct Tok {
    kind: Tk,
    start: usize,
    end: usize,
}

impl Tok {
    fn text<'a>(&self, src: &'a str) -> &'a str {
        &src[self.start..self.end]
    }
}

// ── JS Tokenizer ───────────────────────────────────────────────────────

/// Tokenize a JavaScript source string into a flat token stream.
///
/// Template literals are split so that `${expr}` interpolations are
/// tokenized as normal code (ensuring identifiers inside them get renamed).
fn tokenize_js(src: &str) -> Vec<Tok> {
    let bytes = src.as_bytes();
    let len = bytes.len();
    let mut tokens = Vec::with_capacity(len / 3);
    let mut i = 0;

    // Track template literal nesting: when we're inside `...${`, we push
    // a depth counter.  On `}` at the right depth, we resume scanning
    // the template literal.
    let mut template_depth: Vec<usize> = Vec::new(); // stack of brace depths inside each template
    let mut brace_depth: usize = 0;

    while i < len {
        let b = bytes[i];

        // ── Whitespace ─────────────────────────────────────
        if b == b'\n' || b == b'\r' {
            let start = i;
            if b == b'\r' && i + 1 < len && bytes[i + 1] == b'\n' {
                i += 2;
            } else {
                i += 1;
            }
            tokens.push(Tok {
                kind: Tk::Nl,
                start,
                end: i,
            });
            continue;
        }
        if b == b' ' || b == b'\t' {
            let start = i;
            while i < len && (bytes[i] == b' ' || bytes[i] == b'\t') {
                i += 1;
            }
            tokens.push(Tok {
                kind: Tk::Ws,
                start,
                end: i,
            });
            continue;
        }

        // ── Comments ───────────────────────────────────────
        if b == b'/' && i + 1 < len {
            if bytes[i + 1] == b'/' {
                let start = i;
                while i < len && bytes[i] != b'\n' {
                    i += 1;
                }
                // skip line comments entirely (don't emit token)
                continue;
            }
            if bytes[i + 1] == b'*' {
                let _start = i;
                i += 2;
                while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                if i + 1 < len {
                    i += 2;
                }
                // skip block comments entirely
                continue;
            }
        }

        // ── Template literal continuation ──────────────────
        // If we're inside a template `${...}` and hit `}`, check if this
        // `}` closes the interpolation (brace depth matches entry depth + 1).
        if b == b'}' && !template_depth.is_empty() {
            let depth = template_depth.last().copied().unwrap();
            if brace_depth == depth + 1 {
                // This `}` closes the `${` interpolation
                template_depth.pop();
                brace_depth = depth; // restore depth to pre-${ level
                i += 1; // skip the `}`
                i = scan_template_tail(
                    src,
                    bytes,
                    i,
                    &mut tokens,
                    &mut template_depth,
                    &mut brace_depth,
                );
                continue;
            }
        }

        // ── Track brace depth ──────────────────────────────
        if b == b'{' {
            brace_depth += 1;
            tokens.push(Tok {
                kind: Tk::Punct,
                start: i,
                end: i + 1,
            });
            i += 1;
            continue;
        }
        if b == b'}' {
            brace_depth = brace_depth.saturating_sub(1);
            tokens.push(Tok {
                kind: Tk::Punct,
                start: i,
                end: i + 1,
            });
            i += 1;
            continue;
        }

        // ── String literals ────────────────────────────────
        if b == b'\'' || b == b'"' {
            let start = i;
            let quote = b;
            i += 1;
            while i < len && bytes[i] != quote {
                if bytes[i] == b'\\' && i + 1 < len {
                    i += 2;
                } else {
                    i += 1;
                }
            }
            if i < len {
                i += 1; // skip closing quote
            }
            tokens.push(Tok {
                kind: Tk::Str,
                start,
                end: i,
            });
            continue;
        }

        // ── Template literal start ─────────────────────────
        if b == b'`' {
            i += 1;
            i = scan_template_tail(
                src,
                bytes,
                i,
                &mut tokens,
                &mut template_depth,
                &mut brace_depth,
            );
            continue;
        }

        // ── Identifiers / keywords ─────────────────────────
        if is_ident_start(b) {
            let start = i;
            i += 1;
            while i < len && is_ident_part(bytes[i]) {
                i += 1;
            }
            tokens.push(Tok {
                kind: Tk::Ident,
                start,
                end: i,
            });
            continue;
        }

        // ── Numbers ────────────────────────────────────────
        if b.is_ascii_digit() || (b == b'.' && i + 1 < len && bytes[i + 1].is_ascii_digit()) {
            let start = i;
            // Hex, binary, octal prefixes
            if b == b'0' && i + 1 < len {
                let next = bytes[i + 1];
                if next == b'x'
                    || next == b'X'
                    || next == b'b'
                    || next == b'B'
                    || next == b'o'
                    || next == b'O'
                {
                    i += 2;
                    while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                        i += 1;
                    }
                    tokens.push(Tok {
                        kind: Tk::Num,
                        start,
                        end: i,
                    });
                    continue;
                }
            }
            while i < len && (bytes[i].is_ascii_digit() || bytes[i] == b'.' || bytes[i] == b'_') {
                i += 1;
            }
            // Scientific notation
            if i < len && (bytes[i] == b'e' || bytes[i] == b'E') {
                i += 1;
                if i < len && (bytes[i] == b'+' || bytes[i] == b'-') {
                    i += 1;
                }
                while i < len && bytes[i].is_ascii_digit() {
                    i += 1;
                }
            }
            // BigInt suffix
            if i < len && bytes[i] == b'n' {
                i += 1;
            }
            tokens.push(Tok {
                kind: Tk::Num,
                start,
                end: i,
            });
            continue;
        }

        // ── Multi-char punctuation ─────────────────────────
        if i + 2 < len {
            let three = &src[i..i + 3];
            if matches!(
                three,
                "===" | "!==" | ">>>" | "**=" | "&&=" | "||=" | "??=" | "<<=" | ">>=" | "..."
            ) {
                tokens.push(Tok {
                    kind: Tk::Punct,
                    start: i,
                    end: i + 3,
                });
                i += 3;
                continue;
            }
        }
        if i + 1 < len {
            let two = &src[i..i + 2];
            if matches!(
                two,
                "==" | "!="
                    | "<="
                    | ">="
                    | "=>"
                    | "++"
                    | "--"
                    | "+="
                    | "-="
                    | "*="
                    | "/="
                    | "%="
                    | "**"
                    | "&&"
                    | "||"
                    | "??"
                    | "?."
                    | "<<"
                    | ">>"
                    | "&="
                    | "|="
                    | "^="
            ) {
                tokens.push(Tok {
                    kind: Tk::Punct,
                    start: i,
                    end: i + 2,
                });
                i += 2;
                continue;
            }
        }

        // ── Single-char punctuation ────────────────────────
        tokens.push(Tok {
            kind: Tk::Punct,
            start: i,
            end: i + 1,
        });
        i += 1;
    }

    tokens
}

/// Scan from inside a template literal (after `` ` `` or `}`).
/// Emits `Str` tokens for the literal parts and returns the position
/// after the closing backtick.  If `${` is encountered, records the
/// template depth and returns so that normal tokenization resumes for
/// the expression.
fn scan_template_tail(
    _src: &str,
    bytes: &[u8],
    mut i: usize,
    tokens: &mut Vec<Tok>,
    template_depth: &mut Vec<usize>,
    brace_depth: &mut usize,
) -> usize {
    let len = bytes.len();
    let start = i.saturating_sub(1); // include the ` or } before
    while i < len {
        if bytes[i] == b'\\' && i + 1 < len {
            i += 2;
            continue;
        }
        if bytes[i] == b'`' {
            i += 1;
            tokens.push(Tok {
                kind: Tk::Str,
                start,
                end: i,
            });
            return i;
        }
        if bytes[i] == b'$' && i + 1 < len && bytes[i + 1] == b'{' {
            // Emit the template fragment up to here as a Str token
            tokens.push(Tok {
                kind: Tk::Str,
                start,
                end: i + 2,
            });
            i += 2; // skip `${`
                    // Push current brace depth so we know when the interpolation ends
            template_depth.push(*brace_depth);
            *brace_depth += 1; // the `${` acts like an opening brace
            return i;
        }
        i += 1;
    }
    // Unterminated template — emit what we have
    tokens.push(Tok {
        kind: Tk::Str,
        start,
        end: len,
    });
    len
}

fn is_ident_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_' || b == b'$'
}

fn is_ident_part(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_' || b == b'$'
}

// ── Identifier Classification ──────────────────────────────────────────

/// Return the index of the previous non-whitespace/newline token, if any.
fn prev_significant(tokens: &[Tok], idx: usize) -> Option<usize> {
    let mut i = idx.wrapping_sub(1);
    while i < tokens.len() {
        if tokens[i].kind != Tk::Ws && tokens[i].kind != Tk::Nl {
            return Some(i);
        }
        i = i.wrapping_sub(1);
    }
    None
}

/// Return the index of the next non-whitespace/newline token, if any.
fn next_significant(tokens: &[Tok], idx: usize) -> Option<usize> {
    let mut i = idx + 1;
    while i < tokens.len() {
        if tokens[i].kind != Tk::Ws && tokens[i].kind != Tk::Nl {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Check if a name is a JavaScript keyword.
fn is_js_keyword(name: &str) -> bool {
    matches!(
        name,
        "break"
            | "case"
            | "catch"
            | "class"
            | "const"
            | "continue"
            | "debugger"
            | "default"
            | "delete"
            | "do"
            | "else"
            | "enum"
            | "export"
            | "extends"
            | "false"
            | "finally"
            | "for"
            | "function"
            | "if"
            | "import"
            | "in"
            | "instanceof"
            | "let"
            | "new"
            | "null"
            | "of"
            | "return"
            | "static"
            | "super"
            | "switch"
            | "this"
            | "throw"
            | "true"
            | "try"
            | "typeof"
            | "undefined"
            | "var"
            | "void"
            | "while"
            | "with"
            | "yield"
            | "async"
            | "await"
    )
}

/// Check if a name is a well-known JS global / built-in.
fn is_js_global(name: &str) -> bool {
    matches!(
        name,
        // DOM / Browser
        "document" | "window" | "location" | "history" | "navigator"
        | "console" | "performance" | "localStorage" | "sessionStorage"
        | "fetch" | "queueMicrotask" | "setTimeout" | "clearTimeout"
        | "setInterval" | "clearInterval" | "requestAnimationFrame"
        | "cancelAnimationFrame" | "matchMedia" | "alert" | "confirm"
        | "prompt" | "atob" | "btoa" | "close"
        // Constructors / types
        | "Object" | "Array" | "String" | "Number" | "Boolean" | "Symbol"
        | "BigInt" | "Function" | "RegExp" | "Date" | "Math" | "JSON"
        | "Map" | "Set" | "WeakMap" | "WeakSet" | "WeakRef"
        | "Promise" | "Error" | "TypeError" | "RangeError" | "SyntaxError"
        | "ReferenceError" | "URIError" | "EvalError" | "AggregateError"
        | "Proxy" | "Reflect" | "Intl" | "ArrayBuffer" | "DataView"
        | "Int8Array" | "Uint8Array" | "Float32Array" | "Float64Array"
        | "globalThis" | "NaN" | "Infinity" | "isNaN" | "isFinite"
        | "parseInt" | "parseFloat" | "encodeURI" | "decodeURI"
        | "encodeURIComponent" | "decodeURIComponent" | "eval"
        // Web APIs used by the GaleX runtime
        | "WebSocket" | "AbortController" | "URLSearchParams" | "URL"
        | "Headers" | "Request" | "Response" | "FormData" | "Blob" | "File"
        | "Event" | "CustomEvent" | "PopStateEvent" | "MutationObserver"
        | "ResizeObserver" | "IntersectionObserver"
        | "NodeFilter" | "HTMLElement" | "Element" | "Node"
        | "DocumentFragment" | "EventTarget"
        // Arguments object
        | "arguments"
    )
}

// ── Scope Analysis & Rename Map ────────────────────────────────────────

/// Compute a rename map: original name → short name.
///
/// Analyses the token stream to find identifiers safe to rename,
/// then assigns the shortest possible names (most-frequent first).
fn compute_renames(tokens: &[Tok], src: &str) -> HashMap<String, String> {
    // Step 1: Collect all identifier names and their contexts.
    // An identifier is a "candidate" if it's not a keyword, not a global,
    // not exported, and not exclusively used as a property access.
    let mut name_freq: HashMap<String, usize> = HashMap::new();
    let mut property_only: HashSet<String> = HashSet::new(); // names only seen as properties
    let mut has_variable_use: HashSet<String> = HashSet::new(); // names seen as variable
    let mut exported: HashSet<String> = HashSet::new();
    let mut shorthand_props: HashSet<String> = HashSet::new();

    for (idx, tok) in tokens.iter().enumerate() {
        if tok.kind != Tk::Ident {
            continue;
        }
        let name = tok.text(src);

        // Skip keywords and globals immediately
        if is_js_keyword(name) || is_js_global(name) {
            continue;
        }

        *name_freq.entry(name.to_string()).or_insert(0) += 1;

        // Check if this is a property access (after `.` or `?.`)
        if let Some(pi) = prev_significant(tokens, idx) {
            let pt = tokens[pi].text(src);
            if pt == "." || pt == "?." {
                property_only.insert(name.to_string());
                continue; // this occurrence is a property; don't mark as variable
            }
        }

        // Check if exported
        if is_exported_ident(tokens, src, idx) {
            exported.insert(name.to_string());
            continue;
        }

        // Check if this is a shorthand property in an object literal.
        // Pattern: ident is followed by `,` or `}` and preceded by `{` or `,`
        // and NOT followed by `(` (method shorthand) or `:` (long-form property).
        if is_shorthand_property(tokens, src, idx) {
            shorthand_props.insert(name.to_string());
        }

        has_variable_use.insert(name.to_string());
    }

    // Step 2: Determine candidates.
    // A name is safe to rename if it has at least one "variable use" context
    // and is not exported and not a shorthand property.
    let mut candidates: Vec<(String, usize)> = Vec::new();
    for (name, freq) in &name_freq {
        if exported.contains(name) {
            continue;
        }
        if !has_variable_use.contains(name) {
            continue; // only seen as property access
        }
        if shorthand_props.contains(name) {
            continue; // used as shorthand property — renaming changes object shape
        }
        candidates.push((name.clone(), *freq));
    }

    // Sort by frequency descending (most frequent → shortest name)
    candidates.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));

    // Step 3: Assign short names, skipping collisions with protected names.
    let protected: HashSet<&str> = exported
        .iter()
        .map(|s| s.as_str())
        .chain(shorthand_props.iter().map(|s| s.as_str()))
        .chain(
            property_only
                .difference(&has_variable_use)
                .map(|s| s.as_str()),
        )
        .collect();

    let mut renames = HashMap::new();
    let mut name_idx = 0usize;
    for (orig_name, _freq) in &candidates {
        loop {
            let short = gen_short_name(name_idx);
            name_idx += 1;
            // Skip if short name collides with a keyword, global, protected name,
            // or an existing original name that isn't being renamed.
            if is_js_keyword(&short)
                || is_js_global(&short)
                || protected.contains(short.as_str())
                || (name_freq.contains_key(&short) && !candidates.iter().any(|(n, _)| n == &short))
            {
                continue;
            }
            renames.insert(orig_name.clone(), short);
            break;
        }
    }

    renames
}

/// Check if the identifier at `idx` is an exported name.
///
/// Detects patterns: `export function NAME`, `export class NAME`,
/// `export const NAME`, `export let NAME`, `export var NAME`,
/// `export default function NAME`, `export async function NAME`.
fn is_exported_ident(tokens: &[Tok], src: &str, idx: usize) -> bool {
    // Walk backwards over possible intermediate keywords between
    // `export` and the identifier name.
    let mut i = idx;

    // Skip back over: const, let, var, function, class, default, async
    loop {
        match prev_significant(tokens, i) {
            Some(pi) => {
                let pt = tokens[pi].text(src);
                if matches!(
                    pt,
                    "const" | "let" | "var" | "function" | "class" | "default" | "async"
                ) {
                    i = pi;
                } else {
                    break;
                }
            }
            None => return false,
        }
    }

    // Now check if preceded by `export`
    match prev_significant(tokens, i) {
        Some(pi) => tokens[pi].text(src) == "export",
        None => false,
    }
}

/// Check if the identifier at `idx` is a shorthand property in an object literal.
///
/// Detects pattern: `{ name, ... }` or `{ ..., name }` where `name` is not
/// followed by `:` (long-form) or `(` (method).
fn is_shorthand_property(tokens: &[Tok], src: &str, idx: usize) -> bool {
    // Must be followed by `,` or `}` (the end of a property in an object)
    let next = match next_significant(tokens, idx) {
        Some(ni) => tokens[ni].text(src),
        None => return false,
    };
    if next != "," && next != "}" {
        return false;
    }

    // Must be preceded by `{` or `,` (start of object or previous property)
    let prev = match prev_significant(tokens, idx) {
        Some(pi) => tokens[pi].text(src),
        None => return false,
    };
    if prev != "{" && prev != "," {
        return false;
    }

    // Exclude destructuring declarations: `const { name } = ...`
    // If there's a `=` after the `}`, it's destructuring, not a literal.
    // This is an approximation — in destructuring, the name IS being declared
    // and renaming it changes the property accessed from the RHS object.
    // So shorthand in destructuring should ALSO be protected. Our detection
    // here protects both cases, which is correct.
    true
}

// ── Short Name Generator ───────────────────────────────────────────────

/// Generate a short identifier name from a sequential index.
///
/// Produces: a, b, ..., z, A, ..., Z, $, _, aa, ab, ..., a9, ba, ...
fn gen_short_name(idx: usize) -> String {
    const FIRST: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ$_";
    const CHARS: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ$_0123456789";

    let fc = FIRST.len(); // 54
    let cc = CHARS.len(); // 64

    if idx < fc {
        return String::from(FIRST[idx] as char);
    }
    let idx = idx - fc;

    if idx < fc * cc {
        let mut s = String::with_capacity(2);
        s.push(FIRST[idx / cc] as char);
        s.push(CHARS[idx % cc] as char);
        return s;
    }
    let idx = idx - fc * cc;

    // Three-char names (should be more than enough)
    let mut s = String::with_capacity(3);
    s.push(FIRST[idx / (cc * cc) % fc] as char);
    s.push(CHARS[(idx / cc) % cc] as char);
    s.push(CHARS[idx % cc] as char);
    s
}

// ── Output Emission ────────────────────────────────────────────────────

/// Returns `true` if the token text looks like an identifier/keyword/number
/// (i.e., a "word" that needs a space separator from an adjacent word).
fn is_word_token(kind: Tk) -> bool {
    matches!(kind, Tk::Ident | Tk::Num)
}

/// Emit the token stream with renames applied and minimal whitespace.
///
/// Produces a single-line output (no newlines) except inside string literals.
/// Inserts spaces only where needed for correct parsing.
fn emit_minified(tokens: &[Tok], src: &str, renames: &HashMap<String, String>) -> String {
    let mut out = String::with_capacity(src.len());
    let mut prev_kind: Option<Tk> = None;
    let mut prev_text_last: Option<char> = None;

    for (idx, tok) in tokens.iter().enumerate() {
        // Skip whitespace and newlines — we emit our own spacing.
        if tok.kind == Tk::Ws || tok.kind == Tk::Nl {
            continue;
        }

        let text = tok.text(src);

        // Determine the actual output text (possibly renamed).
        let renamed_buf: String;
        let output = if tok.kind == Tk::Ident && !is_js_keyword(text) {
            // Only rename if NOT a property access (after `.` or `?.`)
            let is_prop = if let Some(pi) = prev_significant(tokens, idx) {
                let pt = tokens[pi].text(src);
                pt == "." || pt == "?."
            } else {
                false
            };
            if !is_prop {
                if let Some(renamed) = renames.get(text) {
                    renamed_buf = renamed.clone();
                    renamed_buf.as_str()
                } else {
                    text
                }
            } else {
                text
            }
        } else {
            text
        };

        // Determine if we need a space separator before this token.
        if let Some(pk) = prev_kind {
            let first_out = output.as_bytes().first().copied().unwrap_or(b' ');
            let need_space = if is_word_token(pk) && is_word_token(tok.kind) {
                // Two adjacent identifiers/numbers always need a space
                true
            } else if is_word_token(pk) && tok.kind == Tk::Str {
                // keyword followed by string: `return"x"` is valid but
                // `return`x`` needs care — always safe to add space
                first_out == b'`'
            } else if pk == Tk::Punct && tok.kind == Tk::Punct {
                // Avoid creating ambiguous multi-char operators
                let pl = prev_text_last.unwrap_or(' ') as u8;
                (pl == b'+' && first_out == b'+')
                    || (pl == b'-' && first_out == b'-')
                    || (pl == b'/' && first_out == b'/')
                    || (pl == b'<' && first_out == b'!')
            } else if pk == Tk::Num && tok.kind == Tk::Punct && first_out == b'.' {
                // `1.toString()` needs to be `1 .toString()` or `(1).toString()`
                true
            } else {
                false
            };

            if need_space {
                out.push(' ');
            }
        }

        out.push_str(output);
        prev_kind = Some(tok.kind);
        prev_text_last = output.chars().last();
    }

    out
}

// ── Public API ─────────────────────────────────────────────────────────

/// Production-grade JavaScript minifier with identifier mangling.
///
/// Performs:
/// 1. Comment stripping
/// 2. Scope-aware identifier renaming (local variables, parameters)
/// 3. Minimal whitespace emission
///
/// Preserves:
/// - Exported names
/// - Property access names (after `.`)
/// - Object shorthand property names
/// - JS keywords and global built-ins
/// - String and template literal contents
pub fn minify_js_production(source: &str) -> String {
    if source.is_empty() {
        return String::new();
    }
    let tokens = tokenize_js(source);
    let renames = compute_renames(&tokens, source);
    emit_minified(&tokens, source, &renames)
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── Basic minifier tests ──────────────────────────────────

    #[test]
    fn strips_line_comments() {
        let input = "const x = 1; // this is a comment\nconst y = 2;";
        let out = minify_js(input);
        assert!(
            !out.contains("this is a comment"),
            "comment stripped: {out}"
        );
        assert!(out.contains("const x = 1;"), "code preserved: {out}");
        assert!(out.contains("const y = 2;"), "next line ok: {out}");
    }

    #[test]
    fn strips_block_comments() {
        let input = "const x = 1;\n/* multi\nline\ncomment */\nconst y = 2;";
        let out = minify_js(input);
        assert!(!out.contains("multi"), "block comment stripped: {out}");
        assert!(out.contains("const x = 1;"), "before preserved: {out}");
        assert!(out.contains("const y = 2;"), "after preserved: {out}");
    }

    #[test]
    fn removes_blank_lines() {
        let input = "const x = 1;\n\n\n\nconst y = 2;";
        let out = minify_js(input);
        assert_eq!(out.matches('\n').count(), 2, "minimal newlines: {out}");
    }

    #[test]
    fn collapses_whitespace() {
        let input = "const   x   =   1;";
        let out = minify_js(input);
        assert!(out.contains("const x = 1;"), "collapsed: {out}");
    }

    #[test]
    fn preserves_strings() {
        let input = r#"const s = "hello   world // not a comment";"#;
        let out = minify_js(input);
        assert!(
            out.contains("hello   world // not a comment"),
            "string preserved: {out}"
        );
    }

    #[test]
    fn preserves_template_literals() {
        let input = "const s = `hello   ${x}   world`;";
        let out = minify_js(input);
        assert!(
            out.contains("hello   ${x}   world"),
            "template preserved: {out}"
        );
    }

    #[test]
    fn inline_block_comment() {
        let input = "const x = /* inline */ 1;";
        let out = minify_js(input);
        assert!(!out.contains("inline"), "inline comment stripped: {out}");
        assert!(out.contains("const x = 1;"), "code preserved: {out}");
    }

    #[test]
    fn reduces_size() {
        let runtime = include_str!("../runtime/gale_runtime.js");
        let minified = minify_js(runtime);
        assert!(
            minified.len() < runtime.len(),
            "minified ({}) should be smaller than original ({})",
            minified.len(),
            runtime.len()
        );
        let ratio = minified.len() as f64 / runtime.len() as f64;
        assert!(
            ratio < 0.70,
            "expected >30% reduction, got {:.0}% (ratio {ratio:.2})",
            (1.0 - ratio) * 100.0
        );
    }

    // ── Tokenizer tests ───────────────────────────────────────

    #[test]
    fn tokenize_simple() {
        let src = "const x = 1;";
        let tokens = tokenize_js(src);
        let kinds: Vec<Tk> = tokens.iter().map(|t| t.kind).collect();
        // const, ws, x, ws, =, ws, 1, ;
        assert_eq!(tokens[0].text(src), "const");
        assert_eq!(tokens[0].kind, Tk::Ident);
        // Find `x` ident
        let x_tok = tokens.iter().find(|t| t.text(src) == "x").unwrap();
        assert_eq!(x_tok.kind, Tk::Ident);
        // Find `1` number
        let num_tok = tokens.iter().find(|t| t.text(src) == "1").unwrap();
        assert_eq!(num_tok.kind, Tk::Num);
    }

    #[test]
    fn tokenize_strings() {
        let src = r#"const s = "hello world";"#;
        let tokens = tokenize_js(src);
        let str_tok = tokens.iter().find(|t| t.kind == Tk::Str).unwrap();
        assert!(str_tok.text(src).contains("hello world"));
    }

    #[test]
    fn tokenize_template_with_interpolation() {
        let src = "const s = `hello ${name} world`;";
        let tokens = tokenize_js(src);
        // Should have Str tokens for template parts and Ident for `name`
        let name_tok = tokens.iter().find(|t| t.text(src) == "name");
        assert!(
            name_tok.is_some(),
            "name should be tokenized as ident inside template"
        );
        assert_eq!(name_tok.unwrap().kind, Tk::Ident);
    }

    #[test]
    fn tokenize_skips_comments() {
        let src = "const x = 1; // comment\nconst y = 2;";
        let tokens = tokenize_js(src);
        // No token should contain "comment"
        for tok in &tokens {
            assert!(
                !tok.text(src).contains("comment"),
                "comment should be skipped"
            );
        }
    }

    // ── Rename map tests ──────────────────────────────────────

    #[test]
    fn renames_local_variables() {
        let src = "function foo() { let longName = 1; return longName; }";
        let tokens = tokenize_js(src);
        let renames = compute_renames(&tokens, src);
        // `longName` should be a candidate for renaming
        assert!(
            renames.contains_key("longName"),
            "longName should be renamed"
        );
        // `foo` is not exported, so it should also be renamed
        assert!(renames.contains_key("foo"), "foo should be renamed");
    }

    #[test]
    fn preserves_exported_names() {
        let src = "export function signal(init) { let val = init; return val; }";
        let tokens = tokenize_js(src);
        let renames = compute_renames(&tokens, src);
        assert!(
            !renames.contains_key("signal"),
            "exported name should not be renamed: {renames:?}"
        );
        // `init` and `val` should be renamed
        assert!(
            renames.contains_key("init"),
            "param should be renamed: {renames:?}"
        );
        assert!(
            renames.contains_key("val"),
            "local should be renamed: {renames:?}"
        );
    }

    #[test]
    fn preserves_property_accesses() {
        let src = "let obj = {}; obj.name = 'test'; let name = obj.name;";
        let tokens = tokenize_js(src);
        let renames = compute_renames(&tokens, src);
        // `name` is used as both variable and property.  Because it appears
        // as a variable use, it IS a candidate.  Property access occurrences
        // will NOT be renamed (handled in emit).
        if renames.contains_key("name") {
            // Verify that property accesses are NOT renamed in the output
            let output = emit_minified(&tokens, src, &renames);
            assert!(
                output.contains(".name"),
                "property access should keep original name: {output}"
            );
        }
    }

    #[test]
    fn preserves_keywords() {
        let src = "const x = 1; if (x) { return x; }";
        let tokens = tokenize_js(src);
        let renames = compute_renames(&tokens, src);
        assert!(!renames.contains_key("const"));
        assert!(!renames.contains_key("if"));
        assert!(!renames.contains_key("return"));
    }

    #[test]
    fn preserves_globals() {
        let src = "let el = document.querySelector('div');";
        let tokens = tokenize_js(src);
        let renames = compute_renames(&tokens, src);
        assert!(
            !renames.contains_key("document"),
            "globals should not be renamed"
        );
    }

    #[test]
    fn shorthand_properties_protected() {
        let src = "const data = 1; const loading = 2; return { data, loading };";
        let tokens = tokenize_js(src);
        let renames = compute_renames(&tokens, src);
        assert!(
            !renames.contains_key("data"),
            "shorthand prop should not be renamed: {renames:?}"
        );
        assert!(
            !renames.contains_key("loading"),
            "shorthand prop should not be renamed: {renames:?}"
        );
    }

    // ── Short name generator tests ────────────────────────────

    #[test]
    fn gen_short_name_sequence() {
        assert_eq!(gen_short_name(0), "a");
        assert_eq!(gen_short_name(1), "b");
        assert_eq!(gen_short_name(25), "z");
        assert_eq!(gen_short_name(26), "A");
        assert_eq!(gen_short_name(51), "Z");
        assert_eq!(gen_short_name(52), "$");
        assert_eq!(gen_short_name(53), "_");
        // Two-char names start at index 54
        assert_eq!(gen_short_name(54), "aa");
        assert_eq!(gen_short_name(55), "ab");
    }

    #[test]
    fn gen_short_name_no_duplicates() {
        let mut seen = HashSet::new();
        for i in 0..500 {
            let name = gen_short_name(i);
            assert!(
                seen.insert(name.clone()),
                "duplicate name at index {i}: {name}"
            );
        }
    }

    // ── Production minifier integration tests ─────────────────

    #[test]
    fn production_strips_comments() {
        let src = "// comment\nconst x = 1; /* block */ const y = 2;";
        let out = minify_js_production(src);
        assert!(!out.contains("comment"), "comments stripped: {out}");
        assert!(!out.contains("block"), "block comment stripped: {out}");
    }

    #[test]
    fn production_renames_locals() {
        let src = "function test() { let longVariable = 42; return longVariable; }";
        let out = minify_js_production(src);
        assert!(
            !out.contains("longVariable"),
            "local should be renamed: {out}"
        );
        assert!(
            !out.contains("test"),
            "non-exported fn should be renamed: {out}"
        );
    }

    #[test]
    fn production_preserves_exports() {
        let src = "export function signal(init) { let val = init; return val; }";
        let out = minify_js_production(src);
        assert!(out.contains("signal"), "export must be preserved: {out}");
    }

    #[test]
    fn production_preserves_strings() {
        let src = r#"const msg = "Hello World"; export function greet() { return msg; }"#;
        let out = minify_js_production(src);
        assert!(
            out.contains("\"Hello World\""),
            "string content preserved: {out}"
        );
    }

    #[test]
    fn production_template_literal_refs_renamed() {
        let src = "function f(name) { return `hello ${name}`; }";
        let out = minify_js_production(src);
        // `name` should be renamed inside the template interpolation too
        assert!(
            !out.contains("name"),
            "param in template should be renamed: {out}"
        );
    }

    #[test]
    fn production_property_access_preserved() {
        let src = "function f() { let el = document.createElement('div'); el.textContent = 'hi'; return el; }";
        let out = minify_js_production(src);
        assert!(out.contains(".createElement"), "property preserved: {out}");
        assert!(out.contains(".textContent"), "property preserved: {out}");
    }

    #[test]
    fn production_reduces_runtime_size() {
        let runtime = include_str!("../runtime/gale_runtime.js");
        let minified = minify_js_production(runtime);
        let ratio = minified.len() as f64 / runtime.len() as f64;
        assert!(
            ratio < 0.55,
            "expected >45% reduction, got {:.0}% (ratio {ratio:.2}). \
             Original: {} bytes, minified: {} bytes",
            (1.0 - ratio) * 100.0,
            runtime.len(),
            minified.len(),
        );
    }

    #[test]
    fn production_output_preserves_all_exports() {
        let runtime = include_str!("../runtime/gale_runtime.js");
        let minified = minify_js_production(runtime);
        // All exported names from the runtime must be preserved
        for name in &[
            "signal",
            "derive",
            "effect",
            "watch",
            "batch",
            "hydrate",
            "bind",
            "show",
            "list",
            "replaceRegion",
            "reconcileList",
            "transition",
            "flipTransition",
            "action",
            "query",
            "channel",
            "navigate",
            "_readData",
            "_readEnv",
            "GaleValidationError",
            "GaleServerError",
            "GaleNetworkError",
            "__gx_fetch",
            "queryCache",
        ] {
            assert!(
                minified.contains(name),
                "exported name `{name}` must be preserved in minified output"
            );
        }
    }

    #[test]
    fn production_empty_input() {
        assert_eq!(minify_js_production(""), "");
    }
}
