# Dashboard

An admin dashboard demonstrating layouts with sidebar navigation and multi-page routing.

## Features

- Sidebar layout with persistent navigation
- Multiple pages (overview, users, settings)
- Table rendering for user data
- Form elements (inputs, selects, checkboxes)
- Reactive dashboard signals

## Build

```bash
gale build --app-dir examples/dashboard/app --output-dir examples/dashboard/gale_build --name dashboard
```

## Project Structure

```
dashboard/
  galex.toml
  app/
    layout.gx           # Dashboard layout with sidebar
    page.gx              # Overview page (route: /)
    users/
      page.gx            # User management (route: /users)
    settings/
      page.gx            # Settings (route: /settings)
```
