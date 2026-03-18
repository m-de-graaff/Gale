//! Route validation (GX1200–GX1211).
//!
//! Validates discovered routes for structural issues:
//! - GX1200: No page.gx found in route directory
//! - GX1201: Conflicting routes
//! - GX1202: Dynamic segment conflicts with static segment
//! - GX1203: Multiple catch-all routes at same level
//! - GX1204: guard.gx without page.gx
//! - GX1205: layout.gx must export Layout
//! - GX1206: page.gx must export Page or api
//! - GX1207: error.gx must export ErrorPage
//! - GX1208: loading.gx must export Loading
//! - GX1209: Empty route directory (warning)
//! - GX1210: middleware.gx must contain handle fn
//! - GX1211: Dynamic route param has no type hint

use super::{DiscoveredRoute, RouteNode};
use crate::errors::{codes, Diagnostic};
use crate::span::Span;
use std::collections::HashMap;

/// Validate discovered routes for conflicts and structural issues.
pub fn validate_routes(routes: &[DiscoveredRoute]) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();

    // GX1201: Check for conflicting routes
    let mut url_patterns: HashMap<String, Vec<&DiscoveredRoute>> = HashMap::new();
    for route in routes {
        url_patterns
            .entry(route.url_path.clone())
            .or_default()
            .push(route);
    }
    for (pattern, matching_routes) in &url_patterns {
        if matching_routes.len() > 1 {
            for route in matching_routes.iter().skip(1) {
                diagnostics.push(
                    Diagnostic::with_message(
                        &codes::GX1201,
                        format!(
                            "Conflicting routes: `{}` and `{}` both resolve to `{}`",
                            matching_routes[0].page_file.display(),
                            route.page_file.display(),
                            pattern,
                        ),
                        Span::dummy(),
                    )
                    .with_hint("rename or remove one of the conflicting route directories"),
                );
            }
        }
    }

    diagnostics
}

/// Validate a route tree node for structural issues.
///
/// This is called during route tree construction, before flattening.
pub fn validate_route_tree(node: &RouteNode) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    validate_route_node(node, &mut diagnostics);
    diagnostics
}

/// Recursively validate a route tree node.
fn validate_route_node(node: &RouteNode, diagnostics: &mut Vec<Diagnostic>) {
    // GX1204: guard.gx without page.gx in same directory or children
    if node.guard.is_some() && node.page.is_none() && !has_page_in_children(node) {
        if let Some(ref guard_path) = node.guard {
            diagnostics.push(
                Diagnostic::with_message(
                    &codes::GX1204,
                    format!(
                        "`guard.gx` at `{}` has no `page.gx` in same directory or children",
                        guard_path.display()
                    ),
                    Span::dummy(),
                )
                .with_hint("a guard file must protect at least one page"),
            );
        }
    }

    // GX1209: Empty route directory (warning)
    if node.page.is_none()
        && node.layout.is_none()
        && node.guard.is_none()
        && node.middleware.is_none()
        && node.error.is_none()
        && node.loading.is_none()
        && node.children.is_empty()
        && !node.segment.is_empty()
    {
        diagnostics.push(
            Diagnostic::with_message(
                &codes::GX1209,
                format!("Empty route directory `{}`", node.segment),
                Span::dummy(),
            )
            .with_hint("add a `page.gx` or remove this directory"),
        );
    }

    // GX1202: Dynamic segment conflicts with static segment at same level
    check_segment_conflicts(&node.children, diagnostics);

    // GX1203: Multiple catch-all routes at same level
    check_multiple_catch_alls(&node.children, diagnostics);

    // Recurse into children
    for child in &node.children {
        validate_route_node(child, diagnostics);
    }
}

/// Check for dynamic/static segment conflicts at the same level (GX1202).
fn check_segment_conflicts(children: &[RouteNode], diagnostics: &mut Vec<Diagnostic>) {
    let mut static_segments: Vec<&str> = Vec::new();
    let mut dynamic_segments: Vec<&str> = Vec::new();

    for child in children {
        if child.segment.starts_with('[') && !child.segment.starts_with("[...") {
            dynamic_segments.push(&child.segment);
        } else if !child.segment.starts_with('(') && !child.segment.starts_with('[') {
            static_segments.push(&child.segment);
        }
    }

    // If both dynamic and static segments exist at the same level with
    // a matching static name, that's ambiguous
    if !dynamic_segments.is_empty() && !static_segments.is_empty() {
        for dynamic in &dynamic_segments {
            for static_seg in &static_segments {
                diagnostics.push(
                    Diagnostic::with_message(
                        &codes::GX1202,
                        format!(
                            "Dynamic segment `{}` conflicts with static segment `{}` at the same level",
                            dynamic, static_seg
                        ),
                        Span::dummy(),
                    )
                    .with_hint("the router can't distinguish between dynamic and static segments at the same level"),
                );
            }
        }
    }
}

/// Check for multiple catch-all routes at the same level (GX1203).
fn check_multiple_catch_alls(children: &[RouteNode], diagnostics: &mut Vec<Diagnostic>) {
    let catch_alls: Vec<&RouteNode> = children
        .iter()
        .filter(|c| c.segment.starts_with("[..."))
        .collect();

    if catch_alls.len() > 1 {
        for ca in catch_alls.iter().skip(1) {
            diagnostics.push(
                Diagnostic::with_message(
                    &codes::GX1203,
                    format!(
                        "Multiple catch-all routes at the same level: `{}`",
                        ca.segment
                    ),
                    Span::dummy(),
                )
                .with_hint("only one catch-all segment `[...x]` is allowed per directory level"),
            );
        }
    }
}

/// Check whether any child node (recursively) has a page.
fn has_page_in_children(node: &RouteNode) -> bool {
    for child in &node.children {
        if child.page.is_some() {
            return true;
        }
        if has_page_in_children(child) {
            return true;
        }
    }
    false
}

/// Check that a page.gx directory has the required page.gx file (GX1200).
pub fn check_page_exists_in_dir(
    dir_name: &str,
    has_page: bool,
    has_other_files: bool,
) -> Option<Diagnostic> {
    if !has_page && has_other_files {
        Some(
            Diagnostic::with_message(
                &codes::GX1200,
                format!("No `page.gx` found in route directory `{}`", dir_name),
                Span::dummy(),
            )
            .with_hint("add a `page.gx` to define this route"),
        )
    } else {
        None
    }
}

/// Check that a layout.gx exports the correct component (GX1205).
pub fn check_layout_export(file_path: &str, has_layout_export: bool) -> Option<Diagnostic> {
    if !has_layout_export {
        Some(
            Diagnostic::with_message(
                &codes::GX1205,
                format!(
                    "`layout.gx` at `{}` must export `out ui Layout()`",
                    file_path
                ),
                Span::dummy(),
            )
            .with_hint("add `out ui Layout() { ... }` to your layout file"),
        )
    } else {
        None
    }
}

/// Check that a page.gx exports Page or api (GX1206).
pub fn check_page_export(
    file_path: &str,
    has_page_export: bool,
    has_api_export: bool,
) -> Option<Diagnostic> {
    if !has_page_export && !has_api_export {
        Some(
            Diagnostic::with_message(
                &codes::GX1206,
                format!(
                    "`page.gx` at `{}` must export `out ui Page()` or `out api {{ }}`",
                    file_path
                ),
                Span::dummy(),
            )
            .with_hint("add `out ui Page() { ... }` or `out api { ... }` to your page file"),
        )
    } else {
        None
    }
}

/// Check that an error.gx exports ErrorPage (GX1207).
pub fn check_error_export(file_path: &str, has_error_export: bool) -> Option<Diagnostic> {
    if !has_error_export {
        Some(
            Diagnostic::with_message(
                &codes::GX1207,
                format!(
                    "`error.gx` at `{}` must export `out ui ErrorPage(error, reset)`",
                    file_path
                ),
                Span::dummy(),
            )
            .with_hint("add `out ui ErrorPage(error, reset) { ... }` to your error boundary file"),
        )
    } else {
        None
    }
}

/// Check that a loading.gx exports Loading (GX1208).
pub fn check_loading_export(file_path: &str, has_loading_export: bool) -> Option<Diagnostic> {
    if !has_loading_export {
        Some(
            Diagnostic::with_message(
                &codes::GX1208,
                format!(
                    "`loading.gx` at `{}` must export `out ui Loading()`",
                    file_path
                ),
                Span::dummy(),
            )
            .with_hint("add `out ui Loading() { ... }` to your loading file"),
        )
    } else {
        None
    }
}

/// Check that a middleware.gx contains the handle function (GX1210).
pub fn check_middleware_handle(file_path: &str, has_handle_fn: bool) -> Option<Diagnostic> {
    if !has_handle_fn {
        Some(
            Diagnostic::with_message(
                &codes::GX1210,
                format!(
                    "`middleware.gx` at `{}` must contain `fn handle(req, next) -> Response`",
                    file_path
                ),
                Span::dummy(),
            )
            .with_hint("add a `fn handle(req: Request, next: Next) -> Response { ... }` function"),
        )
    } else {
        None
    }
}

/// Check for dynamic route parameters without type hints (GX1211).
pub fn check_dynamic_param_type(param_name: &str, has_type_hint: bool) -> Option<Diagnostic> {
    if !has_type_hint {
        Some(
            Diagnostic::with_message(
                &codes::GX1211,
                format!("Dynamic route parameter `{}` has no type hint", param_name),
                Span::dummy(),
            )
            .with_hint("consider adding a type: `[id: int]` or use a guard for validation"),
        )
    } else {
        None
    }
}
