# Todo App

A simple GaleX application demonstrating the core framework structure.

## Features

- File-based routing
- Layout composition with `slot`
- Head metadata (`title`, `description`)
- Reactive signals
- Server-side rendering

## Build

```bash
gale build --app-dir examples/todo/app --output-dir examples/todo/gale_build --name todo
```

## Project Structure

```
todo/
  galex.toml        # Project config
  app/
    layout.gx       # HTML shell layout
    page.gx          # Home page component
```
