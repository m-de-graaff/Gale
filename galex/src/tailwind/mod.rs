//! Tailwind CSS integration.
//!
//! Scans `.gx` source files for CSS class usage, generates a Tailwind
//! configuration, and shells out to the Tailwind CLI to produce an
//! optimized, production-ready CSS file.
//!
//! # Pipeline
//!
//! 1. **Extract** — Walk AST templates to collect class names from
//!    `class="..."` attributes, `class:name` directives, and
//!    `transition:type` directives.
//! 2. **Configure** — Load `galex.toml` `[tailwind]` section, generate
//!    a `tailwind.config.js` for the CLI.
//! 3. **Generate** — Run `npx tailwindcss` with the config and input CSS,
//!    producing `public/_gale/styles.css`.

pub mod config;
pub mod extract;
pub mod generate;
