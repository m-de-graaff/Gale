# E-commerce

A multi-page e-commerce storefront demonstrating shared types, actions, and multi-route layouts.

## Features

- Multi-page application (store, product detail, cart)
- Shared enum types across server and client
- Server actions for cart management
- Layout with persistent navigation
- SSR for all pages

## Build

```bash
gale build --app-dir examples/ecommerce/app --output-dir examples/ecommerce/gale_build --name ecommerce
```

## Project Structure

```
ecommerce/
  galex.toml
  app/
    layout.gx          # Store layout with nav
    page.gx             # Product listing (route: /)
    product/
      page.gx           # Product detail (route: /product)
    cart/
      page.gx           # Shopping cart (route: /cart)
```
