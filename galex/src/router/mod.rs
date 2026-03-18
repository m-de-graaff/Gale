//! File-based route discovery.
//!
//! Walks an `app/` directory and maps its structure to URL routes
//! following the Next.js App Router convention:
//!
//! | File/Directory | URL |
//! |---|---|
//! | `app/page.gx` | `/` |
//! | `app/about/page.gx` | `/about` |
//! | `app/blog/[slug]/page.gx` | `/blog/:slug` |
//! | `app/users/[...rest]/page.gx` | `/users/*` |
//! | `app/(group)/page.gx` | `/` (no URL segment) |
//! | `app/api/users.gx` | `/api/users` |
//!
//! Special files at each directory level:
//! - `page.gx` — page component (creates a route)
//! - `layout.gx` — layout wrapping child pages
//! - `error.gx` — error boundary
//! - `loading.gx` — loading skeleton
//! - `guard.gx` — route guard (auth, etc.)
//! - `middleware.gx` — middleware for this segment

pub mod discovery;
pub mod validation;

use std::path::PathBuf;

/// A node in the route tree, corresponding to a directory in `app/`.
#[derive(Debug, Clone)]
pub struct RouteNode {
    /// The directory segment name (e.g. `"about"`, `"[slug]"`, `"(admin)"`).
    pub segment: String,
    /// The resolved URL segment (e.g. `"about"`, `":slug"`, `""` for groups).
    pub url_segment: String,
    /// `page.gx` if present.
    pub page: Option<PathBuf>,
    /// `layout.gx` if present.
    pub layout: Option<PathBuf>,
    /// `error.gx` if present.
    pub error: Option<PathBuf>,
    /// `loading.gx` if present.
    pub loading: Option<PathBuf>,
    /// `guard.gx` if present.
    pub guard: Option<PathBuf>,
    /// `middleware.gx` if present.
    pub middleware: Option<PathBuf>,
    /// Child route nodes (subdirectories).
    pub children: Vec<RouteNode>,
}

/// A fully resolved route with accumulated layouts, guards, and middleware.
#[derive(Debug, Clone)]
pub struct DiscoveredRoute {
    /// Full URL path (e.g. `/blog/:slug`).
    pub url_path: String,
    /// Path to the `page.gx` file.
    pub page_file: PathBuf,
    /// Module name for codegen (e.g. `blog_slug`).
    pub module_name: String,
    /// Layout files from root to leaf.
    pub layouts: Vec<PathBuf>,
    /// Guard files from root to leaf.
    pub guards: Vec<PathBuf>,
    /// Middleware files from root to leaf.
    pub middleware: Vec<PathBuf>,
    /// Dynamic parameter names.
    pub params: Vec<String>,
    /// Whether this is a catch-all route (`[...rest]`).
    pub is_catch_all: bool,
}

/// Error found during route discovery.
#[derive(Debug, Clone)]
pub struct RouteError {
    pub message: String,
    pub path: Option<PathBuf>,
}

impl std::fmt::Display for RouteError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(path) = &self.path {
            write!(f, "{}: {}", path.display(), self.message)
        } else {
            write!(f, "{}", self.message)
        }
    }
}
