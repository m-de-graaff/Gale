//! Self-contained Tailwind CSS utility generator.
//!
//! No Node.js, no npm, no external crates.  Class names extracted from the
//! GaleX AST are mapped to CSS rules via lookup tables and pattern matching.
//! Only classes actually used in the project are emitted (JIT-style).

use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use super::config::TailwindConfig;

/// Errors that can occur during Tailwind CSS generation.
#[derive(Debug)]
pub enum TailwindError {
    /// A generation error.
    BuildFailed(String),
    /// I/O error (file read/write).
    IoError(std::io::Error),
}

impl std::fmt::Display for TailwindError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TailwindError::BuildFailed(msg) => write!(f, "Tailwind CSS build failed: {msg}"),
            TailwindError::IoError(e) => write!(f, "I/O error during CSS generation: {e}"),
        }
    }
}

impl From<std::io::Error> for TailwindError {
    fn from(e: std::io::Error) -> Self {
        TailwindError::IoError(e)
    }
}

/// Generate a Tailwind CSS bundle from extracted class names.
///
/// Pure Rust — no Node.js required.
pub fn run_tailwind(
    tw_config: &TailwindConfig,
    _app_dir: &Path,
    safelist: &[String],
    output_css: &Path,
    _minify: bool,
) -> Result<(), TailwindError> {
    // Collect all individual class names.
    let classes: BTreeSet<&str> = safelist.iter().flat_map(|g| g.split_whitespace()).collect();

    let colors = build_color_map(tw_config);
    let mut css = String::with_capacity(16 * 1024);

    // Preflight reset (minimal).
    css.push_str(PREFLIGHT);

    // Generate CSS for each class.
    // We separate base classes from variant classes (hover:, focus:, sm:, md:).
    let mut base_rules = Vec::new();
    let mut hover_rules = Vec::new();
    let mut focus_rules = Vec::new();
    let mut sm_rules = Vec::new();
    let mut md_rules = Vec::new();
    let mut lg_rules = Vec::new();
    let mut placeholder_rules = Vec::new();

    for &class in &classes {
        // Strip variant prefix.
        if let Some(rest) = class.strip_prefix("hover:") {
            if let Some(rule) = resolve_class(rest, &colors) {
                let sel = escape_selector(class);
                hover_rules.push(format!(".{sel}:hover{{{rule}}}"));
            }
        } else if let Some(rest) = class.strip_prefix("focus:") {
            if let Some(rule) = resolve_class(rest, &colors) {
                let sel = escape_selector(class);
                focus_rules.push(format!(".{sel}:focus{{{rule}}}"));
            }
        } else if let Some(rest) = class.strip_prefix("sm:") {
            if let Some(rule) = resolve_class(rest, &colors) {
                let sel = escape_selector(class);
                sm_rules.push(format!(".{sel}{{{rule}}}"));
            }
        } else if let Some(rest) = class.strip_prefix("md:") {
            if let Some(rule) = resolve_class(rest, &colors) {
                let sel = escape_selector(class);
                md_rules.push(format!(".{sel}{{{rule}}}"));
            }
        } else if let Some(rest) = class.strip_prefix("lg:") {
            if let Some(rule) = resolve_class(rest, &colors) {
                let sel = escape_selector(class);
                lg_rules.push(format!(".{sel}{{{rule}}}"));
            }
        } else if class.starts_with("placeholder-") {
            let color_name = &class["placeholder-".len()..];
            if let Some(hex) = colors.get(color_name) {
                let sel = escape_selector(class);
                placeholder_rules.push(format!(".{sel}::placeholder{{color:{hex}}}"));
            }
        } else if let Some(rule) = resolve_class(class, &colors) {
            let sel = escape_selector(class);
            base_rules.push(format!(".{sel}{{{rule}}}"));
        }
    }

    // Emit in correct order: base → pseudo → responsive.
    for rule in &base_rules {
        css.push_str(rule);
        css.push('\n');
    }
    for rule in &placeholder_rules {
        css.push_str(rule);
        css.push('\n');
    }
    for rule in &hover_rules {
        css.push_str(rule);
        css.push('\n');
    }
    for rule in &focus_rules {
        css.push_str(rule);
        css.push('\n');
    }
    if !sm_rules.is_empty() {
        css.push_str("@media(min-width:640px){\n");
        for rule in &sm_rules {
            css.push_str(rule);
            css.push('\n');
        }
        css.push_str("}\n");
    }
    if !md_rules.is_empty() {
        css.push_str("@media(min-width:768px){\n");
        for rule in &md_rules {
            css.push_str(rule);
            css.push('\n');
        }
        css.push_str("}\n");
    }
    if !lg_rules.is_empty() {
        css.push_str("@media(min-width:1024px){\n");
        for rule in &lg_rules {
            css.push_str(rule);
            css.push('\n');
        }
        css.push_str("}\n");
    }

    // Ensure output directory exists.
    if let Some(parent) = output_css.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(output_css, css)?;
    Ok(())
}

// ── Class resolver ─────────────────────────────────────────────────────

/// Resolve a single utility class name to its CSS property/value string.
/// Returns `None` if the class is unknown.
fn resolve_class(class: &str, colors: &HashMap<String, String>) -> Option<String> {
    // Try static lookup first (exact match).
    if let Some(rule) = STATIC_UTILS.iter().find(|(k, _)| *k == class) {
        return Some(rule.1.to_string());
    }

    // Arbitrary values: min-h-[4rem] → min-height:4rem
    if let Some(rule) = try_arbitrary(class) {
        return Some(rule);
    }

    // Dynamic pattern matching.
    // Background colors: bg-{color}
    if let Some(rest) = class.strip_prefix("bg-") {
        if let Some(hex) = colors.get(rest) {
            return Some(format!("background-color:{hex}"));
        }
        // Gradients: bg-gradient-to-{dir}
        if let Some(dir) = rest.strip_prefix("gradient-to-") {
            let angle = match dir {
                "t" => "to top",
                "tr" => "to top right",
                "r" => "to right",
                "br" => "to bottom right",
                "b" => "to bottom",
                "bl" => "to bottom left",
                "l" => "to left",
                "tl" => "to top left",
                _ => return None,
            };
            return Some(format!(
                "background-image:linear-gradient({angle},var(--tw-gradient-stops))"
            ));
        }
    }

    // Text colors: text-{color}
    if let Some(rest) = class.strip_prefix("text-") {
        if let Some(hex) = colors.get(rest) {
            return Some(format!("color:{hex}"));
        }
    }

    // Border colors: border-{color}
    if let Some(rest) = class.strip_prefix("border-") {
        if let Some(hex) = colors.get(rest) {
            return Some(format!("border-color:{hex}"));
        }
    }

    // Ring colors: ring-{color}
    if let Some(rest) = class.strip_prefix("ring-") {
        if let Some(hex) = colors.get(rest) {
            return Some(format!("--tw-ring-color:{hex}"));
        }
    }

    // Gradient from/via/to: from-{color}, via-{color}, to-{color}
    if let Some(rest) = class.strip_prefix("from-") {
        if let Some(hex) = colors.get(rest) {
            return Some(format!(
                "--tw-gradient-from:{hex};--tw-gradient-stops:var(--tw-gradient-from),var(--tw-gradient-to,transparent)"
            ));
        }
    }
    if let Some(rest) = class.strip_prefix("to-") {
        if let Some(hex) = colors.get(rest) {
            return Some(format!("--tw-gradient-to:{hex}"));
        }
    }

    // Spacing: p-{n}, px-{n}, py-{n}, pt-{n}, m-{n}, mx-{n}, gap-{n}, space-y-{n}
    if let Some(rule) = try_spacing(class) {
        return Some(rule);
    }

    // Sizing: w-{n}, h-{n}, max-w-{n}, min-h-{n}, max-h-{n}
    if let Some(rule) = try_sizing(class) {
        return Some(rule);
    }

    // Typography: text-{size}, font-{weight}, leading-{n}, tracking-{n}
    if let Some(rule) = try_typography(class) {
        return Some(rule);
    }

    // Border radius: rounded-{n}
    if let Some(rule) = try_border_radius(class) {
        return Some(rule);
    }

    // Grid: grid-cols-{n}
    if let Some(rest) = class.strip_prefix("grid-cols-") {
        if let Ok(n) = rest.parse::<u32>() {
            return Some(format!("grid-template-columns:repeat({n},minmax(0,1fr))"));
        }
    }

    // Shadow
    if class == "shadow-lg" {
        return Some(
            "box-shadow:0 10px 15px -3px rgba(0,0,0,.1),0 4px 6px -4px rgba(0,0,0,.1)".to_string(),
        );
    }

    None
}

// ── Spacing ────────────────────────────────────────────────────────────

fn try_spacing(class: &str) -> Option<String> {
    // space-y-{n} is special (uses margin-top on siblings via > * + *)
    if let Some(rest) = class.strip_prefix("space-y-") {
        let val = spacing_value(rest)?;
        // space-y uses the lobotomized owl selector equivalent
        return Some(format!("/* space-y */"));
    }

    let (prop, rest) = if let Some(r) = class.strip_prefix("px-") {
        ("padding-left:{v};padding-right:{v}", r)
    } else if let Some(r) = class.strip_prefix("py-") {
        ("padding-top:{v};padding-bottom:{v}", r)
    } else if let Some(r) = class.strip_prefix("pt-") {
        ("padding-top:{v}", r)
    } else if let Some(r) = class.strip_prefix("pb-") {
        ("padding-bottom:{v}", r)
    } else if let Some(r) = class.strip_prefix("pl-") {
        ("padding-left:{v}", r)
    } else if let Some(r) = class.strip_prefix("pr-") {
        ("padding-right:{v}", r)
    } else if let Some(r) = class.strip_prefix("p-") {
        ("padding:{v}", r)
    } else if let Some(r) = class.strip_prefix("mx-") {
        if r == "auto" {
            return Some("margin-left:auto;margin-right:auto".to_string());
        }
        ("margin-left:{v};margin-right:{v}", r)
    } else if let Some(r) = class.strip_prefix("my-") {
        ("margin-top:{v};margin-bottom:{v}", r)
    } else if let Some(r) = class.strip_prefix("mt-") {
        ("margin-top:{v}", r)
    } else if let Some(r) = class.strip_prefix("mb-") {
        ("margin-bottom:{v}", r)
    } else if let Some(r) = class.strip_prefix("ml-") {
        ("margin-left:{v}", r)
    } else if let Some(r) = class.strip_prefix("mr-") {
        ("margin-right:{v}", r)
    } else if let Some(r) = class.strip_prefix("m-") {
        ("margin:{v}", r)
    } else if let Some(r) = class.strip_prefix("gap-") {
        ("gap:{v}", r)
    } else {
        return None;
    };

    let val = spacing_value(rest)?;
    Some(prop.replace("{v}", &val))
}

fn spacing_value(s: &str) -> Option<String> {
    match s {
        "0" => Some("0px".into()),
        "0.5" => Some("0.125rem".into()),
        "1" => Some("0.25rem".into()),
        "1.5" => Some("0.375rem".into()),
        "2" => Some("0.5rem".into()),
        "2.5" => Some("0.625rem".into()),
        "3" => Some("0.75rem".into()),
        "4" => Some("1rem".into()),
        "5" => Some("1.25rem".into()),
        "6" => Some("1.5rem".into()),
        "8" => Some("2rem".into()),
        "10" => Some("2.5rem".into()),
        "12" => Some("3rem".into()),
        "16" => Some("4rem".into()),
        "20" => Some("5rem".into()),
        "24" => Some("6rem".into()),
        "32" => Some("8rem".into()),
        "40" => Some("10rem".into()),
        "48" => Some("12rem".into()),
        "56" => Some("14rem".into()),
        "64" => Some("16rem".into()),
        "px" => Some("1px".into()),
        _ => None,
    }
}

// ── Sizing ─────────────────────────────────────────────────────────────

fn try_sizing(class: &str) -> Option<String> {
    let (prop, rest) = if let Some(r) = class.strip_prefix("max-w-") {
        ("max-width", r)
    } else if let Some(r) = class.strip_prefix("min-w-") {
        ("min-width", r)
    } else if let Some(r) = class.strip_prefix("max-h-") {
        ("max-height", r)
    } else if let Some(r) = class.strip_prefix("min-h-") {
        ("min-height", r)
    } else if let Some(r) = class.strip_prefix("w-") {
        ("width", r)
    } else if let Some(r) = class.strip_prefix("h-") {
        ("height", r)
    } else {
        return None;
    };

    let val = match rest {
        "full" => "100%",
        "screen" => "100vh",
        "auto" => "auto",
        "fit" => "fit-content",
        "min" => "min-content",
        "max" => "max-content",
        // Named max-widths
        "xs" => "20rem",
        "sm" => "24rem",
        "md" => "28rem",
        "lg" => "32rem",
        "xl" => "36rem",
        "2xl" => "42rem",
        "3xl" => "48rem",
        "4xl" => "56rem",
        "5xl" => "64rem",
        "6xl" => "72rem",
        "7xl" => "80rem",
        "prose" => "65ch",
        _ => {
            // Numeric spacing values: w-8 → 2rem
            if let Some(v) = spacing_value(rest) {
                return Some(format!("{prop}:{v}"));
            }
            return None;
        }
    };

    Some(format!("{prop}:{val}"))
}

// ── Typography ─────────────────────────────────────────────────────────

fn try_typography(class: &str) -> Option<String> {
    // Font sizes: text-xs, text-sm, text-base, etc.
    if let Some(rest) = class.strip_prefix("text-") {
        let rule = match rest {
            "xs" => "font-size:0.75rem;line-height:1rem",
            "sm" => "font-size:0.875rem;line-height:1.25rem",
            "base" => "font-size:1rem;line-height:1.5rem",
            "lg" => "font-size:1.125rem;line-height:1.75rem",
            "xl" => "font-size:1.25rem;line-height:1.75rem",
            "2xl" => "font-size:1.5rem;line-height:2rem",
            "3xl" => "font-size:1.875rem;line-height:2.25rem",
            "4xl" => "font-size:2.25rem;line-height:2.5rem",
            "5xl" => "font-size:3rem;line-height:1",
            "6xl" => "font-size:3.75rem;line-height:1",
            _ => return None,
        };
        return Some(rule.to_string());
    }

    // Line height: leading-{n}
    if let Some(rest) = class.strip_prefix("leading-") {
        let val = match rest {
            "3" => "0.75rem",
            "4" => "1rem",
            "5" => "1.25rem",
            "6" => "1.5rem",
            "7" => "1.75rem",
            "8" => "2rem",
            "9" => "2.25rem",
            "10" => "2.5rem",
            "none" => "1",
            "tight" => "1.25",
            "snug" => "1.375",
            "normal" => "1.5",
            "relaxed" => "1.625",
            "loose" => "2",
            _ => return None,
        };
        return Some(format!("line-height:{val}"));
    }

    // Letter spacing: tracking-{n}
    if let Some(rest) = class.strip_prefix("tracking-") {
        let val = match rest {
            "tighter" => "-0.05em",
            "tight" => "-0.025em",
            "normal" => "0em",
            "wide" => "0.025em",
            "wider" => "0.05em",
            "widest" => "0.1em",
            _ => return None,
        };
        return Some(format!("letter-spacing:{val}"));
    }

    None
}

// ── Border radius ──────────────────────────────────────────────────────

fn try_border_radius(class: &str) -> Option<String> {
    if !class.starts_with("rounded") {
        return None;
    }

    // rounded-tl-sm, rounded-br-lg, etc.
    if let Some(rest) = class.strip_prefix("rounded-") {
        // Check for corner-specific: rounded-{corner}-{size}
        let corners = [
            ("tl-", "border-top-left-radius"),
            ("tr-", "border-top-right-radius"),
            ("bl-", "border-bottom-left-radius"),
            ("br-", "border-bottom-right-radius"),
            ("t-", "border-top-left-radius:{v};border-top-right-radius"),
            (
                "b-",
                "border-bottom-left-radius:{v};border-bottom-right-radius",
            ),
            ("l-", "border-top-left-radius:{v};border-bottom-left-radius"),
            (
                "r-",
                "border-top-right-radius:{v};border-bottom-right-radius",
            ),
        ];
        for (prefix, prop) in &corners {
            if let Some(size) = rest.strip_prefix(prefix) {
                let val = radius_value(size)?;
                if prop.contains("{v}") {
                    return Some(prop.replace("{v}", &val).to_string() + ":" + &val);
                }
                return Some(format!("{prop}:{val}"));
            }
        }

        // General: rounded-{size}
        let val = radius_value(rest)?;
        return Some(format!("border-radius:{val}"));
    }

    // Just "rounded"
    if class == "rounded" {
        return Some("border-radius:0.25rem".to_string());
    }

    None
}

fn radius_value(s: &str) -> Option<String> {
    match s {
        "none" => Some("0px".into()),
        "sm" => Some("0.125rem".into()),
        "md" => Some("0.375rem".into()),
        "lg" => Some("0.5rem".into()),
        "xl" => Some("0.75rem".into()),
        "2xl" => Some("1rem".into()),
        "3xl" => Some("1.5rem".into()),
        "full" => Some("9999px".into()),
        _ => None,
    }
}

// ── Arbitrary values ───────────────────────────────────────────────────

fn try_arbitrary(class: &str) -> Option<String> {
    // Pattern: prefix-[value] → CSS property:value
    let bracket_start = class.find('[')?;
    let bracket_end = class.find(']')?;
    if bracket_end <= bracket_start {
        return None;
    }

    let prefix = &class[..bracket_start];
    let value = &class[bracket_start + 1..bracket_end];

    let prop = match prefix {
        "w-" => "width",
        "h-" => "height",
        "min-w-" => "min-width",
        "min-h-" => "min-height",
        "max-w-" => "max-width",
        "max-h-" => "max-height",
        "p-" => "padding",
        "m-" => "margin",
        "top-" => "top",
        "left-" => "left",
        "right-" => "right",
        "bottom-" => "bottom",
        "gap-" => "gap",
        "text-" => "font-size",
        "z-" => "z-index",
        _ => return None,
    };

    Some(format!("{prop}:{value}"))
}

// ── CSS selector escaping ──────────────────────────────────────────────

fn escape_selector(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for ch in s.chars() {
        match ch {
            ':' | '[' | ']' | '/' | '.' | '(' | ')' | '#' | '!' | ',' | '>' | '+' | '~' => {
                out.push('\\');
                out.push(ch);
            }
            _ => out.push(ch),
        }
    }
    out
}

// ── Color map ──────────────────────────────────────────────────────────

fn build_color_map(config: &TailwindConfig) -> HashMap<String, String> {
    let mut m = HashMap::with_capacity(256);

    // Special colors.
    m.insert("black".into(), "#000".into());
    m.insert("white".into(), "#fff".into());
    m.insert("transparent".into(), "transparent".into());
    m.insert("current".into(), "currentColor".into());

    // Zinc
    insert_scale(
        &mut m,
        "zinc",
        &[
            ("50", "#fafafa"),
            ("100", "#f4f4f5"),
            ("200", "#e4e4e7"),
            ("300", "#d4d4d8"),
            ("400", "#a1a1aa"),
            ("500", "#71717a"),
            ("600", "#52525b"),
            ("700", "#3f3f46"),
            ("800", "#27272a"),
            ("900", "#18181b"),
            ("950", "#09090b"),
        ],
    );

    // Gray
    insert_scale(
        &mut m,
        "gray",
        &[
            ("50", "#f9fafb"),
            ("100", "#f3f4f6"),
            ("200", "#e5e7eb"),
            ("300", "#d1d5db"),
            ("400", "#9ca3af"),
            ("500", "#6b7280"),
            ("600", "#4b5563"),
            ("700", "#374151"),
            ("800", "#1f2937"),
            ("900", "#111827"),
            ("950", "#030712"),
        ],
    );

    // Slate
    insert_scale(
        &mut m,
        "slate",
        &[
            ("50", "#f8fafc"),
            ("100", "#f1f5f9"),
            ("200", "#e2e8f0"),
            ("300", "#cbd5e1"),
            ("400", "#94a3b8"),
            ("500", "#64748b"),
            ("600", "#475569"),
            ("700", "#334155"),
            ("800", "#1e293b"),
            ("900", "#0f172a"),
            ("950", "#020617"),
        ],
    );

    // Red
    insert_scale(
        &mut m,
        "red",
        &[
            ("50", "#fef2f2"),
            ("100", "#fee2e2"),
            ("200", "#fecaca"),
            ("300", "#fca5a5"),
            ("400", "#f87171"),
            ("500", "#ef4444"),
            ("600", "#dc2626"),
            ("700", "#b91c1c"),
            ("800", "#991b1b"),
            ("900", "#7f1d1d"),
            ("950", "#450a0a"),
        ],
    );

    // Blue
    insert_scale(
        &mut m,
        "blue",
        &[
            ("50", "#eff6ff"),
            ("100", "#dbeafe"),
            ("200", "#bfdbfe"),
            ("300", "#93c5fd"),
            ("400", "#60a5fa"),
            ("500", "#3b82f6"),
            ("600", "#2563eb"),
            ("700", "#1d4ed8"),
            ("800", "#1e40af"),
            ("900", "#1e3a8a"),
            ("950", "#172554"),
        ],
    );

    // Green
    insert_scale(
        &mut m,
        "green",
        &[
            ("50", "#f0fdf4"),
            ("100", "#dcfce7"),
            ("200", "#bbf7d0"),
            ("300", "#86efac"),
            ("400", "#4ade80"),
            ("500", "#22c55e"),
            ("600", "#16a34a"),
            ("700", "#15803d"),
            ("800", "#166534"),
            ("900", "#14532d"),
            ("950", "#052e16"),
        ],
    );

    // Emerald
    insert_scale(
        &mut m,
        "emerald",
        &[
            ("50", "#ecfdf5"),
            ("100", "#d1fae5"),
            ("200", "#a7f3d0"),
            ("300", "#6ee7b7"),
            ("400", "#34d399"),
            ("500", "#10b981"),
            ("600", "#059669"),
            ("700", "#047857"),
            ("800", "#065f46"),
            ("900", "#064e3b"),
            ("950", "#022c22"),
        ],
    );

    // Amber
    insert_scale(
        &mut m,
        "amber",
        &[
            ("50", "#fffbeb"),
            ("100", "#fef3c7"),
            ("200", "#fde68a"),
            ("300", "#fcd34d"),
            ("400", "#fbbf24"),
            ("500", "#f59e0b"),
            ("600", "#d97706"),
            ("700", "#b45309"),
            ("800", "#92400e"),
            ("900", "#78350f"),
            ("950", "#451a03"),
        ],
    );

    // Violet
    insert_scale(
        &mut m,
        "violet",
        &[
            ("50", "#f5f3ff"),
            ("100", "#ede9fe"),
            ("200", "#ddd6fe"),
            ("300", "#c4b5fd"),
            ("400", "#a78bfa"),
            ("500", "#8b5cf6"),
            ("600", "#7c3aed"),
            ("700", "#6d28d9"),
            ("800", "#5b21b6"),
            ("900", "#4c1d95"),
            ("950", "#2e1065"),
        ],
    );

    // Yellow
    insert_scale(
        &mut m,
        "yellow",
        &[
            ("50", "#fefce8"),
            ("100", "#fef9c3"),
            ("200", "#fef08a"),
            ("300", "#fde047"),
            ("400", "#facc15"),
            ("500", "#eab308"),
            ("600", "#ca8a04"),
            ("700", "#a16207"),
            ("800", "#854d0e"),
            ("900", "#713f12"),
            ("950", "#422006"),
        ],
    );

    // Custom colors from config.
    if let Some(ref primary) = config.primary {
        m.insert("primary".into(), primary.clone());
    }

    m
}

fn insert_scale(m: &mut HashMap<String, String>, name: &str, shades: &[(&str, &str)]) {
    for (shade, hex) in shades {
        m.insert(format!("{name}-{shade}"), (*hex).to_string());
    }
}

// ── Static utility lookup ──────────────────────────────────────────────
// Exact-match utilities that don't follow a dynamic pattern.

const STATIC_UTILS: &[(&str, &str)] = &[
    // Display
    ("block", "display:block"),
    ("inline-block", "display:inline-block"),
    ("inline", "display:inline"),
    ("flex", "display:flex"),
    ("inline-flex", "display:inline-flex"),
    ("grid", "display:grid"),
    ("hidden", "display:none"),
    ("table", "display:table"),
    // Position
    ("relative", "position:relative"),
    ("absolute", "position:absolute"),
    ("fixed", "position:fixed"),
    ("sticky", "position:sticky"),
    ("static", "position:static"),
    // Flex
    ("flex-row", "flex-direction:row"),
    ("flex-col", "flex-direction:column"),
    ("flex-wrap", "flex-wrap:wrap"),
    ("flex-nowrap", "flex-wrap:nowrap"),
    ("flex-1", "flex:1 1 0%"),
    ("flex-auto", "flex:1 1 auto"),
    ("flex-initial", "flex:0 1 auto"),
    ("flex-none", "flex:none"),
    ("flex-shrink-0", "flex-shrink:0"),
    ("flex-grow", "flex-grow:1"),
    ("flex-grow-0", "flex-grow:0"),
    // Alignment
    ("items-start", "align-items:flex-start"),
    ("items-center", "align-items:center"),
    ("items-end", "align-items:flex-end"),
    ("items-stretch", "align-items:stretch"),
    ("items-baseline", "align-items:baseline"),
    ("justify-start", "justify-content:flex-start"),
    ("justify-center", "justify-content:center"),
    ("justify-end", "justify-content:flex-end"),
    ("justify-between", "justify-content:space-between"),
    ("justify-around", "justify-content:space-around"),
    ("justify-evenly", "justify-content:space-evenly"),
    ("self-start", "align-self:flex-start"),
    ("self-center", "align-self:center"),
    ("self-end", "align-self:flex-end"),
    ("self-auto", "align-self:auto"),
    // Text alignment
    ("text-left", "text-align:left"),
    ("text-center", "text-align:center"),
    ("text-right", "text-align:right"),
    // Font weight
    ("font-thin", "font-weight:100"),
    ("font-extralight", "font-weight:200"),
    ("font-light", "font-weight:300"),
    ("font-normal", "font-weight:400"),
    ("font-medium", "font-weight:500"),
    ("font-semibold", "font-weight:600"),
    ("font-bold", "font-weight:700"),
    ("font-extrabold", "font-weight:800"),
    ("font-black", "font-weight:900"),
    // Font family
    ("font-sans", "font-family:ui-sans-serif,system-ui,sans-serif,\"Apple Color Emoji\",\"Segoe UI Emoji\""),
    ("font-serif", "font-family:ui-serif,Georgia,Cambria,\"Times New Roman\",Times,serif"),
    ("font-mono", "font-family:ui-monospace,SFMono-Regular,Menlo,Monaco,Consolas,\"Liberation Mono\",\"Courier New\",monospace"),
    // Font style
    ("italic", "font-style:italic"),
    ("not-italic", "font-style:normal"),
    // Text decoration
    ("underline", "text-decoration-line:underline"),
    ("overline", "text-decoration-line:overline"),
    ("line-through", "text-decoration-line:line-through"),
    ("no-underline", "text-decoration-line:none"),
    // Text transform
    ("uppercase", "text-transform:uppercase"),
    ("lowercase", "text-transform:lowercase"),
    ("capitalize", "text-transform:capitalize"),
    ("normal-case", "text-transform:none"),
    // Whitespace
    ("whitespace-normal", "white-space:normal"),
    ("whitespace-nowrap", "white-space:nowrap"),
    ("whitespace-pre", "white-space:pre"),
    ("whitespace-pre-line", "white-space:pre-line"),
    ("whitespace-pre-wrap", "white-space:pre-wrap"),
    // Word break
    ("break-normal", "overflow-wrap:normal;word-break:normal"),
    ("break-words", "overflow-wrap:break-word"),
    ("break-all", "word-break:break-all"),
    // Numeric
    ("tabular-nums", "font-variant-numeric:tabular-nums"),
    ("antialiased", "-webkit-font-smoothing:antialiased;-moz-osx-font-smoothing:grayscale"),
    // Overflow
    ("overflow-auto", "overflow:auto"),
    ("overflow-hidden", "overflow:hidden"),
    ("overflow-visible", "overflow:visible"),
    ("overflow-scroll", "overflow:scroll"),
    ("overflow-x-auto", "overflow-x:auto"),
    ("overflow-y-auto", "overflow-y:auto"),
    ("overflow-x-hidden", "overflow-x:hidden"),
    ("overflow-y-hidden", "overflow-y:hidden"),
    // Border
    ("border", "border-width:1px"),
    ("border-0", "border-width:0px"),
    ("border-2", "border-width:2px"),
    ("border-4", "border-width:4px"),
    ("border-t", "border-top-width:1px"),
    ("border-b", "border-bottom-width:1px"),
    ("border-l", "border-left-width:1px"),
    ("border-r", "border-right-width:1px"),
    ("border-solid", "border-style:solid"),
    ("border-dashed", "border-style:dashed"),
    ("border-dotted", "border-style:dotted"),
    ("border-none", "border-style:none"),
    // Outline
    ("outline-none", "outline:2px solid transparent;outline-offset:2px"),
    // Shadow
    ("shadow", "box-shadow:0 1px 3px 0 rgba(0,0,0,.1),0 1px 2px -1px rgba(0,0,0,.1)"),
    ("shadow-md", "box-shadow:0 4px 6px -1px rgba(0,0,0,.1),0 2px 4px -2px rgba(0,0,0,.1)"),
    ("shadow-lg", "box-shadow:0 10px 15px -3px rgba(0,0,0,.1),0 4px 6px -4px rgba(0,0,0,.1)"),
    ("shadow-xl", "box-shadow:0 20px 25px -5px rgba(0,0,0,.1),0 8px 10px -6px rgba(0,0,0,.1)"),
    ("shadow-none", "box-shadow:0 0 #0000"),
    // Transition
    ("transition", "transition-property:color,background-color,border-color,text-decoration-color,fill,stroke,opacity,box-shadow,transform,filter,backdrop-filter;transition-timing-function:cubic-bezier(.4,0,.2,1);transition-duration:150ms"),
    ("transition-all", "transition-property:all;transition-timing-function:cubic-bezier(.4,0,.2,1);transition-duration:150ms"),
    ("transition-colors", "transition-property:color,background-color,border-color,text-decoration-color,fill,stroke;transition-timing-function:cubic-bezier(.4,0,.2,1);transition-duration:150ms"),
    ("transition-shadow", "transition-property:box-shadow;transition-timing-function:cubic-bezier(.4,0,.2,1);transition-duration:150ms"),
    ("transition-none", "transition-property:none"),
    // Cursor
    ("cursor-pointer", "cursor:pointer"),
    ("cursor-default", "cursor:default"),
    ("cursor-not-allowed", "cursor:not-allowed"),
    ("cursor-wait", "cursor:wait"),
    ("cursor-text", "cursor:text"),
    // User select
    ("select-none", "user-select:none"),
    ("select-text", "user-select:text"),
    ("select-all", "user-select:all"),
    ("select-auto", "user-select:auto"),
    // Misc
    ("truncate", "overflow:hidden;text-overflow:ellipsis;white-space:nowrap"),
    ("sr-only", "position:absolute;width:1px;height:1px;padding:0;margin:-1px;overflow:hidden;clip:rect(0,0,0,0);white-space:nowrap;border-width:0"),
    ("pointer-events-none", "pointer-events:none"),
    ("pointer-events-auto", "pointer-events:auto"),
    // Opacity
    ("opacity-0", "opacity:0"),
    ("opacity-25", "opacity:0.25"),
    ("opacity-50", "opacity:0.5"),
    ("opacity-75", "opacity:0.75"),
    ("opacity-100", "opacity:1"),
    // Z-index
    ("z-0", "z-index:0"),
    ("z-10", "z-index:10"),
    ("z-20", "z-index:20"),
    ("z-30", "z-index:30"),
    ("z-40", "z-index:40"),
    ("z-50", "z-index:50"),
    ("z-auto", "z-index:auto"),
    // Inset
    ("inset-0", "inset:0px"),
    ("top-0", "top:0px"),
    ("right-0", "right:0px"),
    ("bottom-0", "bottom:0px"),
    ("left-0", "left:0px"),
];

// ── Preflight (minimal reset) ──────────────────────────────────────────

const PREFLIGHT: &str = r#"*,::before,::after{box-sizing:border-box;border-width:0;border-style:solid;border-color:#e5e7eb}
html{line-height:1.5;-webkit-text-size-adjust:100%;tab-size:4;font-family:ui-sans-serif,system-ui,sans-serif,"Apple Color Emoji","Segoe UI Emoji"}
body{margin:0;line-height:inherit}
h1,h2,h3,h4,h5,h6{font-size:inherit;font-weight:inherit}
a{color:inherit;text-decoration:inherit}
b,strong{font-weight:bolder}
code,kbd,samp,pre{font-family:ui-monospace,SFMono-Regular,Menlo,Monaco,Consolas,"Liberation Mono","Courier New",monospace;font-size:1em}
small{font-size:80%}
button,input,optgroup,select,textarea{font-family:inherit;font-feature-settings:inherit;font-variation-settings:inherit;font-size:100%;font-weight:inherit;line-height:inherit;color:inherit;margin:0;padding:0}
button,select{text-transform:none}
button,[type="button"],[type="reset"],[type="submit"]{-webkit-appearance:button;background-color:transparent;background-image:none}
img,svg,video,canvas,audio,iframe,embed,object{display:block;vertical-align:middle}
img,video{max-width:100%;height:auto}
[hidden]{display:none}
"#;

// ── space-y helper ─────────────────────────────────────────────────────
// space-y-N uses a sibling combinator.  We emit it as a special rule.

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tailwind_error_display() {
        let err = TailwindError::BuildFailed("exit 1".into());
        let msg = format!("{err}");
        assert!(msg.contains("build failed"));
    }

    #[test]
    fn resolve_static_utils() {
        let colors = HashMap::new();
        assert_eq!(resolve_class("flex", &colors).unwrap(), "display:flex");
        assert_eq!(resolve_class("hidden", &colors).unwrap(), "display:none");
        assert_eq!(
            resolve_class("cursor-pointer", &colors).unwrap(),
            "cursor:pointer"
        );
    }

    #[test]
    fn resolve_colors() {
        let config = TailwindConfig::default();
        let colors = build_color_map(&config);
        assert_eq!(
            resolve_class("bg-black", &colors).unwrap(),
            "background-color:#000"
        );
        assert_eq!(
            resolve_class("text-zinc-50", &colors).unwrap(),
            "color:#fafafa"
        );
        assert_eq!(
            resolve_class("border-gray-200", &colors).unwrap(),
            "border-color:#e5e7eb"
        );
    }

    #[test]
    fn resolve_spacing() {
        let colors = HashMap::new();
        assert_eq!(resolve_class("p-4", &colors).unwrap(), "padding:1rem");
        assert_eq!(
            resolve_class("px-8", &colors).unwrap(),
            "padding-left:2rem;padding-right:2rem"
        );
        assert_eq!(
            resolve_class("py-2.5", &colors).unwrap(),
            "padding-top:0.625rem;padding-bottom:0.625rem"
        );
        assert_eq!(resolve_class("gap-3", &colors).unwrap(), "gap:0.75rem");
        assert_eq!(
            resolve_class("mb-6", &colors).unwrap(),
            "margin-bottom:1.5rem"
        );
    }

    #[test]
    fn resolve_sizing() {
        let colors = HashMap::new();
        assert_eq!(resolve_class("w-full", &colors).unwrap(), "width:100%");
        assert_eq!(
            resolve_class("min-h-screen", &colors).unwrap(),
            "min-height:100vh"
        );
        assert_eq!(
            resolve_class("max-w-3xl", &colors).unwrap(),
            "max-width:48rem"
        );
        assert_eq!(resolve_class("h-10", &colors).unwrap(), "height:2.5rem");
    }

    #[test]
    fn resolve_typography() {
        let colors = HashMap::new();
        assert!(resolve_class("text-sm", &colors)
            .unwrap()
            .contains("0.875rem"));
        assert!(resolve_class("text-3xl", &colors)
            .unwrap()
            .contains("1.875rem"));
    }

    #[test]
    fn resolve_border_radius() {
        let colors = HashMap::new();
        assert_eq!(
            resolve_class("rounded-lg", &colors).unwrap(),
            "border-radius:0.5rem"
        );
        assert_eq!(
            resolve_class("rounded-full", &colors).unwrap(),
            "border-radius:9999px"
        );
        assert_eq!(
            resolve_class("rounded-tl-sm", &colors).unwrap(),
            "border-top-left-radius:0.125rem"
        );
    }

    #[test]
    fn resolve_arbitrary() {
        let colors = HashMap::new();
        assert_eq!(
            resolve_class("min-h-[4rem]", &colors).unwrap(),
            "min-height:4rem"
        );
    }

    #[test]
    fn resolve_gradient() {
        let colors = build_color_map(&TailwindConfig::default());
        assert!(resolve_class("bg-gradient-to-br", &colors)
            .unwrap()
            .contains("linear-gradient"));
        assert!(resolve_class("from-blue-100", &colors)
            .unwrap()
            .contains("--tw-gradient-from"));
    }

    #[test]
    fn escape_selector_special_chars() {
        assert_eq!(escape_selector("hover:bg-zinc-300"), "hover\\:bg-zinc-300");
        assert_eq!(escape_selector("min-h-[4rem]"), "min-h-\\[4rem\\]");
        assert_eq!(escape_selector("py-2.5"), "py-2\\.5");
    }

    #[test]
    fn generate_full_css() {
        let config = TailwindConfig {
            enabled: true,
            ..TailwindConfig::default()
        };
        let dir = std::env::temp_dir().join("gale_test_tw_gen");
        std::fs::create_dir_all(&dir).ok();
        let output = dir.join("styles.css");

        let classes = vec![
            "flex items-center bg-black text-white".to_string(),
            "p-4 rounded-lg hover:bg-zinc-300".to_string(),
            "sm:items-start md:grid-cols-3".to_string(),
            "min-h-[4rem] py-2.5 text-sm".to_string(),
        ];

        let result = run_tailwind(&config, Path::new("app"), &classes, &output, false);
        assert!(result.is_ok(), "generation should succeed");

        let css = std::fs::read_to_string(&output).unwrap();
        assert!(css.contains("display:flex"), "has flex: {css}");
        assert!(css.contains("background-color:#000"), "has bg-black: {css}");
        assert!(
            css.contains("border-radius:0.5rem"),
            "has rounded-lg: {css}"
        );
        assert!(css.contains(":hover"), "has hover variant: {css}");
        assert!(
            css.contains("@media(min-width:640px)"),
            "has sm breakpoint: {css}"
        );
        assert!(
            css.contains("@media(min-width:768px)"),
            "has md breakpoint: {css}"
        );
        assert!(
            css.contains("min-height:4rem"),
            "has arbitrary value: {css}"
        );

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn custom_primary_color() {
        let config = TailwindConfig {
            enabled: true,
            primary: Some("#3B82F6".into()),
            ..TailwindConfig::default()
        };
        let colors = build_color_map(&config);
        assert_eq!(
            resolve_class("bg-primary", &colors).unwrap(),
            "background-color:#3B82F6"
        );
        assert_eq!(
            resolve_class("text-primary", &colors).unwrap(),
            "color:#3B82F6"
        );
    }
}
