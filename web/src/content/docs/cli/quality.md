# CLI — Quality Tools

## `gale lint`

Static analysis with lint rules.

```bash
gale lint
```

Selected rules:

| Code | Rule |
|------|------|
| GX1700 | Unused variable |
| GX1701 | Unused guard |
| GX1702 | Unused function |
| GX1703 | Unused store |
| GX1704 | `console.log` in production code |
| GX1705 | Empty block `{}` |
| GX1707 | Unnecessary `else` after `return` |
| GX1708 | Missing `alt` attribute on `<img>` |
| GX1709 | Missing `<label>` for form input |
| GX1712 | Function is too long (>50 lines) |
| GX1713 | File has more than 300 lines |
| GX1717 | TODO/FIXME comment found |

## `gale fmt`

> **Status: Disabled.** The formatter is implemented but intentionally disabled due to known bugs — it drops parenthesized expression grouping and strips all comments. Running `gale fmt` returns an error message.

## `gale test`

> **Status: Partial.** Test discovery works — the compiler finds and parses `test` blocks in `.gx` files. However, the test runner does not yet compile test bodies to Rust. All discovered tests pass vacuously.

```bash
gale test
gale test --filter "user"
```
