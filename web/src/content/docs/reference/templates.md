# Templates & Directives

## Elements

Standard HTML elements with self-closing support:

```text
<div class="container">
  <h1>Title</h1>
  <img src="/logo.png" alt="Logo" />
</div>
```

The compiler warns about void elements with children (GX0708), inline styles (GX0720), and deep nesting beyond 10 levels (GX0722).

## Expression interpolation

```text
<p>{user.name}</p>
<p>{count * 2}</p>
```

Interpolated expressions must be renderable (`string`, `int`, `float`, or `bool`).

## Conditionals — `when`

```text
when loggedIn {
  <p>Welcome, {name}</p>
} else {
  <p>Please log in</p>
}
```

Condition must be `bool`. Supports `else when` chaining. Uses curly braces, not angle brackets.

## Lists — `each`

```text
each item, index in items {
  <li key={item.id}>{item.name}</li>
} empty {
  <p>No items found</p>
}
```

Iterable must be an `Array`. The linter warns if `key` is missing (GX0705).

## Async boundaries — `suspend`

```text
suspend {
  <AsyncComponent />
}
```

## Directives

| Directive | Example | Type checking |
|-----------|---------|--------------|
| `bind:field` | `<input bind:value={name} />` | Target must be `Signal` (GX0712); type must be compatible (GX0713) |
| `on:event` | `<button on:click={handler}>` | Handler must be function (GX0714); event type checked against DOM |
| `on:event.mod` | `<form on:submit.prevent={fn}>` | Modifiers: `prevent`, `stop`, `once`, `self` |
| `class:name` | `<div class:active={flag}>` | Condition must be `bool` (GX0716) |
| `ref:name` | `<input ref:inputEl />` | Target must be `DomRef` (GX0717); type must match element (GX0718) |
| `key` | `<li key={id}>` | Must be `string` or `int` |
| `transition:type` | `<div transition:fade={cfg}>` | Config expression is type-inferred |
| `form:action` | `<form form:action={submit}>` | Must be function/action |
| `form:guard` | `<form form:guard={Login}>` | Must be guard type |
| `form:error` | `<form:error field="email" />` | Displays validation error |
| `into:slot` | `<div into:sidebar>` | Targets named slot |
| `prefetch` | `<a prefetch="hover">` | Link prefetching |

## DOM type checking

The compiler knows 50+ DOM event types (MouseEvent, KeyboardEvent, InputEvent, etc.) and 16 specific element types. Event handlers receive correctly-typed event parameters.
