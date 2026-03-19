# Components & Layouts

## Components — `out ui`

Every page or reusable component is declared with `out ui`:

```text
out ui HomePage(title: string) {
  head {
    title: "My App"
    description: "Built with GaleX"
  }

  <main>
    <h1>{title}</h1>
  </main>
}
```

The `head` block sets `<title>` and `<meta>` tags. The compiler warns if title (GX1403) or description (GX1404) are missing, and flags overly long values (GX1405/GX1406). 11 head properties are recognized.

## Layouts — `out layout`

Layouts wrap pages and must contain a `<slot/>` element:

```text
out layout RootLayout() {
  <html>
    <body>
      <nav><a href="/">Home</a></nav>
      <slot/>
    </body>
  </html>
}
```

The compiler validates that every layout includes `<slot/>`. Layouts nest automatically based on the filesystem — `app/layout.gx` wraps all pages, `app/blog/layout.gx` wraps blog pages.

## Named slots

```text
out layout DashboardLayout() {
  <div class="dashboard">
    <aside><slot name="sidebar" /></aside>
    <main><slot/></main>
  </div>
}
```

Pages target named slots with `into:`:

```text
<div into:sidebar>Sidebar content</div>
<p>Main content goes into the default slot</p>
```

## File-based routing

| File | Route |
|------|-------|
| `app/page.gx` | `/` |
| `app/about/page.gx` | `/about` |
| `app/blog/[slug]/page.gx` | `/blog/:slug` |
| `app/docs/[...rest]/page.gx` | `/docs/*` |
