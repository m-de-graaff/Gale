# Type System

GaleX uses constraint-based type inference with Robinson unification.

## Primitive types

`string`, `int`, `float`, `bool`, `void`, `null`, `never`

## Compound types

| Type | Syntax | Example |
|------|--------|---------|
| Array | `T[]` | `string[]` |
| Optional | `T?` | `string?` |
| Union | `T \| U` | `string \| null` |
| Tuple | `(T, U)` | `(string, int)` |
| Object | `{ key: T }` | `{ name: string, age: int }` |
| Function | `fn(T) -> U` | `fn(string) -> bool` |
| String literal | `"value"` | `"primary" \| "ghost"` |

## Type inference

Types are inferred from:

- Literal values (`42` is `int`, `"hello"` is `string`)
- Operator usage (`a + b` where `a: int` infers `b: int`)
- Function calls (argument types constrain parameter types)
- Signal wrapping (`signal x = 0` creates `Signal<int>`)
- Guard field types (from annotation + validators)

## Subtyping rules

- `StringLiteral <: string`
- `IntLiteral <: int`
- `int <: float`
- `never <: T` (for any T)
- `null <: T?`
- `T <: T | U`
- Guard-to-Object structural subtyping (bidirectional)
- Width subtyping for objects (extra fields allowed)

## Enums and type aliases

```text
shared {
  enum Status { Active, Inactive, Pending }
  type UserId = string
}
```

Enums are string-like. `shared` makes them available to both server and client.
