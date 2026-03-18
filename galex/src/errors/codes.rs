//! All 331 GaleX error codes — the single source of truth.
//!
//! Codes are stable across versions. Once assigned, a code never changes meaning.
//! Ranges are reserved per subsystem so future codes slot in without renumbering.
//!
//! # Ranges
//!
//! | Range         | Subsystem                                |
//! |---------------|------------------------------------------|
//! | GX0001–GX0099 | Lexer / tokenizer                        |
//! | GX0100–GX0299 | Parser / syntax                          |
//! | GX0300–GX0499 | Type system / inference                   |
//! | GX0500–GX0599 | Boundary analysis (server/client/shared)  |
//! | GX0600–GX0699 | Guard system                             |
//! | GX0700–GX0799 | Template / UI system                     |
//! | GX0800–GX0899 | Module / import system                   |
//! | GX0900–GX0999 | Action / query / channel system          |
//! | GX1000–GX1099 | Store system                             |
//! | GX1100–GX1199 | Env system                               |
//! | GX1200–GX1299 | Routing / file structure                  |
//! | GX1300–GX1399 | Middleware system                         |
//! | GX1400–GX1499 | Head / SEO system                        |
//! | GX1500–GX1599 | Form system                              |
//! | GX1600–GX1699 | Reactivity (signal/derive/effect/watch)  |
//! | GX1700–GX1799 | Lint warnings                            |
//! | GX1800–GX1899 | Build / codegen                          |
//! | GX1900–GX1999 | Runtime errors (dev mode only)            |
//! | GX2000–GX2099 | Package / dependency system               |

#![allow(dead_code)]

use super::{DiagnosticLevel, ErrorCode};

use DiagnosticLevel::Error as E;
#[allow(unused_imports)]
use DiagnosticLevel::Hint as H;
use DiagnosticLevel::Warning as W;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// GX0001–GX0099 — Lexer / tokenizer
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub static GX0001: ErrorCode = ErrorCode::new(
    1,
    E,
    "Unterminated string literal",
    "string opened here but never closed before end of line or file",
);

pub static GX0002: ErrorCode = ErrorCode::new(
    2,
    E,
    "Unterminated template literal",
    "backtick template string opened here but never closed",
);

pub static GX0003: ErrorCode = ErrorCode::new(
    3,
    E,
    "Unterminated block comment",
    "`/*` opened here but no matching `*/` was found before EOF",
);

pub static GX0004: ErrorCode = ErrorCode::new(
    4,
    E,
    "Invalid escape sequence",
    "valid escapes: \\n \\t \\r \\\\ \\\" \\' \\` \\{ \\$ \\0 \\xHH \\u{HHHH}",
);

pub static GX0005: ErrorCode = ErrorCode::new(
    5,
    E,
    "Invalid number literal",
    "expected valid digits for this number base",
);

pub static GX0006: ErrorCode = ErrorCode::new(
    6,
    E,
    "Unexpected character",
    "this character is not valid in GaleX source",
);

pub static GX0007: ErrorCode = ErrorCode::new(
    7,
    E,
    "Empty character literal",
    "character literal `''` has no content",
);

pub static GX0008: ErrorCode = ErrorCode::new(
    8,
    E,
    "Number literal overflow",
    "integer literal exceeds the maximum representable value (i64 range)",
);

pub static GX0009: ErrorCode = ErrorCode::new(
    9,
    E,
    "Unterminated regex literal",
    "regex `/pattern` opened here but never closed with a matching `/`",
);

pub static GX0010: ErrorCode =
    ErrorCode::new(10, E, "Invalid regex flag", "valid regex flags: i, g, m, s");

pub static GX0011: ErrorCode = ErrorCode::new(
    11,
    E,
    "Invalid Unicode escape",
    "Unicode escape sequence `\\u{...}` is malformed or out of range",
);

pub static GX0012: ErrorCode = ErrorCode::new(
    12,
    E,
    "Unexpected null byte in source",
    "source files must not contain \\0 bytes",
);

pub static GX0013: ErrorCode = ErrorCode::new(
    13,
    W,
    "Trailing whitespace",
    "line has whitespace after the last visible character (fixable by `gale fmt`)",
);

pub static GX0014: ErrorCode = ErrorCode::new(
    14,
    W,
    "Mixed tabs and spaces",
    "GaleX expects spaces only for indentation",
);

pub static GX0015: ErrorCode = ErrorCode::new(
    15,
    E,
    "Nested template interpolation too deep",
    "template `${}` nesting exceeds maximum depth (3 levels)",
);

pub static GX0016: ErrorCode = ErrorCode::new(
    16,
    E,
    "Invalid binary literal",
    "`0b` prefix must be followed by `0` and `1` only",
);

pub static GX0017: ErrorCode = ErrorCode::new(
    17,
    E,
    "Invalid hex literal",
    "`0x` prefix must be followed by 0-9, a-f, A-F only",
);

pub static GX0018: ErrorCode = ErrorCode::new(
    18,
    E,
    "Unterminated template interpolation",
    "`${` inside template literal has no matching `}`",
);

pub static GX0019: ErrorCode = ErrorCode::new(
    19,
    E,
    "Number separator `_` in invalid position",
    "underscore cannot appear at start, end, or doubled in a number literal",
);

pub static GX0020: ErrorCode = ErrorCode::new(
    20,
    W,
    "File contains BOM",
    "GaleX files should be plain UTF-8 without a BOM marker",
);

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// GX0100–GX0299 — Parser / syntax
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub static GX0100: ErrorCode = ErrorCode::new(
    100,
    E,
    "Unexpected token",
    "the parser encountered a token it did not expect at this position",
);

pub static GX0101: ErrorCode = ErrorCode::new(
    101,
    E,
    "Unexpected end of file",
    "the file ended in the middle of an expression, block, or declaration",
);

pub static GX0102: ErrorCode = ErrorCode::new(
    102,
    E,
    "Expected `{` after `server` keyword",
    "a `server` block must be followed by `{`",
);

pub static GX0103: ErrorCode = ErrorCode::new(
    103,
    E,
    "Expected `{` after `client` keyword",
    "a `client` block must be followed by `{`",
);

pub static GX0104: ErrorCode = ErrorCode::new(
    104,
    E,
    "Expected `{` after `shared` keyword",
    "a `shared` block must be followed by `{`",
);

pub static GX0105: ErrorCode = ErrorCode::new(
    105,
    E,
    "Expected `{` after guard name",
    "a guard object declaration needs a body: `guard Name { ... }`",
);

pub static GX0106: ErrorCode = ErrorCode::new(
    106,
    E,
    "Expected `{` after store name",
    "a store declaration needs a body: `store Name { ... }`",
);

pub static GX0107: ErrorCode = ErrorCode::new(
    107,
    E,
    "Expected `(` after function name",
    "a function declaration needs a parameter list",
);

pub static GX0108: ErrorCode = ErrorCode::new(
    108,
    E,
    "Expected `)` to close parameter list",
    "a `(` in a function signature was never closed",
);

pub static GX0109: ErrorCode = ErrorCode::new(
    109,
    E,
    "Expected `}` to close block",
    "a `{` was never matched with a `}`",
);

pub static GX0110: ErrorCode = ErrorCode::new(
    110,
    E,
    "Expected `]` to close array",
    "a `[` was never matched with a `]`",
);

pub static GX0111: ErrorCode = ErrorCode::new(
    111,
    E,
    "Unterminated HTML tag",
    "HTML opening tag was never closed with `>` or `/>`",
);

pub static GX0112: ErrorCode = ErrorCode::new(
    112,
    E,
    "Mismatched HTML closing tag",
    "`</tag>` doesn't match the most recently opened tag",
);

pub static GX0113: ErrorCode = ErrorCode::new(
    113,
    E,
    "Unexpected closing tag",
    "closing tag appears without a matching opening tag",
);

pub static GX0114: ErrorCode = ErrorCode::new(
    114,
    E,
    "Duplicate attribute on element",
    "the same attribute name appears twice on one HTML element",
);

pub static GX0115: ErrorCode = ErrorCode::new(
    115,
    E,
    "Expected expression",
    "this position requires an expression but found something else",
);

pub static GX0116: ErrorCode = ErrorCode::new(
    116,
    E,
    "Expected type annotation after `:`",
    "a colon in a type annotation position is not followed by a valid type",
);

pub static GX0117: ErrorCode = ErrorCode::new(
    117,
    E,
    "Invalid assignment target",
    "the left side of `=` is not something that can be assigned to",
);

pub static GX0118: ErrorCode = ErrorCode::new(
    118,
    E,
    "`return` outside of function",
    "a `return` statement appears outside any function body",
);

pub static GX0119: ErrorCode = ErrorCode::new(
    119,
    E,
    "`await` outside of async context",
    "an `await` expression appears outside a function that supports async",
);

pub static GX0120: ErrorCode = ErrorCode::new(
    120,
    E,
    "`break` outside of loop",
    "a `break` statement appears outside a `for` loop",
);

pub static GX0121: ErrorCode = ErrorCode::new(
    121,
    E,
    "Multiple `server` blocks in one file",
    "a `.gx` file can only have one `server { }` block",
);

pub static GX0122: ErrorCode = ErrorCode::new(
    122,
    E,
    "Multiple `client` blocks in one file",
    "a `.gx` file can only have one `client { }` block",
);

pub static GX0123: ErrorCode = ErrorCode::new(
    123,
    E,
    "Multiple `shared` blocks in one file",
    "a `.gx` file can only have one `shared { }` block",
);

pub static GX0124: ErrorCode = ErrorCode::new(
    124,
    E,
    "`out ui` must have a name",
    "a UI component declaration requires a name: `out ui MyComponent() { }`",
);

pub static GX0125: ErrorCode = ErrorCode::new(
    125,
    E,
    "`out api` must have a body",
    "an API declaration requires a body: `out api { ... }`",
);

pub static GX0126: ErrorCode = ErrorCode::new(
    126, E,
    "Invalid `out` target",
    "`out` can only prefix `ui`, `api`, `guard`, `store`, `fn`, `type`, `enum`, `env`, `server action`",
);

pub static GX0127: ErrorCode = ErrorCode::new(
    127,
    E,
    "Duplicate parameter name",
    "two parameters in the same function have the same name",
);

pub static GX0128: ErrorCode = ErrorCode::new(
    128,
    E,
    "Spread `...` must be last parameter",
    "a rest/spread parameter must be the last one in the list",
);

pub static GX0129: ErrorCode = ErrorCode::new(
    129,
    E,
    "Default parameter after non-default",
    "a parameter with a default value appears before a parameter without one",
);

pub static GX0130: ErrorCode =
    ErrorCode::new(130, E, "Empty `when` block", "a `when` block has no body");

pub static GX0131: ErrorCode = ErrorCode::new(
    131,
    E,
    "`each` requires `in` keyword",
    "an `each` loop must use the form `each item in collection`",
);

pub static GX0132: ErrorCode = ErrorCode::new(
    132,
    E,
    "`each` requires iterable expression",
    "the expression after `in` must be iterable (array, Set, Map)",
);

pub static GX0133: ErrorCode = ErrorCode::new(
    133,
    E,
    "`empty` without preceding `each`",
    "an `empty` block must immediately follow an `each` block",
);

pub static GX0134: ErrorCode = ErrorCode::new(
    134,
    E,
    "`else` without preceding `when` or `if`",
    "an `else` or `else when` appears without a preceding condition",
);

pub static GX0135: ErrorCode = ErrorCode::new(
    135,
    E,
    "Invalid guard chain method",
    "this method is not recognized in a guard chain",
);

pub static GX0136: ErrorCode = ErrorCode::new(
    136,
    E,
    "Guard chain on non-primitive",
    "chain methods can only be applied to primitive types",
);

pub static GX0137: ErrorCode = ErrorCode::new(
    137,
    E,
    "Expected string literal for test name",
    "`test` must be followed by a string name: `test \"description\" { }`",
);

pub static GX0138: ErrorCode = ErrorCode::new(
    138,
    E,
    "`assert` outside of test block",
    "an `assert` statement appears outside a `test { }` block",
);

pub static GX0139: ErrorCode = ErrorCode::new(
    139,
    E,
    "Expected `->` for return type",
    "function signature uses an invalid return type separator",
);

pub static GX0140: ErrorCode = ErrorCode::new(
    140,
    E,
    "Trailing comma in expression",
    "a comma appears at the end of an expression where it is not allowed",
);

pub static GX0141: ErrorCode = ErrorCode::new(
    141,
    E,
    "Invalid destructuring pattern",
    "the destructuring pattern on the left of `=` or in parameters is malformed",
);

pub static GX0142: ErrorCode = ErrorCode::new(
    142,
    E,
    "`channel` requires direction `->` or `<->`",
    "a channel declaration must specify its message direction",
);

pub static GX0143: ErrorCode = ErrorCode::new(
    143,
    E,
    "`suspend` requires `fallback`",
    "a `suspend` block must provide a fallback UI: `suspend fallback={...} { }`",
);

pub static GX0144: ErrorCode = ErrorCode::new(
    144,
    E,
    "`slot` must be inside a `ui` component",
    "a `slot` declaration appears outside an `out ui` block",
);

pub static GX0145: ErrorCode = ErrorCode::new(
    145,
    E,
    "`into:` directive outside component call",
    "an `into:slotname` directive is used outside a parent component's children",
);

pub static GX0146: ErrorCode = ErrorCode::new(
    146,
    E,
    "Duplicate `head` block",
    "a component defines `head { }` more than once",
);

pub static GX0147: ErrorCode = ErrorCode::new(
    147,
    E,
    "`redirect` outside server context",
    "a `redirect()` call appears outside a `server { }` or `guard.gx`",
);

pub static GX0148: ErrorCode = ErrorCode::new(
    148,
    E,
    "`env` block requires `server` or `client` section",
    "an `out env { }` must contain at least one `server { }` or `client { }` sub-block",
);

pub static GX0149: ErrorCode = ErrorCode::new(
    149,
    E,
    "`middleware` requires `handle` function",
    "a `middleware.gx` file must contain a `fn handle(req, next)` function",
);

pub static GX0150: ErrorCode = ErrorCode::new(
    150,
    E,
    "Invalid event modifier",
    "valid modifiers: prevent, stop, once, self, enter, escape",
);

pub static GX0151: ErrorCode = ErrorCode::new(
    151,
    E,
    "Multiple `out ui Page()` in one file",
    "only one page component is allowed per `page.gx` file",
);

pub static GX0152: ErrorCode = ErrorCode::new(
    152,
    E,
    "`frozen` requires initializer",
    "a `frozen` binding must be initialized at declaration: `frozen X = value`",
);

pub static GX0153: ErrorCode = ErrorCode::new(
    153,
    E,
    "Invalid `transition` type",
    "valid types: fade, slide, scale, blur, custom",
);

pub static GX0154: ErrorCode = ErrorCode::new(
    154,
    E,
    "`query` URL must start with `/`",
    "a query declaration's URL must be an absolute path",
);

pub static GX0155: ErrorCode = ErrorCode::new(
    155,
    E,
    "Unexpected `server` block in API file",
    "API route files use `out api { }`, not `server { }` for handlers",
);

pub static GX0156: ErrorCode = ErrorCode::new(
    156,
    E,
    "`form:error` requires `field` attribute",
    "a `<form:error>` element must specify which field it displays errors for",
);

pub static GX0157: ErrorCode = ErrorCode::new(
    157,
    W,
    "Unnecessary semicolon",
    "GaleX does not require semicolons; newlines are sufficient",
);

pub static GX0158: ErrorCode = ErrorCode::new(
    158,
    E,
    "`ref` requires type annotation",
    "a `ref` declaration needs an explicit type: `ref canvas: HTMLCanvasElement`",
);

pub static GX0159: ErrorCode = ErrorCode::new(
    159,
    E,
    "Invalid `link` `prefetch` value",
    "valid values are \"hover\", \"load\", \"none\"",
);

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// GX0300–GX0499 — Type system / inference
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub static GX0300: ErrorCode = ErrorCode::new(
    300,
    E,
    "Type mismatch",
    "an expression has a type that doesn't match what is expected",
);

pub static GX0301: ErrorCode = ErrorCode::new(
    301,
    E,
    "Cannot assign type",
    "the right side of an assignment doesn't match the left side's type",
);

pub static GX0302: ErrorCode = ErrorCode::new(
    302,
    E,
    "Undefined variable",
    "this name has not been declared in any reachable scope",
);

pub static GX0303: ErrorCode = ErrorCode::new(
    303,
    E,
    "Cannot call non-function",
    "a call expression is used on something that is not a function",
);

pub static GX0304: ErrorCode = ErrorCode::new(
    304,
    E,
    "Missing argument in function call",
    "a required function parameter has no argument provided",
);

pub static GX0305: ErrorCode = ErrorCode::new(
    305,
    E,
    "Extra argument in function call",
    "more arguments are provided than the function accepts",
);

pub static GX0306: ErrorCode = ErrorCode::new(
    306,
    E,
    "Argument type mismatch",
    "an argument's type doesn't match the corresponding parameter",
);

pub static GX0307: ErrorCode = ErrorCode::new(
    307,
    E,
    "Property does not exist on type",
    "member access refers to a property that doesn't exist on the object's type",
);

pub static GX0308: ErrorCode = ErrorCode::new(
    308,
    E,
    "Index out of bounds for tuple",
    "a tuple index access exceeds the tuple's length",
);

pub static GX0309: ErrorCode = ErrorCode::new(
    309,
    E,
    "Cannot index type",
    "an index access is used on a type that doesn't support indexing",
);

pub static GX0310: ErrorCode = ErrorCode::new(
    310,
    E,
    "Operator cannot be applied to types",
    "a binary operator doesn't work with the given operand types",
);

pub static GX0311: ErrorCode = ErrorCode::new(
    311,
    E,
    "Cannot use `!` on type",
    "the logical NOT operator is applied to a non-boolean type",
);

pub static GX0312: ErrorCode = ErrorCode::new(
    312,
    E,
    "Cannot use unary `-` on type",
    "unary negation is applied to a non-numeric type",
);

pub static GX0313: ErrorCode = ErrorCode::new(
    313,
    E,
    "Cannot infer type",
    "the type checker cannot determine a type — an explicit annotation is needed",
);

pub static GX0314: ErrorCode = ErrorCode::new(
    314,
    E,
    "Circular type reference",
    "a type alias refers to itself, directly or indirectly",
);

pub static GX0315: ErrorCode = ErrorCode::new(
    315,
    E,
    "Duplicate type name",
    "two types or guards with the same name are declared in the same scope",
);

pub static GX0316: ErrorCode = ErrorCode::new(
    316,
    E,
    "Duplicate field in object type",
    "an object type or guard has two fields with the same name",
);

pub static GX0317: ErrorCode = ErrorCode::new(
    317,
    E,
    "Type is not iterable",
    "used in `for`/`each` on something that is not an array, Set, or Map",
);

pub static GX0318: ErrorCode = ErrorCode::new(
    318,
    E,
    "Type is not awaitable",
    "`await` is used on an expression that doesn't return a Promise",
);

pub static GX0319: ErrorCode = ErrorCode::new(
    319,
    E,
    "Return type mismatch",
    "a function's return statement produces a different type than declared",
);

pub static GX0320: ErrorCode = ErrorCode::new(
    320,
    E,
    "Not all code paths return a value",
    "a function with a return type has branches that don't return",
);

pub static GX0321: ErrorCode = ErrorCode::new(
    321,
    E,
    "Cannot spread type",
    "the `...` spread operator is used on something that is not an array or object",
);

pub static GX0322: ErrorCode = ErrorCode::new(
    322,
    E,
    "Type is not nullable",
    "assigning `null` to a type that doesn't include `| null`",
);

pub static GX0323: ErrorCode = ErrorCode::new(
    323,
    E,
    "Unsafe null access",
    "accessing `.property` on a value that could be null without a null check",
);

pub static GX0324: ErrorCode = ErrorCode::new(
    324,
    E,
    "Cannot compare types",
    "using `==` or `!=` between types that can never be equal",
);

pub static GX0325: ErrorCode = ErrorCode::new(
    325,
    E,
    "Exhaustiveness check failed on `when`",
    "a `when` chain on a union/enum doesn't cover all possible values",
);

pub static GX0326: ErrorCode = ErrorCode::new(
    326,
    W,
    "Unreachable code",
    "code exists after an unconditional return or redirect",
);

pub static GX0327: ErrorCode = ErrorCode::new(
    327,
    E,
    "Duplicate variable in scope",
    "a `let`, `mut`, or `signal` re-declares a name that already exists in the same scope",
);

pub static GX0328: ErrorCode = ErrorCode::new(
    328,
    E,
    "Cannot reassign immutable binding",
    "assignment to a `let` (immutable) binding — use `mut` for mutable bindings",
);

pub static GX0329: ErrorCode = ErrorCode::new(
    329,
    E,
    "Cannot mutate `frozen` binding",
    "any mutation attempt on a `frozen` binding, including nested property changes",
);

pub static GX0330: ErrorCode = ErrorCode::new(
    330,
    E,
    "Cannot mutate `frozen` parameter",
    "a function parameter marked `frozen` is being modified inside the function",
);

pub static GX0331: ErrorCode = ErrorCode::new(
    331,
    E,
    "Generic type requires type arguments",
    "a generic type is used without the required number of type parameters",
);

pub static GX0332: ErrorCode = ErrorCode::new(
    332,
    E,
    "Type argument does not satisfy constraint",
    "a type argument doesn't meet the generic's constraint",
);

pub static GX0333: ErrorCode = ErrorCode::new(
    333,
    E,
    "Recursive function needs explicit return type",
    "a function that calls itself can't have its return type inferred — annotate it",
);

pub static GX0334: ErrorCode = ErrorCode::new(
    334,
    E,
    "`void` function cannot return a value",
    "a function with no return type (implicitly void) has a `return expr` statement",
);

pub static GX0335: ErrorCode = ErrorCode::new(
    335,
    E,
    "Expected `int`, found `float`",
    "an integer is expected but a float was provided — GaleX distinguishes these",
);

pub static GX0336: ErrorCode = ErrorCode::new(
    336,
    E,
    "Integer division produces float",
    "division of two ints may produce a float — use `Math.floor()` to be explicit",
);

pub static GX0337: ErrorCode = ErrorCode::new(
    337,
    E,
    "Enum variant does not exist",
    "referencing a variant name that is not part of the enum",
);

pub static GX0338: ErrorCode = ErrorCode::new(
    338,
    W,
    "Comparison always true/false",
    "a comparison like `x == x` or `true == false` has a constant result",
);

pub static GX0339: ErrorCode = ErrorCode::new(
    339,
    E,
    "Union type has no common members",
    "accessing a property that doesn't exist on all branches of a union",
);

pub static GX0340: ErrorCode = ErrorCode::new(
    340,
    E,
    "Not a valid template expression",
    "only string, int, float, bool can be interpolated — objects/arrays need explicit conversion",
);

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// GX0500–GX0599 — Boundary analysis (server/client/shared)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub static GX0500: ErrorCode = ErrorCode::new(
    500,
    E,
    "Cannot access server binding in client block",
    "this variable is declared in `server { }` and cannot be referenced from `client { }`",
);

pub static GX0501: ErrorCode = ErrorCode::new(
    501,
    E,
    "Cannot access client binding in server block",
    "this variable is declared in `client { }` and cannot be referenced from `server { }`",
);

pub static GX0502: ErrorCode = ErrorCode::new(
    502,
    E,
    "Cannot access server binding in shared block",
    "a server-only binding is used in `shared { }`",
);

pub static GX0503: ErrorCode = ErrorCode::new(
    503,
    E,
    "Cannot access client binding in shared block",
    "a client-only binding (signal, ref, DOM API) is used in `shared { }`",
);

pub static GX0504: ErrorCode = ErrorCode::new(
    504,
    E,
    "Non-serializable type crosses server-client boundary",
    "data flowing from server to client must be JSON-serializable",
);

pub static GX0505: ErrorCode = ErrorCode::new(
    505,
    E,
    "Server import used in client block",
    "a module imported with server-only capabilities is accessed from client code",
);

pub static GX0506: ErrorCode = ErrorCode::new(
    506,
    E,
    "Client API used in server block",
    "a browser API (e.g., `document`, `window`) is used in server code",
);

pub static GX0507: ErrorCode = ErrorCode::new(
    507,
    E,
    "`signal` declared in server block",
    "signals are client-only reactive primitives — use `let` for server-side data",
);

pub static GX0508: ErrorCode = ErrorCode::new(
    508,
    E,
    "`derive` declared in server block",
    "derives are client-only computed values — use `let` for server-side computations",
);

pub static GX0509: ErrorCode = ErrorCode::new(
    509,
    E,
    "`effect` declared in server block",
    "effects are client-only — side effects on the server belong in actions or middleware",
);

pub static GX0510: ErrorCode = ErrorCode::new(
    510,
    E,
    "`ref` declared in server block",
    "DOM refs only exist on the client",
);

pub static GX0511: ErrorCode = ErrorCode::new(
    511,
    E,
    "`query` declared in server block",
    "queries are client-side data fetching — on the server, use `await` with direct calls",
);

pub static GX0512: ErrorCode = ErrorCode::new(
    512,
    E,
    "`action` declared in client block",
    "actions must be declared in `server { }` — they are automatically callable from the client",
);

pub static GX0513: ErrorCode = ErrorCode::new(
    513,
    E,
    "`channel` declared in client block",
    "channels must be declared in `server { }` — use `.subscribe()` on the client",
);

pub static GX0514: ErrorCode = ErrorCode::new(
    514,
    E,
    "Side effect in shared block",
    "`shared { }` only allows pure functions, guards, types, and enums",
);

pub static GX0515: ErrorCode = ErrorCode::new(
    515,
    E,
    "`async` function in shared block",
    "shared functions must be pure and synchronous",
);

pub static GX0516: ErrorCode = ErrorCode::new(
    516,
    E,
    "`action` in shared block",
    "actions are server-only",
);

pub static GX0517: ErrorCode = ErrorCode::new(
    517,
    E,
    "Server env accessed in client block",
    "only `GALE_PUBLIC_*` env vars are accessible on the client",
);

pub static GX0518: ErrorCode = ErrorCode::new(
    518,
    E,
    "`store` with server-only dependencies",
    "stores are client-only state containers — use actions for server communication",
);

pub static GX0519: ErrorCode = ErrorCode::new(
    519,
    W,
    "Server data is large (>100KB serialized)",
    "consider pagination or lazy loading for large data crossing the boundary",
);

pub static GX0520: ErrorCode = ErrorCode::new(
    520,
    E,
    "`redirect` in client block",
    "`redirect()` is server-side only — use `navigate()` for client-side routing",
);

pub static GX0521: ErrorCode = ErrorCode::new(
    521,
    E,
    "`out` in server block without `server action` prefix",
    "exporting from a server block requires `out server action`",
);

pub static GX0522: ErrorCode = ErrorCode::new(
    522,
    W,
    "Shared function has side effects",
    "shared functions should be pure — no console.log, Math.random(), or external mutation",
);

pub static GX0523: ErrorCode = ErrorCode::new(
    523,
    E,
    "`bind` directive references server binding",
    "`bind:x` must reference a client-side signal, not a server variable",
);

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// GX0600–GX0699 — Guard system
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub static GX0600: ErrorCode = ErrorCode::new(
    600,
    E,
    "Unknown guard chain method",
    "this method is not recognized for this guard type",
);

pub static GX0601: ErrorCode = ErrorCode::new(
    601,
    E,
    "`.min()` requires a numeric argument",
    "the argument to `.min()` must be a number literal",
);

pub static GX0602: ErrorCode = ErrorCode::new(
    602,
    E,
    "`.max()` requires a numeric argument",
    "the argument to `.max()` must be a number literal",
);

pub static GX0603: ErrorCode = ErrorCode::new(
    603,
    E,
    "`.min()` value exceeds `.max()` value",
    "impossible range: min is greater than max",
);

pub static GX0604: ErrorCode = ErrorCode::new(
    604,
    E,
    "`.range(a, b)` requires `a <= b`",
    "range bounds are inverted",
);

pub static GX0605: ErrorCode = ErrorCode::new(
    605,
    E,
    "`.email()` is only valid on `string`",
    "applying `.email()` to a non-string type",
);

pub static GX0606: ErrorCode = ErrorCode::new(
    606,
    E,
    "`.url()` is only valid on `string`",
    "applying `.url()` to a non-string type",
);

pub static GX0607: ErrorCode = ErrorCode::new(
    607,
    E,
    "`.uuid()` is only valid on `string`",
    "applying `.uuid()` to a non-string type",
);

pub static GX0608: ErrorCode = ErrorCode::new(
    608,
    E,
    "`.regex()` is only valid on `string`",
    "applying `.regex()` to a non-string type",
);

pub static GX0609: ErrorCode = ErrorCode::new(
    609,
    E,
    "`.regex()` requires a valid regex literal",
    "the argument to `.regex()` is not a valid regular expression",
);

pub static GX0610: ErrorCode = ErrorCode::new(
    610,
    E,
    "`.precision()` is only valid on `float`",
    "applying `.precision()` to a non-float type",
);

pub static GX0611: ErrorCode = ErrorCode::new(
    611,
    E,
    "`.positive()` is only valid on `int` or `float`",
    "applying `.positive()` to a non-numeric type",
);

pub static GX0612: ErrorCode = ErrorCode::new(
    612,
    E,
    "`.past()` is only valid on `datetime`",
    "applying `.past()` to a non-datetime type",
);

pub static GX0613: ErrorCode = ErrorCode::new(
    613,
    E,
    "`.future()` is only valid on `datetime`",
    "applying `.future()` to a non-datetime type",
);

pub static GX0614: ErrorCode = ErrorCode::new(
    614,
    E,
    "`.of()` is only valid on `array`",
    "applying `.of(Guard)` to a non-array type",
);

pub static GX0615: ErrorCode = ErrorCode::new(
    615,
    E,
    "`.unique()` is only valid on `array`",
    "applying `.unique()` to a non-array type",
);

pub static GX0616: ErrorCode = ErrorCode::new(
    616,
    E,
    "`.trim()` is only valid on `string`",
    "applying `.trim()` to a non-string type",
);

pub static GX0617: ErrorCode = ErrorCode::new(
    617,
    E,
    "`.lower()` is only valid on `string`",
    "applying `.lower()` to a non-string type",
);

pub static GX0618: ErrorCode = ErrorCode::new(
    618,
    E,
    "`.upper()` is only valid on `string`",
    "applying `.upper()` to a non-string type",
);

pub static GX0619: ErrorCode = ErrorCode::new(
    619,
    E,
    "`enum()` requires at least one value",
    "an enum guard has no variants",
);

pub static GX0620: ErrorCode = ErrorCode::new(
    620,
    E,
    "Duplicate enum variant",
    "an enum guard lists the same value twice",
);

pub static GX0621: ErrorCode = ErrorCode::new(
    621,
    E,
    "`.partial()` is only valid on object guards",
    "applying `.partial()` to a primitive guard",
);

pub static GX0622: ErrorCode = ErrorCode::new(
    622,
    E,
    "`.pick()` field does not exist on guard",
    "picking a field that the guard doesn't have",
);

pub static GX0623: ErrorCode = ErrorCode::new(
    623,
    E,
    "`.omit()` field does not exist on guard",
    "omitting a field that the guard doesn't have",
);

pub static GX0624: ErrorCode = ErrorCode::new(
    624,
    E,
    "Guard cannot extend non-object guard",
    "using `&` to extend a primitive guard",
);

pub static GX0625: ErrorCode = ErrorCode::new(
    625,
    E,
    "Conflicting field in guard extension",
    "extending a guard with a field that already exists with an incompatible type",
);

pub static GX0626: ErrorCode = ErrorCode::new(
    626,
    E,
    "Guard is not defined",
    "referencing a guard name that doesn't exist",
);

pub static GX0627: ErrorCode = ErrorCode::new(
    627,
    E,
    "Circular guard reference",
    "a guard references itself through its extension chain",
);

pub static GX0628: ErrorCode = ErrorCode::new(
    628,
    E,
    "`.default()` value doesn't match field type",
    "the default value's type doesn't match the guard field's type",
);

pub static GX0629: ErrorCode = ErrorCode::new(
    629,
    E,
    "`.transform()` return type doesn't match field type",
    "the transform function's return type doesn't match the field type",
);

pub static GX0630: ErrorCode = ErrorCode::new(
    630,
    W,
    "Guard field has no validation chain",
    "a field is declared with just a type and no constraints — consider adding validation",
);

pub static GX0631: ErrorCode = ErrorCode::new(
    631,
    E,
    "`.optional()` after `.default()` is redundant",
    "if a field has a default, it is implicitly optional",
);

pub static GX0632: ErrorCode = ErrorCode::new(
    632,
    E,
    "`.default(now)` is only valid on `datetime`",
    "the special `now` default only works with datetime fields",
);

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// GX0700–GX0799 — Template / UI system
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub static GX0700: ErrorCode = ErrorCode::new(
    700,
    E,
    "Unknown component",
    "this component name is not imported or declared",
);

pub static GX0701: ErrorCode = ErrorCode::new(
    701,
    E,
    "Missing required prop on component",
    "a component is called without a required property",
);

pub static GX0702: ErrorCode = ErrorCode::new(
    702,
    E,
    "Unknown prop on component",
    "this property is not accepted by the component",
);

pub static GX0703: ErrorCode = ErrorCode::new(
    703,
    E,
    "Prop type mismatch",
    "a property value doesn't match the component's declared type",
);

pub static GX0704: ErrorCode = ErrorCode::new(
    704,
    E,
    "Duplicate `key` value in `each` block",
    "two items in an `each` loop produce the same key value (runtime error in dev mode)",
);

pub static GX0705: ErrorCode = ErrorCode::new(
    705,
    W,
    "Missing `key` in `each` block",
    "an `each` block doesn't specify a `key` attribute — keys are required for efficient updates",
);

pub static GX0706: ErrorCode = ErrorCode::new(
    706,
    E,
    "`slot` name is not defined in component",
    "using `into:slotname` for a slot that the component doesn't declare",
);

pub static GX0707: ErrorCode = ErrorCode::new(
    707,
    E,
    "Multiple default slots",
    "a component declares more than one unnamed `slot`",
);

pub static GX0708: ErrorCode = ErrorCode::new(
    708,
    E,
    "Void element cannot have children",
    "a self-closing HTML element (e.g., `<img>`, `<input>`, `<br>`) has children",
);

pub static GX0709: ErrorCode = ErrorCode::new(
    709,
    E,
    "`<form:error>` outside `<form>`",
    "a form:error element appears outside a `<form>` with `form:guard`",
);

pub static GX0710: ErrorCode = ErrorCode::new(
    710,
    E,
    "`form:error` field not in guard",
    "the `field` attribute references a field that doesn't exist in the form's guard",
);

pub static GX0711: ErrorCode = ErrorCode::new(
    711,
    E,
    "Text content must be quoted in GaleX",
    "bare text inside templates must be wrapped in `\"quotes\"`",
);

pub static GX0712: ErrorCode = ErrorCode::new(
    712,
    E,
    "`bind:x` target is not a signal",
    "the `bind:` directive must reference a signal, not a `let` or `derive`",
);

pub static GX0713: ErrorCode = ErrorCode::new(
    713,
    E,
    "`bind:x` on non-input element",
    "`bind:` only works on `<input>`, `<textarea>`, `<select>`, or components with bindable props",
);

pub static GX0714: ErrorCode = ErrorCode::new(
    714,
    E,
    "`on:x` handler is not a function",
    "an event handler expression doesn't evaluate to a function",
);

pub static GX0715: ErrorCode = ErrorCode::new(
    715,
    E,
    "Unknown event on element",
    "an `on:event` references an event that doesn't exist on this HTML element",
);

pub static GX0716: ErrorCode = ErrorCode::new(
    716,
    E,
    "`class:x` condition is not boolean",
    "the expression in `class:name={expr}` must evaluate to `bool`",
);

pub static GX0717: ErrorCode = ErrorCode::new(
    717,
    E,
    "`ref:x` target is not declared as `ref`",
    "using `ref:name` in a template but `name` isn't declared as `ref` in the client block",
);

pub static GX0718: ErrorCode = ErrorCode::new(
    718,
    E,
    "`ref:x` type mismatch",
    "the ref's declared type doesn't match the actual HTML element",
);

pub static GX0719: ErrorCode = ErrorCode::new(
    719,
    E,
    "`transition` on non-conditional element",
    "transitions need enter/exit triggers (inside `when` or `each` blocks)",
);

pub static GX0720: ErrorCode = ErrorCode::new(
    720,
    W,
    "Inline `style` attribute",
    "consider using Tailwind classes instead of inline `style=\"...\"`",
);

pub static GX0721: ErrorCode = ErrorCode::new(
    721,
    E,
    "Self-closing component cannot have children",
    "`<Component />` is self-closing, so it can't also have children between tags",
);

pub static GX0722: ErrorCode = ErrorCode::new(
    722,
    W,
    "Deeply nested template (>10 levels)",
    "template nesting is extremely deep — consider extracting components",
);

pub static GX0723: ErrorCode = ErrorCode::new(
    723,
    E,
    "`<link>` element requires `href` attribute",
    "GaleX `<link>` (navigation) elements need a destination",
);

pub static GX0724: ErrorCode = ErrorCode::new(
    724,
    E,
    "Dynamic tag name not allowed",
    "tag names must be static strings, not expressions",
);

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// GX0800–GX0899 — Module / import system
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub static GX0800: ErrorCode = ErrorCode::new(
    800,
    E,
    "Module not found",
    "the `use ... from \"path\"` path doesn't resolve to any file or package",
);

pub static GX0801: ErrorCode = ErrorCode::new(
    801,
    E,
    "Named export not found in module",
    "the module doesn't export this symbol",
);

pub static GX0802: ErrorCode = ErrorCode::new(
    802,
    E,
    "Circular import detected",
    "two or more files import each other in a cycle",
);

pub static GX0803: ErrorCode = ErrorCode::new(
    803,
    E,
    "Ambiguous import",
    "this symbol is exported by multiple modules",
);

pub static GX0804: ErrorCode = ErrorCode::new(
    804,
    E,
    "Cannot import — symbol is not exported",
    "this symbol exists in the module but has no `out` keyword",
);

pub static GX0805: ErrorCode = ErrorCode::new(
    805,
    W,
    "Unused import",
    "this imported symbol is never referenced in the file",
);

pub static GX0806: ErrorCode = ErrorCode::new(
    806,
    E,
    "Cannot import server-only module in client block",
    "a module like `db/postgres` is imported in a client context",
);

pub static GX0807: ErrorCode = ErrorCode::new(
    807,
    E,
    "Package is not installed",
    "a `use ... from \"package/name\"` references a package not in `gale_modules/`",
);

pub static GX0808: ErrorCode = ErrorCode::new(
    808,
    E,
    "Package version conflict",
    "two dependencies require incompatible versions of the same package",
);

pub static GX0809: ErrorCode = ErrorCode::new(
    809,
    W,
    "Duplicate import",
    "the same symbol is imported twice (from the same or different paths)",
);

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// GX0900–GX0999 — Action / query / channel system
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub static GX0900: ErrorCode = ErrorCode::new(
    900,
    E,
    "Action is not defined",
    "calling an action that doesn't exist in the server block",
);

pub static GX0901: ErrorCode = ErrorCode::new(
    901,
    E,
    "Action parameter guard validation failed",
    "an action was called with data that fails its guard (runtime)",
);

pub static GX0902: ErrorCode = ErrorCode::new(
    902,
    E,
    "Action must be in a `server` block",
    "an `action` declaration appears outside `server { }`",
);

pub static GX0903: ErrorCode = ErrorCode::new(
    903,
    E,
    "Action return type is not serializable",
    "an action returns a type that can't be serialized to JSON for the client",
);

pub static GX0904: ErrorCode = ErrorCode::new(
    904,
    E,
    "Duplicate action name",
    "two actions with the same name in one file",
);

pub static GX0905: ErrorCode = ErrorCode::new(
    905,
    E,
    "Query URL contains unresolvable interpolation",
    "a `query` URL has `{expr}` where `expr` references an undefined variable",
);

pub static GX0906: ErrorCode = ErrorCode::new(
    906,
    E,
    "Query return type is not deserializable",
    "the type annotation on a query can't be deserialized from JSON",
);

pub static GX0907: ErrorCode = ErrorCode::new(
    907,
    E,
    "Channel is not defined",
    "calling `.subscribe()` or `.connect()` on an undeclared channel",
);

pub static GX0908: ErrorCode = ErrorCode::new(
    908,
    E,
    "Channel is unidirectional — cannot `.send()`",
    "calling `.send()` on a `->` (server->client) channel — use `<->` for bidirectional",
);

pub static GX0909: ErrorCode = ErrorCode::new(
    909,
    E,
    "Channel message type mismatch",
    "sending a message that doesn't match the channel's declared guard/type",
);

pub static GX0910: ErrorCode = ErrorCode::new(
    910,
    E,
    "Channel requires parameters",
    "a parameterized channel is subscribed to without arguments",
);

pub static GX0911: ErrorCode = ErrorCode::new(
    911,
    E,
    "Action called outside client or server context",
    "an action is invoked from `shared {}` or at the top level",
);

pub static GX0912: ErrorCode = ErrorCode::new(
    912,
    W,
    "Action has no guard on parameters",
    "an action accepts parameters but doesn't use a guard — consider adding validation",
);

pub static GX0913: ErrorCode = ErrorCode::new(
    913,
    E,
    "Query declared in server block",
    "queries are client-only — use direct DB calls on the server",
);

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// GX1000–GX1099 — Store system
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub static GX1000: ErrorCode = ErrorCode::new(
    1000,
    E,
    "Store is not defined",
    "referencing a store that hasn't been declared",
);

pub static GX1001: ErrorCode = ErrorCode::new(
    1001,
    E,
    "Store field does not exist",
    "accessing a signal, derive, or method that the store doesn't have",
);

pub static GX1002: ErrorCode = ErrorCode::new(
    1002,
    E,
    "Cannot mutate store signal outside store methods",
    "direct assignment to a store's signal from outside the store — use a store method",
);

pub static GX1003: ErrorCode = ErrorCode::new(
    1003,
    E,
    "Duplicate store name",
    "two stores with the same name",
);

pub static GX1004: ErrorCode = ErrorCode::new(
    1004,
    E,
    "Store contains `action` — use `fn` instead",
    "actions are server concepts — store methods should use `fn`",
);

pub static GX1005: ErrorCode = ErrorCode::new(
    1005,
    E,
    "Store contains `query` — use in component instead",
    "queries belong in components, not stores — stores hold derived/local state",
);

pub static GX1006: ErrorCode = ErrorCode::new(
    1006,
    W,
    "Store has no signals",
    "a store without any signals isn't reactive — consider using plain functions instead",
);

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// GX1100–GX1199 — Env system
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub static GX1100: ErrorCode = ErrorCode::new(
    1100,
    E,
    "Required env variable is not set",
    "a required environment variable has no value (build-time)",
);

pub static GX1101: ErrorCode = ErrorCode::new(
    1101,
    E,
    "Env variable failed validation",
    "an env var's value doesn't pass its guard chain",
);

pub static GX1102: ErrorCode = ErrorCode::new(
    1102,
    E,
    "Client env variable must start with `GALE_PUBLIC_`",
    "client-accessible env vars must have the public prefix",
);

pub static GX1103: ErrorCode = ErrorCode::new(
    1103,
    E,
    "Server env variable has `GALE_PUBLIC_` prefix",
    "a server-only var has the public prefix — move it to `client { }` or remove the prefix",
);

pub static GX1104: ErrorCode = ErrorCode::new(
    1104,
    E,
    "Duplicate env variable",
    "the same env var appears in both `server { }` and `client { }` sections",
);

pub static GX1105: ErrorCode = ErrorCode::new(
    1105,
    E,
    "Env variable is not declared in `env.gx`",
    "accessing `env.X` but `X` isn't defined in the env block",
);

pub static GX1106: ErrorCode = ErrorCode::new(
    1106,
    W,
    "Env variable has no validation chain",
    "an env var is declared with just a type — consider adding constraints",
);

pub static GX1107: ErrorCode = ErrorCode::new(
    1107,
    E,
    "`env` accessed outside server or client context",
    "accessing `env` from `shared {}` is not allowed — env vars have scope",
);

pub static GX1108: ErrorCode = ErrorCode::new(
    1108,
    E,
    "No `env.gx` file found",
    "code references `env.X` but no `env.gx` (or `out env {}` in `gale.toml`) exists",
);

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// GX1200–GX1299 — Routing / file structure
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub static GX1200: ErrorCode = ErrorCode::new(
    1200,
    E,
    "No `page.gx` found in route directory",
    "a directory in `app/` has files but no `page.gx` to define the route",
);

pub static GX1201: ErrorCode = ErrorCode::new(
    1201,
    E,
    "Conflicting routes resolve to the same URL",
    "two directories produce the same URL pattern",
);

pub static GX1202: ErrorCode = ErrorCode::new(
    1202,
    E,
    "Dynamic segment conflicts with static segment",
    "a directory has both `[slug]/` and `slug/` at the same level",
);

pub static GX1203: ErrorCode = ErrorCode::new(
    1203,
    E,
    "Multiple catch-all routes at same level",
    "only one catch-all segment `[...x]` is allowed per directory level",
);

pub static GX1204: ErrorCode = ErrorCode::new(
    1204,
    E,
    "`guard.gx` without `page.gx` in same directory or children",
    "a guard file exists in a directory that has no page to guard",
);

pub static GX1205: ErrorCode = ErrorCode::new(
    1205,
    E,
    "`layout.gx` must export `out ui Layout()`",
    "a layout file doesn't contain the required component",
);

pub static GX1206: ErrorCode = ErrorCode::new(
    1206,
    E,
    "`page.gx` must export `out ui Page()` or `out api {}`",
    "a page file doesn't contain the required export",
);

pub static GX1207: ErrorCode = ErrorCode::new(
    1207,
    E,
    "`error.gx` must export `out ui ErrorPage(error, reset)`",
    "an error boundary file has the wrong component signature",
);

pub static GX1208: ErrorCode = ErrorCode::new(
    1208,
    E,
    "`loading.gx` must export `out ui Loading()`",
    "a loading file doesn't contain the required component",
);

pub static GX1209: ErrorCode = ErrorCode::new(
    1209,
    W,
    "Empty route directory",
    "a directory in `app/` has no `.gx` files",
);

pub static GX1210: ErrorCode = ErrorCode::new(
    1210,
    E,
    "`middleware.gx` must contain `fn handle(req, next) -> Response`",
    "a middleware file is missing the handle function or has the wrong signature",
);

pub static GX1211: ErrorCode = ErrorCode::new(
    1211,
    E,
    "Dynamic route parameter has no type hint",
    "consider adding a type: `[id: int]` or use a guard in the page for validation",
);

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// GX1300–GX1399 — Middleware system
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub static GX1300: ErrorCode = ErrorCode::new(
    1300,
    E,
    "Middleware `handle` must call `next()` or return a `Response`",
    "a middleware function neither forwards the request nor returns a response",
);

pub static GX1301: ErrorCode = ErrorCode::new(
    1301,
    E,
    "Middleware `handle` has wrong signature",
    "expected `fn handle(req: Request, next: fn) -> Response`",
);

pub static GX1302: ErrorCode = ErrorCode::new(
    1302,
    E,
    "Middleware `handle` calls `next()` multiple times",
    "the `next()` handler should only be called once per request",
);

pub static GX1303: ErrorCode = ErrorCode::new(
    1303,
    W,
    "Middleware modifies response after stream started",
    "modifying headers after the response body has started streaming has no effect",
);

pub static GX1304: ErrorCode = ErrorCode::new(
    1304,
    E,
    "Middleware contains `signal` or client code",
    "middleware is server-only — no reactive or client-side code is allowed",
);

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// GX1400–GX1499 — Head / SEO system
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub static GX1400: ErrorCode = ErrorCode::new(
    1400,
    E,
    "Unknown `head` property",
    "valid: title, description, og, canonical, robots, favicon, css, script",
);

pub static GX1401: ErrorCode = ErrorCode::new(
    1401,
    E,
    "`head.title` must be a string",
    "the title property must evaluate to a string",
);

pub static GX1402: ErrorCode = ErrorCode::new(
    1402,
    E,
    "`head.og.image` must be a URL string",
    "the OG image must be a valid URL",
);

pub static GX1403: ErrorCode = ErrorCode::new(
    1403,
    W,
    "Missing `head.title` on page",
    "a page doesn't set a title — this harms SEO and accessibility",
);

pub static GX1404: ErrorCode = ErrorCode::new(
    1404,
    W,
    "Missing `head.description` on page",
    "a page doesn't set a meta description",
);

pub static GX1405: ErrorCode = ErrorCode::new(
    1405,
    W,
    "`head.title` exceeds 60 characters",
    "title tags over 60 characters are truncated by search engines",
);

pub static GX1406: ErrorCode = ErrorCode::new(
    1406,
    W,
    "`head.description` exceeds 160 characters",
    "meta descriptions over 160 characters are truncated by search engines",
);

pub static GX1407: ErrorCode = ErrorCode::new(
    1407,
    E,
    "`head.script.src` must be a URL string",
    "a script source must be a valid path or URL",
);

pub static GX1408: ErrorCode = ErrorCode::new(
    1408,
    E,
    "`head.css` must be an array of URL strings",
    "the `css` property must be an array of stylesheet paths",
);

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// GX1500–GX1599 — Form system
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub static GX1500: ErrorCode = ErrorCode::new(
    1500,
    E,
    "`form:action` references undefined action",
    "the action name doesn't exist in the server block",
);

pub static GX1501: ErrorCode = ErrorCode::new(
    1501,
    E,
    "`form:guard` references undefined guard",
    "the guard name doesn't exist",
);

pub static GX1502: ErrorCode = ErrorCode::new(
    1502,
    E,
    "`form:action` and `form:guard` parameter mismatch",
    "the form's guard type doesn't match the action's parameter type",
);

pub static GX1503: ErrorCode = ErrorCode::new(
    1503,
    E,
    "`form:onSuccess` handler has wrong signature",
    "the success handler should accept the action's return type",
);

pub static GX1504: ErrorCode = ErrorCode::new(
    1504,
    E,
    "`<form:error>` field name not in guard",
    "the error display references a field the guard doesn't have",
);

pub static GX1505: ErrorCode = ErrorCode::new(
    1505,
    E,
    "`<form>` with `form:action` has no `form:guard`",
    "an action form should have a guard for client-side validation",
);

pub static GX1506: ErrorCode = ErrorCode::new(
    1506,
    W,
    "Form field has no matching `<input>`",
    "a guard field exists but no form input provides it",
);

pub static GX1507: ErrorCode = ErrorCode::new(
    1507,
    W,
    "Form input name not in guard",
    "an input has a name that doesn't match any field in the form's guard",
);

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// GX1600–GX1699 — Reactivity (signal/derive/effect/watch)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub static GX1600: ErrorCode = ErrorCode::new(
    1600,
    E,
    "`signal` declared outside component or client block",
    "signals must be inside `client { }`, `out ui`, or `store { }`",
);

pub static GX1601: ErrorCode = ErrorCode::new(
    1601,
    E,
    "`derive` references no reactive sources",
    "a derive expression doesn't read any signals — use `let` instead",
);

pub static GX1602: ErrorCode = ErrorCode::new(
    1602,
    E,
    "`effect` has no reactive dependencies",
    "an effect body doesn't read any signals — it will never re-run",
);

pub static GX1603: ErrorCode = ErrorCode::new(
    1603,
    W,
    "`watch` expression is not reactive",
    "a watch observes an expression that contains no signals and will never trigger",
);

pub static GX1604: ErrorCode = ErrorCode::new(
    1604,
    E,
    "Circular reactive dependency",
    "two or more derives or effects form a cycle",
);

pub static GX1605: ErrorCode = ErrorCode::new(
    1605,
    E,
    "Signal mutated inside `derive`",
    "a derive body must be pure — it cannot write to signals",
);

pub static GX1606: ErrorCode = ErrorCode::new(
    1606,
    W,
    "Signal declared but never read",
    "a signal is created but no template, derive, or effect reads it",
);

pub static GX1607: ErrorCode = ErrorCode::new(
    1607,
    W,
    "Signal declared but never written",
    "a signal is created but never updated — consider using `let`",
);

pub static GX1608: ErrorCode = ErrorCode::new(
    1608,
    E,
    "`effect` return value is not a function",
    "if an effect returns something (for cleanup), it must be a function",
);

pub static GX1609: ErrorCode = ErrorCode::new(
    1609,
    W,
    "Effect performs expensive operation without batching",
    "an effect modifies multiple signals — consider wrapping in `batch()`",
);

pub static GX1610: ErrorCode = ErrorCode::new(
    1610,
    E,
    "`watch` body mutates the watched signal",
    "a watch handler writes to the same signal it observes, causing an infinite loop",
);

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// GX1700–GX1799 — Lint warnings
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub static GX1700: ErrorCode = ErrorCode::new(
    1700,
    W,
    "Unused variable",
    "a variable is declared but never referenced",
);

pub static GX1701: ErrorCode = ErrorCode::new(
    1701,
    W,
    "Unused guard",
    "a guard is declared but never used as a type or validator",
);

pub static GX1702: ErrorCode = ErrorCode::new(
    1702,
    W,
    "Unused function",
    "a function is declared but never called",
);

pub static GX1703: ErrorCode = ErrorCode::new(
    1703,
    W,
    "Unused store",
    "a store is declared but never imported or referenced",
);

pub static GX1704: ErrorCode = ErrorCode::new(
    1704,
    W,
    "`console.log` in production code",
    "consider removing debug logging",
);

pub static GX1705: ErrorCode =
    ErrorCode::new(1705, W, "Empty block `{}`", "a block has no statements");

pub static GX1706: ErrorCode = ErrorCode::new(
    1706,
    W,
    "Identical branches in `when`/`if`",
    "the then and else branches produce the same code",
);

pub static GX1707: ErrorCode = ErrorCode::new(
    1707,
    W,
    "Unnecessary `else` after `return`",
    "an `else` block after a `return` is redundant",
);

pub static GX1708: ErrorCode = ErrorCode::new(
    1708,
    W,
    "Missing `alt` attribute on `<img>`",
    "images should have alternative text for accessibility",
);

pub static GX1709: ErrorCode = ErrorCode::new(
    1709,
    W,
    "Missing `<label>` for form input",
    "form inputs should be associated with labels",
);

pub static GX1710: ErrorCode = ErrorCode::new(
    1710,
    W,
    "Hardcoded color value",
    "consider using Tailwind classes instead of inline colors",
);

pub static GX1711: ErrorCode = ErrorCode::new(
    1711,
    W,
    "Magic number",
    "a numeric literal with unclear meaning — consider extracting to a named constant",
);

pub static GX1712: ErrorCode = ErrorCode::new(
    1712,
    W,
    "Function is too long (>50 lines)",
    "consider breaking large functions into smaller ones",
);

pub static GX1713: ErrorCode = ErrorCode::new(
    1713,
    W,
    "File has more than 300 lines",
    "consider splitting into separate files or components",
);

pub static GX1714: ErrorCode = ErrorCode::new(
    1714,
    W,
    "Deeply nested conditionals (>4 levels)",
    "consider flattening with early returns or extracting logic",
);

pub static GX1715: ErrorCode = ErrorCode::new(
    1715,
    W,
    "Prefer `let` over `mut` when value is never reassigned",
    "a `mut` binding is never modified after initialization",
);

pub static GX1716: ErrorCode = ErrorCode::new(
    1716,
    W,
    "Unreachable `each` — collection is always empty",
    "static analysis shows the iterable is always an empty array",
);

pub static GX1717: ErrorCode = ErrorCode::new(
    1717,
    W,
    "TODO/FIXME comment found",
    "a comment contains TODO, FIXME, HACK, or XXX",
);

pub static GX1718: ErrorCode = ErrorCode::new(
    1718,
    W,
    "Deprecated API",
    "a deprecated function or pattern is used",
);

pub static GX1719: ErrorCode = ErrorCode::new(
    1719,
    W,
    "No `error.gx` for route segment",
    "a route segment has no error boundary",
);

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// GX1800–GX1899 — Build / codegen
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub static GX1800: ErrorCode = ErrorCode::new(
    1800,
    E,
    "Generated Rust code failed to compile",
    "the codegen produced invalid Rust — this is a compiler bug, please report it",
);

pub static GX1801: ErrorCode = ErrorCode::new(
    1801,
    E,
    "Generated JS exceeds size limit (>50KB per page)",
    "check for large inline data or excessive components",
);

pub static GX1802: ErrorCode = ErrorCode::new(
    1802,
    E,
    "Cargo build failed",
    "the Rust compilation step failed — check the full error output",
);

pub static GX1803: ErrorCode = ErrorCode::new(
    1803,
    E,
    "Cannot write to output directory",
    "file system permission error during build",
);

pub static GX1804: ErrorCode = ErrorCode::new(
    1804,
    E,
    "`gale.toml` is invalid",
    "the project config file has syntax errors or invalid values",
);

pub static GX1805: ErrorCode = ErrorCode::new(
    1805,
    E,
    "Unsupported `gale.toml` version",
    "the config file uses a newer format than this compiler supports",
);

pub static GX1806: ErrorCode = ErrorCode::new(
    1806,
    W,
    "Build produced no routes",
    "no `page.gx` files were found in `app/` — the server will have nothing to serve",
);

pub static GX1807: ErrorCode = ErrorCode::new(
    1807,
    E,
    "Tailwind CSS compilation failed",
    "the Tailwind engine errored — check for invalid custom config in `gale.toml`",
);

pub static GX1808: ErrorCode = ErrorCode::new(
    1808,
    E,
    "Static asset not found",
    "a `head { css: [\"x\"] }` or template references a file that doesn't exist in `public/`",
);

pub static GX1809: ErrorCode = ErrorCode::new(
    1809,
    W,
    "Slow build",
    "a file took unusually long to compile (>10s)",
);

pub static GX1810: ErrorCode = ErrorCode::new(
    1810,
    E,
    "Codegen target directory collision",
    "two routes would generate the same output file",
);

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// GX1900–GX1999 — Runtime errors (dev mode only)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub static GX1900: ErrorCode = ErrorCode::new(
    1900,
    E,
    "Unhandled action error",
    "a server action threw an error that wasn't caught",
);

pub static GX1901: ErrorCode = ErrorCode::new(
    1901,
    E,
    "Action returned non-serializable value",
    "an action returned something that can't be sent to the client as JSON",
);

pub static GX1902: ErrorCode = ErrorCode::new(
    1902,
    E,
    "Hydration mismatch: server HTML differs from client",
    "usually caused by non-deterministic rendering",
);

pub static GX1903: ErrorCode = ErrorCode::new(
    1903,
    E,
    "Signal update during render",
    "a signal was written to while a template was rendering — causes infinite loops",
);

pub static GX1904: ErrorCode = ErrorCode::new(
    1904,
    E,
    "Effect threw an error",
    "an effect's body threw an exception",
);

pub static GX1905: ErrorCode = ErrorCode::new(
    1905,
    E,
    "Channel connection failed",
    "a WebSocket channel couldn't connect to the server",
);

pub static GX1906: ErrorCode = ErrorCode::new(
    1906,
    E,
    "Query fetch failed",
    "a query's HTTP request returned an error status",
);

pub static GX1907: ErrorCode = ErrorCode::new(
    1907,
    E,
    "Guard runtime validation failed",
    "a guard's runtime validation threw an unexpected error (not a validation failure)",
);

pub static GX1908: ErrorCode = ErrorCode::new(
    1908,
    W,
    "Slow action",
    "a server action took unusually long to complete (>5s)",
);

pub static GX1909: ErrorCode = ErrorCode::new(
    1909,
    W,
    "Memory usage high",
    "the Gale server process is using more memory than expected",
);

pub static GX1910: ErrorCode = ErrorCode::new(
    1910,
    E,
    "Maximum re-render depth exceeded",
    "a reactive chain caused more than 100 synchronous updates — likely a circular dependency",
);

pub static GX1911: ErrorCode = ErrorCode::new(
    1911,
    E,
    "Duplicate `key` in `each` block",
    "two items in a rendered list produced the same key",
);

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// GX2000–GX2099 — Package / dependency system
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

pub static GX2000: ErrorCode = ErrorCode::new(
    2000,
    E,
    "Package not found in registry",
    "`gale add x` can't find the package",
);

pub static GX2001: ErrorCode = ErrorCode::new(
    2001,
    E,
    "Package has no compatible version",
    "the requested version doesn't exist or conflicts with other dependencies",
);

pub static GX2002: ErrorCode = ErrorCode::new(
    2002,
    E,
    "Package requires newer Gale version",
    "the package needs a newer Gale version than currently running",
);

pub static GX2003: ErrorCode = ErrorCode::new(
    2003,
    E,
    "Package checksum mismatch",
    "downloaded package doesn't match its expected hash — possible tampering",
);

pub static GX2004: ErrorCode = ErrorCode::new(
    2004,
    E,
    "`gale.lock` is out of date — run `gale install`",
    "the lockfile doesn't match `gale.toml` dependencies",
);

pub static GX2005: ErrorCode = ErrorCode::new(
    2005,
    W,
    "Package has known vulnerability",
    "a dependency has a published security advisory",
);

pub static GX2006: ErrorCode = ErrorCode::new(
    2006,
    W,
    "Package is deprecated",
    "the package author has marked it as deprecated — use the suggested replacement",
);

pub static GX2007: ErrorCode = ErrorCode::new(
    2007,
    E,
    "Circular package dependency",
    "packages depend on each other in a cycle",
);

pub static GX2008: ErrorCode = ErrorCode::new(
    2008,
    E,
    "Package contains invalid `.gx` files",
    "a downloaded package has syntax errors",
);

pub static GX2009: ErrorCode = ErrorCode::new(
    2009,
    W,
    "Unused package",
    "a package is in `gale.toml` but never imported",
);

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Registry helpers
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// All defined error codes, for iteration and lookup.
pub static ALL_CODES: &[&ErrorCode] = &[
    // Lexer (20)
    &GX0001, &GX0002, &GX0003, &GX0004, &GX0005, &GX0006, &GX0007, &GX0008, &GX0009, &GX0010,
    &GX0011, &GX0012, &GX0013, &GX0014, &GX0015, &GX0016, &GX0017, &GX0018, &GX0019, &GX0020,
    // Parser (60)
    &GX0100, &GX0101, &GX0102, &GX0103, &GX0104, &GX0105, &GX0106, &GX0107, &GX0108, &GX0109,
    &GX0110, &GX0111, &GX0112, &GX0113, &GX0114, &GX0115, &GX0116, &GX0117, &GX0118, &GX0119,
    &GX0120, &GX0121, &GX0122, &GX0123, &GX0124, &GX0125, &GX0126, &GX0127, &GX0128, &GX0129,
    &GX0130, &GX0131, &GX0132, &GX0133, &GX0134, &GX0135, &GX0136, &GX0137, &GX0138, &GX0139,
    &GX0140, &GX0141, &GX0142, &GX0143, &GX0144, &GX0145, &GX0146, &GX0147, &GX0148, &GX0149,
    &GX0150, &GX0151, &GX0152, &GX0153, &GX0154, &GX0155, &GX0156, &GX0157, &GX0158, &GX0159,
    // Types (41)
    &GX0300, &GX0301, &GX0302, &GX0303, &GX0304, &GX0305, &GX0306, &GX0307, &GX0308, &GX0309,
    &GX0310, &GX0311, &GX0312, &GX0313, &GX0314, &GX0315, &GX0316, &GX0317, &GX0318, &GX0319,
    &GX0320, &GX0321, &GX0322, &GX0323, &GX0324, &GX0325, &GX0326, &GX0327, &GX0328, &GX0329,
    &GX0330, &GX0331, &GX0332, &GX0333, &GX0334, &GX0335, &GX0336, &GX0337, &GX0338, &GX0339,
    &GX0340, // Boundary (24)
    &GX0500, &GX0501, &GX0502, &GX0503, &GX0504, &GX0505, &GX0506, &GX0507, &GX0508, &GX0509,
    &GX0510, &GX0511, &GX0512, &GX0513, &GX0514, &GX0515, &GX0516, &GX0517, &GX0518, &GX0519,
    &GX0520, &GX0521, &GX0522, &GX0523, // Guard (33)
    &GX0600, &GX0601, &GX0602, &GX0603, &GX0604, &GX0605, &GX0606, &GX0607, &GX0608, &GX0609,
    &GX0610, &GX0611, &GX0612, &GX0613, &GX0614, &GX0615, &GX0616, &GX0617, &GX0618, &GX0619,
    &GX0620, &GX0621, &GX0622, &GX0623, &GX0624, &GX0625, &GX0626, &GX0627, &GX0628, &GX0629,
    &GX0630, &GX0631, &GX0632, // Template (25)
    &GX0700, &GX0701, &GX0702, &GX0703, &GX0704, &GX0705, &GX0706, &GX0707, &GX0708, &GX0709,
    &GX0710, &GX0711, &GX0712, &GX0713, &GX0714, &GX0715, &GX0716, &GX0717, &GX0718, &GX0719,
    &GX0720, &GX0721, &GX0722, &GX0723, &GX0724, // Module (10)
    &GX0800, &GX0801, &GX0802, &GX0803, &GX0804, &GX0805, &GX0806, &GX0807, &GX0808, &GX0809,
    // Action/query/channel (14)
    &GX0900, &GX0901, &GX0902, &GX0903, &GX0904, &GX0905, &GX0906, &GX0907, &GX0908, &GX0909,
    &GX0910, &GX0911, &GX0912, &GX0913, // Store (7)
    &GX1000, &GX1001, &GX1002, &GX1003, &GX1004, &GX1005, &GX1006, // Env (9)
    &GX1100, &GX1101, &GX1102, &GX1103, &GX1104, &GX1105, &GX1106, &GX1107, &GX1108,
    // Routing (12)
    &GX1200, &GX1201, &GX1202, &GX1203, &GX1204, &GX1205, &GX1206, &GX1207, &GX1208, &GX1209,
    &GX1210, &GX1211, // Middleware (5)
    &GX1300, &GX1301, &GX1302, &GX1303, &GX1304, // Head/SEO (9)
    &GX1400, &GX1401, &GX1402, &GX1403, &GX1404, &GX1405, &GX1406, &GX1407, &GX1408,
    // Form (8)
    &GX1500, &GX1501, &GX1502, &GX1503, &GX1504, &GX1505, &GX1506, &GX1507,
    // Reactivity (11)
    &GX1600, &GX1601, &GX1602, &GX1603, &GX1604, &GX1605, &GX1606, &GX1607, &GX1608, &GX1609,
    &GX1610, // Lint (20)
    &GX1700, &GX1701, &GX1702, &GX1703, &GX1704, &GX1705, &GX1706, &GX1707, &GX1708, &GX1709,
    &GX1710, &GX1711, &GX1712, &GX1713, &GX1714, &GX1715, &GX1716, &GX1717, &GX1718, &GX1719,
    // Build (11)
    &GX1800, &GX1801, &GX1802, &GX1803, &GX1804, &GX1805, &GX1806, &GX1807, &GX1808, &GX1809,
    &GX1810, // Runtime (12)
    &GX1900, &GX1901, &GX1902, &GX1903, &GX1904, &GX1905, &GX1906, &GX1907, &GX1908, &GX1909,
    &GX1910, &GX1911, // Package (10)
    &GX2000, &GX2001, &GX2002, &GX2003, &GX2004, &GX2005, &GX2006, &GX2007, &GX2008, &GX2009,
];

/// Look up an error code by its numeric value.
pub fn lookup(code: u16) -> Option<&'static ErrorCode> {
    ALL_CODES.iter().find(|c| c.code == code).copied()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_codes_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for code in ALL_CODES {
            assert!(
                seen.insert(code.code),
                "duplicate error code: GX{:04}",
                code.code
            );
        }
    }

    #[test]
    fn all_codes_have_nonempty_message() {
        for code in ALL_CODES {
            assert!(
                !code.message.is_empty(),
                "GX{:04} has empty message",
                code.code
            );
        }
    }

    #[test]
    fn all_codes_have_nonempty_hint() {
        for code in ALL_CODES {
            assert!(!code.hint.is_empty(), "GX{:04} has empty hint", code.code);
        }
    }

    #[test]
    fn total_code_count() {
        assert!(
            ALL_CODES.len() >= 331,
            "expected at least 331 defined error codes, got {}",
            ALL_CODES.len()
        );
    }

    #[test]
    fn lookup_existing_code() {
        let lexer_code = lookup(1).unwrap();
        assert_eq!(lexer_code.code, 1);
        assert_eq!(lexer_code.message, "Unterminated string literal");
    }

    #[test]
    fn lookup_nonexistent_code() {
        // GX0042 is not defined (gap between lexer 0020 and parser 0100)
        assert!(lookup(42).is_none());
    }

    #[test]
    fn codes_in_correct_ranges() {
        for code in ALL_CODES {
            let sub = code.subsystem();
            assert_ne!(sub, "unknown", "GX{:04} has unknown subsystem", code.code);
        }
    }
}
