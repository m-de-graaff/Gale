//! Guard field validation constraints.
//!
//! Validations are attached to [`GuardField`](super::ty::GuardField) entries
//! and describe runtime checks that values must pass beyond their base type.
//! Some validations are *checks* (reject invalid values) while others are
//! *transforms* (modify values before checking).
//!
//! Example in GaleX source:
//! ```text
//! guard User {
//!     name: string.trim().minLen(2).maxLen(100)
//!     email: string.email()
//!     age: int.range(0, 150)
//!     role: string.oneOf("admin", "user")
//!     bio: string.optional().maxLen(500)
//!     token: string.uuid()
//! }
//! ```

use smol_str::SmolStr;

/// A runtime validation constraint on a guard field.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Validation {
    // ── Numeric checks ─────────────────────────────────────────
    /// Minimum numeric value: `.min(n)`
    Min(i64),
    /// Maximum numeric value: `.max(n)`
    Max(i64),
    /// Combined min+max range check: `.range(min, max)`
    Range(i64, i64),

    // ── Length checks ──────────────────────────────────────────
    /// Minimum string/array length: `.minLen(n)`
    MinLen(usize),
    /// Maximum string/array length: `.maxLen(n)`
    MaxLen(usize),

    // ── Format checks ──────────────────────────────────────────
    /// Must be a valid email: `.email()`
    Email,
    /// Must be a valid URL: `.url()`
    Url,
    /// Must be a valid UUID v4: `.uuid()`
    Uuid,
    /// Must match a regex pattern: `.regex("pattern")`
    Regex(SmolStr),
    /// Must be one of the listed values: `.oneOf("a", "b")`
    OneOf(Vec<SmolStr>),

    // ── Numeric type checks ────────────────────────────────────
    /// Integer value only (no decimals): `.integer()`
    Integer,
    /// Positive number: `.positive()`
    Positive,
    /// Non-negative number: `.nonNegative()`
    NonNegative,

    // ── Emptiness checks ───────────────────────────────────────
    /// Non-empty string/array: `.nonEmpty()`
    NonEmpty,

    // ── Optionality / nullability ──────────────────────────────
    /// Optional field (may be absent or null): `.optional()`
    Optional,
    /// Allow null but require the field to be present: `.nullable()`
    Nullable,

    // ── Transforms (modify value before validation) ────────────
    /// Trim whitespace from both ends: `.trim()`
    Trim,
    /// Round to n decimal places: `.precision(n)`
    Precision(u32),
    /// Fill with a default value if missing: `.default(value)`
    /// The value is stored as a JSON-encoded string.
    Default(SmolStr),

    // ── Custom ─────────────────────────────────────────────────
    /// Custom validation function: `.validate(fnName)`
    Custom(SmolStr),
}

impl Validation {
    /// Human-readable description for error messages.
    pub fn description(&self) -> String {
        match self {
            Validation::Min(n) => format!("minimum value {}", n),
            Validation::Max(n) => format!("maximum value {}", n),
            Validation::Range(a, b) => format!("value between {} and {}", a, b),
            Validation::MinLen(n) => format!("minimum length {}", n),
            Validation::MaxLen(n) => format!("maximum length {}", n),
            Validation::Email => "valid email address".into(),
            Validation::Url => "valid URL".into(),
            Validation::Uuid => "valid UUID".into(),
            Validation::Regex(p) => format!("matches pattern /{}/", p),
            Validation::OneOf(vals) => {
                let joined: Vec<&str> = vals.iter().map(|s| s.as_str()).collect();
                format!("one of [{}]", joined.join(", "))
            }
            Validation::Integer => "integer value".into(),
            Validation::Positive => "positive number".into(),
            Validation::NonNegative => "non-negative number".into(),
            Validation::NonEmpty => "non-empty".into(),
            Validation::Optional => "optional".into(),
            Validation::Nullable => "nullable".into(),
            Validation::Trim => "trimmed".into(),
            Validation::Precision(n) => format!("rounded to {} decimal place(s)", n),
            Validation::Default(v) => format!("defaults to {}", v),
            Validation::Custom(name) => format!("custom validation `{}`", name),
        }
    }

    /// Whether this validation is a *transform* that modifies the value
    /// (vs. a *check* that only validates it).
    pub fn is_transform(&self) -> bool {
        matches!(
            self,
            Validation::Trim | Validation::Precision(_) | Validation::Default(_)
        )
    }
}
