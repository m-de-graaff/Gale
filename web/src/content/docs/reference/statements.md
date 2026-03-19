# Statements

## Variable declarations

| Statement | Example | Notes |
|-----------|---------|-------|
| `let` | `let x = 42` | Immutable binding |
| `mut` | `mut x = 42` | Mutable binding |
| `signal` | `signal count = 0` | Reactive state (GX0507 if in server block) |
| `derive` | `derive doubled = count * 2` | Computed from signals (no mutations, GX1605) |
| `frozen` | `frozen cfg = load()` | Deep immutable binding |
| `ref` | `ref el: HTMLInputElement` | DOM element reference (GX0510 if in server block) |

## Control flow

### If / else

```text
if x > 0 {
  doSomething()
} else {
  doOther()
}
```

Condition must be `bool`.

### For loop

```text
for item, index in list {
  process(item)
}
```

Iterable must be `Array`. Index is optional.

### Return

```text
return value
```

Return type is constrained against the enclosing function's return type.

## Reactivity

### Effect

```text
effect {
  console.log("count is", count)
  return fn() {
    // cleanup
  }
}
```

Side effect with optional cleanup function.

### Watch

```text
watch count as (next, prev) {
  console.log("changed from", prev, "to", next)
}
```

Must watch a reactive source (`Signal` or `Derived`). The `next` and `prev` parameters receive the unwrapped inner type.

## Functions

```text
fn formatDate(date: string) -> string {
  // ...
}

async fn fetchData(url: string) -> Response {
  // ...
}
```

Functions support the `async` modifier. Parameters can have default values.

## Tests

```text
test "user creation" {
  let user = createUser({ name: "Alice" })
  assert user.name == "Alice"
}
```

> **Note:** Test discovery works but the test runner does not yet compile test bodies to Rust. Tests are discovered and parsed but always pass vacuously.
