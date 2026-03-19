//! Token-level JavaScript minifier for generated code.
//!
//! Strips comments, collapses whitespace, folds boolean/undefined constants,
//! and mangles local variable names.  Not a full-featured minifier — designed
//! for the predictable output of the GaleX JS codegen.

use std::collections::{HashMap, HashSet};

// ── Public API ─────────────────────────────────────────────────────────

/// Minify a JavaScript source string.
pub fn minify_js(src: &str) -> String {
    let mut tokens = tokenize(src);
    fold_constants(&mut tokens);
    mangle_identifiers(&mut tokens);
    emit(&tokens)
}

// ── Token types ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum Ty {
    Ws,
    Comment,
    Str,
    Regex,
    Ident,
    Num,
    Op,
    Tmpl,
}

#[derive(Debug, Clone)]
struct Token {
    ty: Ty,
    val: String,
}

// ── Reserved words that must never be mangled ──────────────────────────

fn is_reserved(s: &str) -> bool {
    matches!(
        s,
        "abstract"
            | "arguments"
            | "await"
            | "boolean"
            | "break"
            | "byte"
            | "case"
            | "catch"
            | "char"
            | "class"
            | "const"
            | "continue"
            | "debugger"
            | "default"
            | "delete"
            | "do"
            | "double"
            | "else"
            | "enum"
            | "eval"
            | "export"
            | "extends"
            | "false"
            | "final"
            | "finally"
            | "float"
            | "for"
            | "function"
            | "goto"
            | "if"
            | "implements"
            | "import"
            | "in"
            | "instanceof"
            | "int"
            | "interface"
            | "let"
            | "long"
            | "native"
            | "new"
            | "null"
            | "of"
            | "package"
            | "private"
            | "protected"
            | "public"
            | "return"
            | "short"
            | "static"
            | "super"
            | "switch"
            | "synchronized"
            | "this"
            | "throw"
            | "throws"
            | "transient"
            | "true"
            | "try"
            | "typeof"
            | "undefined"
            | "var"
            | "void"
            | "volatile"
            | "while"
            | "with"
            | "yield"
            | "console"
            | "window"
            | "document"
            | "global"
            | "process"
            | "require"
            | "module"
            | "exports"
            | "Promise"
            | "Array"
            | "Object"
            | "String"
            | "Number"
            | "Boolean"
            | "Symbol"
            | "Map"
            | "Set"
            | "Date"
            | "RegExp"
            | "Error"
            | "JSON"
            | "Math"
            | "parseInt"
            | "parseFloat"
            | "isNaN"
            | "isFinite"
            | "setTimeout"
            | "setInterval"
            | "clearTimeout"
            | "clearInterval"
            | "fetch"
            | "Response"
            | "Request"
            | "URL"
            | "Proxy"
            | "Reflect"
            | "globalThis"
            | "NaN"
            | "Infinity"
            | "alert"
            | "from"
            | "as"
            | "async"
            | "get"
            | "set"
    )
}

// ── Base-52 short names ────────────────────────────────────────────────

const B52: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";

fn to_base52(mut n: usize) -> String {
    let mut s = Vec::new();
    loop {
        s.push(B52[n % 52]);
        n /= 52;
        if n == 0 {
            break;
        }
    }
    s.reverse();
    String::from_utf8(s).unwrap()
}

// ── Tokenizer ──────────────────────────────────────────────────────────

fn tokenize(src: &str) -> Vec<Token> {
    let bytes = src.as_bytes();
    let len = bytes.len();
    let mut tokens = Vec::new();
    let mut i = 0;

    while i < len {
        let c = bytes[i];

        // Whitespace
        if c == b' ' || c == b'\t' || c == b'\n' || c == b'\r' {
            while i < len
                && (bytes[i] == b' ' || bytes[i] == b'\t' || bytes[i] == b'\n' || bytes[i] == b'\r')
            {
                i += 1;
            }
            tokens.push(Token {
                ty: Ty::Ws,
                val: " ".into(),
            });
            continue;
        }

        // Single-line comment
        if c == b'/' && i + 1 < len && bytes[i + 1] == b'/' {
            i += 2;
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
            tokens.push(Token {
                ty: Ty::Comment,
                val: String::new(),
            });
            continue;
        }

        // Multi-line comment
        if c == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
            i += 2;
            while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i += 2;
            tokens.push(Token {
                ty: Ty::Comment,
                val: String::new(),
            });
            continue;
        }

        // Template literal
        if c == b'`' {
            let start = i;
            i += 1;
            let mut depth = 0u32;
            while i < len {
                if bytes[i] == b'\\' && i + 1 < len {
                    i += 2;
                    continue;
                }
                if bytes[i] == b'$' && i + 1 < len && bytes[i + 1] == b'{' {
                    depth += 1;
                    i += 2;
                    continue;
                }
                if bytes[i] == b'}' && depth > 0 {
                    depth -= 1;
                    i += 1;
                    continue;
                }
                if bytes[i] == b'`' && depth == 0 {
                    i += 1;
                    break;
                }
                i += 1;
            }
            tokens.push(Token {
                ty: Ty::Tmpl,
                val: src[start..i].into(),
            });
            continue;
        }

        // String literals
        if c == b'"' || c == b'\'' {
            let q = c;
            let start = i;
            i += 1;
            while i < len {
                if bytes[i] == b'\\' {
                    i += 2;
                    continue;
                }
                if bytes[i] == q {
                    i += 1;
                    break;
                }
                i += 1;
            }
            tokens.push(Token {
                ty: Ty::Str,
                val: src[start..i].into(),
            });
            continue;
        }

        // Regex literal (heuristic: after operator or keyword)
        if c == b'/' {
            let prev = last_non_ws(&tokens);
            let is_regex = match prev {
                None => true,
                Some(t) => {
                    t.ty == Ty::Op
                        || (t.ty == Ty::Ident
                            && matches!(
                                t.val.as_str(),
                                "return"
                                    | "typeof"
                                    | "void"
                                    | "delete"
                                    | "throw"
                                    | "in"
                                    | "instanceof"
                                    | "case"
                                    | "new"
                            ))
                }
            };
            if is_regex {
                let start = i;
                i += 1;
                while i < len {
                    if bytes[i] == b'\\' {
                        i += 2;
                        continue;
                    }
                    if bytes[i] == b'/' {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                while i < len && bytes[i].is_ascii_alphabetic() {
                    i += 1;
                }
                tokens.push(Token {
                    ty: Ty::Regex,
                    val: src[start..i].into(),
                });
                continue;
            }
        }

        // Identifier / keyword
        if c.is_ascii_alphabetic() || c == b'_' || c == b'$' {
            let start = i;
            i += 1;
            while i < len
                && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'$')
            {
                i += 1;
            }
            tokens.push(Token {
                ty: Ty::Ident,
                val: src[start..i].into(),
            });
            continue;
        }

        // Number
        if c.is_ascii_digit() || (c == b'.' && i + 1 < len && bytes[i + 1].is_ascii_digit()) {
            let start = i;
            if bytes[i] == b'0'
                && i + 1 < len
                && (bytes[i + 1] == b'x'
                    || bytes[i + 1] == b'X'
                    || bytes[i + 1] == b'b'
                    || bytes[i + 1] == b'B'
                    || bytes[i + 1] == b'o'
                    || bytes[i + 1] == b'O')
            {
                i += 2;
                while i < len && (bytes[i].is_ascii_hexdigit() || bytes[i] == b'_') {
                    i += 1;
                }
            } else {
                while i < len && (bytes[i].is_ascii_digit() || bytes[i] == b'.' || bytes[i] == b'_')
                {
                    i += 1;
                }
                if i < len && (bytes[i] == b'e' || bytes[i] == b'E') {
                    i += 1;
                    if i < len && (bytes[i] == b'+' || bytes[i] == b'-') {
                        i += 1;
                    }
                    while i < len && bytes[i].is_ascii_digit() {
                        i += 1;
                    }
                }
            }
            if i < len && bytes[i] == b'n' {
                i += 1; // BigInt
            }
            tokens.push(Token {
                ty: Ty::Num,
                val: src[start..i].into(),
            });
            continue;
        }

        // Multi-char operators (3-char)
        if i + 2 < len {
            let tri = &src[i..i + 3];
            if matches!(
                tri,
                "===" | "!==" | ">>>" | "**=" | "&&=" | "||=" | "??=" | "<<=" | ">>="
            ) {
                tokens.push(Token {
                    ty: Ty::Op,
                    val: tri.into(),
                });
                i += 3;
                continue;
            }
        }

        // Multi-char operators (2-char)
        if i + 1 < len {
            let bi = &src[i..i + 2];
            if matches!(
                bi,
                "==" | "!="
                    | "<="
                    | ">="
                    | "++"
                    | "--"
                    | "&&"
                    | "||"
                    | "??"
                    | "=>"
                    | "+="
                    | "-="
                    | "*="
                    | "/="
                    | "%="
                    | "**"
                    | "<<"
                    | ">>"
                    | "?."
                    | "&="
                    | "|="
                    | "^="
            ) {
                tokens.push(Token {
                    ty: Ty::Op,
                    val: bi.into(),
                });
                i += 2;
                continue;
            }
        }

        // Single-char operator
        tokens.push(Token {
            ty: Ty::Op,
            val: src[i..i + 1].into(),
        });
        i += 1;
    }
    tokens
}

fn last_non_ws(tokens: &[Token]) -> Option<&Token> {
    tokens
        .iter()
        .rev()
        .find(|t| t.ty != Ty::Ws && t.ty != Ty::Comment)
}

// ── Constant folding ───────────────────────────────────────────────────

fn fold_constants(tokens: &mut [Token]) {
    for tok in tokens.iter_mut() {
        if tok.ty == Ty::Ident {
            match tok.val.as_str() {
                "true" => {
                    tok.ty = Ty::Op;
                    tok.val = "!0".into();
                }
                "false" => {
                    tok.ty = Ty::Op;
                    tok.val = "!1".into();
                }
                "undefined" => {
                    tok.ty = Ty::Op;
                    tok.val = "void 0".into();
                }
                _ => {}
            }
        }
    }
}

// ── Identifier mangling ────────────────────────────────────────────────

fn mangle_identifiers(tokens: &mut Vec<Token>) {
    // Collect local declarations
    let mut decls = HashSet::new();
    for i in 0..tokens.len() {
        if tokens[i].ty != Ty::Ident {
            continue;
        }
        if matches!(tokens[i].val.as_str(), "let" | "const" | "var") {
            // Next non-ws ident is a declaration
            for j in (i + 1)..tokens.len() {
                if tokens[j].ty == Ty::Ws || tokens[j].ty == Ty::Comment {
                    continue;
                }
                if tokens[j].ty == Ty::Ident && !is_reserved(&tokens[j].val) {
                    decls.insert(tokens[j].val.clone());
                }
                break;
            }
        }
        if tokens[i].val == "function" {
            let mut j = i + 1;
            while j < tokens.len() && tokens[j].ty == Ty::Ws {
                j += 1;
            }
            // Function name
            if j < tokens.len() && tokens[j].ty == Ty::Ident && !is_reserved(&tokens[j].val) {
                decls.insert(tokens[j].val.clone());
                j += 1;
            }
            // Params
            if j < tokens.len() && tokens[j].val == "(" {
                j += 1;
                while j < tokens.len() && tokens[j].val != ")" {
                    if tokens[j].ty == Ty::Ident && !is_reserved(&tokens[j].val) {
                        decls.insert(tokens[j].val.clone());
                    }
                    j += 1;
                }
            }
        }
    }

    // Build rename map
    let mut map = HashMap::new();
    let mut idx = 0usize;
    for name in &decls {
        loop {
            let short = to_base52(idx);
            idx += 1;
            if !is_reserved(&short) {
                map.insert(name.clone(), short);
                break;
            }
        }
    }

    // Rename — skip property access (after '.') and object keys (before ':')
    for i in 0..tokens.len() {
        if tokens[i].ty != Ty::Ident {
            continue;
        }
        let short = match map.get(&tokens[i].val) {
            Some(s) => s.clone(),
            None => continue,
        };
        // Check if preceded by '.'
        let prev = (0..i)
            .rev()
            .find(|&j| tokens[j].ty != Ty::Ws && tokens[j].ty != Ty::Comment)
            .map(|j| &tokens[j]);
        if prev.map(|t| t.val.as_str()) == Some(".") {
            continue;
        }
        // Check if followed by ':'  (object key)
        let next = ((i + 1)..tokens.len())
            .find(|&j| tokens[j].ty != Ty::Ws && tokens[j].ty != Ty::Comment)
            .map(|j| &tokens[j]);
        if next.map(|t| t.val.as_str()) == Some(":") {
            continue;
        }
        tokens[i].val = short;
    }
}

// ── Whitespace elimination ─────────────────────────────────────────────

fn needs_space(a: &Token, b: &Token) -> bool {
    let a_word = a.ty == Ty::Ident || a.ty == Ty::Num;
    let b_word = b.ty == Ty::Ident || b.ty == Ty::Num;
    if a_word && b_word {
        return true;
    }
    // Keywords that need space before certain tokens
    if a.ty == Ty::Ident
        && matches!(
            a.val.as_str(),
            "return"
                | "throw"
                | "typeof"
                | "void"
                | "delete"
                | "new"
                | "in"
                | "instanceof"
                | "case"
                | "of"
                | "yield"
                | "await"
                | "else"
                | "catch"
                | "finally"
                | "export"
                | "import"
                | "from"
                | "async"
        )
    {
        if b.ty == Ty::Ident
            || b.ty == Ty::Num
            || b.ty == Ty::Str
            || b.ty == Ty::Tmpl
            || b.ty == Ty::Regex
            || matches!(
                b.val.as_str(),
                "(" | "{" | "[" | "!" | "~" | "+" | "-" | "/"
            )
        {
            return true;
        }
    }
    // Avoid ++ -- ambiguity
    if (a.val == "+" && (b.val == "+" || b.val == "++"))
        || (a.val == "-" && (b.val == "-" || b.val == "--"))
    {
        return true;
    }
    false
}

fn emit(tokens: &[Token]) -> String {
    let mut out = String::with_capacity(tokens.iter().map(|t| t.val.len()).sum());
    let mut prev: Option<&Token> = None;
    for tok in tokens {
        if tok.ty == Ty::Ws || tok.ty == Ty::Comment {
            continue;
        }
        if let Some(p) = prev {
            if needs_space(p, tok) {
                out.push(' ');
            }
        }
        out.push_str(&tok.val);
        prev = Some(tok);
    }
    out
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_comments() {
        assert_eq!(minify_js("let x = 1; // comment\n"), "let a=1;");
    }

    #[test]
    fn collapses_whitespace() {
        assert_eq!(minify_js("let  x  =  1 ;"), "let a=1;");
    }

    #[test]
    fn folds_booleans() {
        let out = minify_js("let a = true; let b = false;");
        assert!(out.contains("!0"));
        assert!(out.contains("!1"));
    }

    #[test]
    fn preserves_strings() {
        let out = minify_js(r#"let x = "hello world";"#);
        assert!(out.contains("\"hello world\""));
    }

    #[test]
    fn preserves_template_literals() {
        let out = minify_js("let x = `hello ${name}`;");
        assert!(out.contains("`hello ${name}`"));
    }

    #[test]
    fn mangles_locals() {
        let out = minify_js("let counter = 0; counter++;");
        assert!(!out.contains("counter"));
    }

    #[test]
    fn does_not_mangle_reserved() {
        let out = minify_js("let x = document.getElementById('a');");
        assert!(out.contains("document"));
        assert!(out.contains("getElementById"));
    }
}
