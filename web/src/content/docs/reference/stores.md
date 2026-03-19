# Stores

Reactive state containers shared across components.

```text
store CartStore {
  signal items: CartItem[] = []
  derive total = items.reduce(fn(sum, item) { sum + item.price }, 0)
  derive count = items.length

  fn addItem(item: CartItem) {
    items = [...items, item]
  }

  fn clear() {
    items = []
  }
}
```

Stores are generated as JavaScript singleton modules. Signals inside a store cannot be mutated from outside the store (GX1002).

## Compiler checks

| Code | Condition |
|------|-----------|
| GX1002 | External mutation of store signal |
| GX1003 | Duplicate store name |
| GX1004 | Store contains action (not allowed) |
| GX1005 | Store contains query (not allowed) |
| GX1006 | Store has no signals (warning) |
