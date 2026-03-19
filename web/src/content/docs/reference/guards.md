# Guards

Typed validation schemas that generate both server-side Rust validation and client-side JavaScript validation.

```text
guard CreateUser {
  name: string.trim().minLen(2).maxLen(50)
  email: string.trim().email()
  age: int.min(13).max(150)
  role: string.oneOf("user", "admin").default("user")
  bio: string.optional().maxLen(500)
}
```

## All 28 chain methods

**String-only** (GX0605–GX0618 if misused on wrong type):

| Method | Purpose | Error if misused |
|--------|---------|-----------------|
| `.trim()` | Strip whitespace | GX0616 |
| `.email()` | RFC email pattern | GX0605 |
| `.url()` | Valid URL format | GX0606 |
| `.uuid()` | UUID v4 format | GX0607 |
| `.regex(pattern)` | Custom regex match | GX0608 |
| `.minLen(n)` | Min character length | — |
| `.maxLen(n)` | Max character length | — |
| `.nonEmpty()` | Must not be empty | — |
| `.oneOf("a","b")` | Must be listed value | — |
| `.lower()` | Transform to lowercase | GX0617 |
| `.upper()` | Transform to uppercase | GX0618 |

**Numeric** (`int` and `float`):

| Method | Purpose | Error if misused |
|--------|---------|-----------------|
| `.min(n)` | Minimum value (GX0601 if arg invalid) | — |
| `.max(n)` | Maximum value (GX0602 if arg invalid) | — |
| `.range(a, b)` | Min and max (GX0604 if a > b) | — |
| `.positive()` | Must be > 0 | GX0611 |
| `.nonNegative()` | Must be >= 0 | GX0611 |

**Float-only:**

| Method | Purpose | Error if misused |
|--------|---------|-----------------|
| `.integer()` | Must be whole number | — |
| `.precision(n)` | Decimal places | GX0610 |

**Array-only:**

| Method | Purpose | Error if misused |
|--------|---------|-----------------|
| `.of(Guard)` | Element validation | GX0614 |
| `.unique()` | No duplicates | GX0615 |

**Universal:**

| Method | Purpose | Notes |
|--------|---------|-------|
| `.optional()` | Field may be omitted | — |
| `.nullable()` | Field may be null | — |
| `.default(v)` | Fallback value | GX0628 if type mismatch |
| `.validate(fn)` | Custom validation | — |
| `.transform(fn)` | Custom transform | GX0629 if return type wrong |

## Guard composition

```text
let partial = CreateUser.partial()       // All fields optional
let subset = CreateUser.pick("email")    // Only listed fields
let without = CreateUser.omit("bio")     // Exclude fields
```

## Compiler checks

| Code | Condition |
|------|-----------|
| GX0600 | Unknown chain method (28 known) |
| GX0603 | `.min()` value exceeds `.max()` |
| GX0619 | `.oneOf()` with no values |
| GX0620 | Duplicate `.oneOf()` variant |
| GX0626 | Guard reference not defined |
| GX0627 | Circular guard dependency (DFS) |
| GX0630 | Field has no validation chain (warning) |
| GX0631 | `.optional()` after `.default()` is redundant |
