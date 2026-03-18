# Blog

A content-focused blog built with GaleX demonstrating SSR and nested routes.

## Features

- Server-side rendered content
- Nested file-based routing (`/posts/hello-world`)
- Layout with navigation
- Head metadata per page (title, description)

## Build

```bash
gale build --app-dir examples/blog/app --output-dir examples/blog/gale_build --name blog
```

## Project Structure

```
blog/
  galex.toml
  app/
    layout.gx                # Blog layout with nav
    page.gx                  # Blog index (route: /)
    posts/
      hello-world/
        page.gx              # Blog post (route: /posts/hello-world)
```
