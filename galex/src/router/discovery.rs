//! Filesystem walking and route tree construction.

use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use super::{DiscoveredRoute, RouteError, RouteNode};

/// Discover routes by walking the `app_dir` directory.
///
/// Returns a list of resolved routes and any validation errors.
pub fn discover_routes(app_dir: &Path) -> Result<Vec<DiscoveredRoute>, Vec<RouteError>> {
    if !app_dir.is_dir() {
        return Err(vec![RouteError {
            message: format!("app directory does not exist: {}", app_dir.display()),
            path: Some(app_dir.to_path_buf()),
        }]);
    }

    let root = build_route_tree(app_dir, app_dir);
    let mut routes = Vec::new();
    let mut errors = Vec::new();

    flatten_routes(&root, String::new(), &[], &[], &[], &mut routes);

    // Validate: check for conflicts
    check_conflicts(&routes, &mut errors);

    if errors.is_empty() {
        Ok(routes)
    } else {
        Err(errors)
    }
}

/// Build the route tree by scanning directories.
fn build_route_tree(dir: &Path, app_root: &Path) -> RouteNode {
    let segment = if dir == app_root {
        String::new() // root has no segment
    } else {
        dir.file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string()
    };

    let url_segment = dir_name_to_url_segment(&segment);

    // Check for special files
    let page = check_file(dir, "page.gx");
    let layout = check_file(dir, "layout.gx");
    let error = check_file(dir, "error.gx");
    let loading = check_file(dir, "loading.gx");
    let guard = check_file(dir, "guard.gx");
    let middleware = check_file(dir, "middleware.gx");

    // Recurse into child directories
    let mut children = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        let mut dirs: Vec<PathBuf> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
            .map(|e| e.path())
            .collect();
        dirs.sort(); // deterministic order
        for child_dir in dirs {
            children.push(build_route_tree(&child_dir, app_root));
        }
    }

    RouteNode {
        segment,
        url_segment,
        page,
        layout,
        error,
        loading,
        guard,
        middleware,
        children,
    }
}

/// Flatten the route tree into a list of discovered routes.
fn flatten_routes(
    node: &RouteNode,
    parent_path: String,
    parent_layouts: &[PathBuf],
    parent_guards: &[PathBuf],
    parent_middleware: &[PathBuf],
    routes: &mut Vec<DiscoveredRoute>,
) {
    // Accumulate layouts, guards, middleware from this level
    let mut layouts: Vec<PathBuf> = parent_layouts.to_vec();
    if let Some(ref l) = node.layout {
        layouts.push(l.clone());
    }
    let mut guards: Vec<PathBuf> = parent_guards.to_vec();
    if let Some(ref g) = node.guard {
        guards.push(g.clone());
    }
    let mut middleware: Vec<PathBuf> = parent_middleware.to_vec();
    if let Some(ref m) = node.middleware {
        middleware.push(m.clone());
    }

    // Build URL path
    let url_path = if node.url_segment.is_empty() {
        parent_path.clone()
    } else if parent_path == "/" || parent_path.is_empty() {
        format!("/{}", node.url_segment)
    } else {
        format!("{}/{}", parent_path, node.url_segment)
    };

    // If this node has a page, create a route
    if let Some(ref page_file) = node.page {
        let final_path = if url_path.is_empty() {
            "/".to_string()
        } else {
            url_path.clone()
        };
        let params = extract_params(&final_path);
        let is_catch_all = final_path.contains('*');
        let module_name = path_to_module_name(&final_path);

        routes.push(DiscoveredRoute {
            url_path: final_path,
            page_file: page_file.clone(),
            module_name,
            layouts: layouts.clone(),
            guards: guards.clone(),
            middleware: middleware.clone(),
            params,
            is_catch_all,
        });
    }

    // Recurse into children
    let child_parent = if url_path.is_empty() {
        "/".to_string()
    } else {
        url_path
    };
    for child in &node.children {
        flatten_routes(
            child,
            child_parent.clone(),
            &layouts,
            &guards,
            &middleware,
            routes,
        );
    }
}

/// Convert a directory name to a URL segment.
///
/// - `about` → `about`
/// - `[slug]` → `:slug`
/// - `[...rest]` → `*`
/// - `(group)` → `` (empty — route groups don't add URL segments)
fn dir_name_to_url_segment(name: &str) -> String {
    if name.is_empty() {
        return String::new();
    }
    // Route group: (name) → no URL segment
    if name.starts_with('(') && name.ends_with(')') {
        return String::new();
    }
    // Catch-all: [...name]
    if name.starts_with("[...") && name.ends_with(']') {
        return "*".to_string();
    }
    // Dynamic param: [name]
    if name.starts_with('[') && name.ends_with(']') {
        let param = &name[1..name.len() - 1];
        return format!(":{param}");
    }
    // Static segment — kebab-case
    name.to_string()
}

/// Check if a special file exists in a directory.
fn check_file(dir: &Path, filename: &str) -> Option<PathBuf> {
    let path = dir.join(filename);
    if path.is_file() {
        Some(path)
    } else {
        None
    }
}

/// Extract dynamic parameter names from a URL path.
fn extract_params(path: &str) -> Vec<String> {
    path.split('/')
        .filter(|s| s.starts_with(':'))
        .map(|s| s[1..].to_string())
        .collect()
}

/// Convert a URL path to a Rust module name.
///
/// `/` → `home`, `/about` → `about`, `/blog/:slug` → `blog_slug`
fn path_to_module_name(path: &str) -> String {
    if path == "/" {
        return "home".to_string();
    }
    path.trim_start_matches('/')
        .replace('/', "_")
        .replace(':', "")
        .replace('*', "catch_all")
        .replace('-', "_")
}

/// Check for conflicting routes (e.g. static vs dynamic at same level).
fn check_conflicts(routes: &[DiscoveredRoute], errors: &mut Vec<RouteError>) {
    for i in 0..routes.len() {
        for j in (i + 1)..routes.len() {
            if routes[i].url_path == routes[j].url_path {
                errors.push(RouteError {
                    message: format!(
                        "conflicting routes: {} and {} both resolve to '{}'",
                        routes[i].page_file.display(),
                        routes[j].page_file.display(),
                        routes[i].url_path,
                    ),
                    path: Some(routes[j].page_file.clone()),
                });
            }
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Create a directory structure for testing.
    fn create_app_dir(structure: &[&str]) -> TempDir {
        let tmp = TempDir::new().unwrap();
        let app = tmp.path().join("app");
        fs::create_dir(&app).unwrap();
        for path in structure {
            let full = app.join(path);
            if let Some(parent) = full.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&full, "// placeholder").unwrap();
        }
        tmp
    }

    #[test]
    fn basic_routes() {
        let tmp = create_app_dir(&["page.gx", "about/page.gx"]);
        let routes = discover_routes(&tmp.path().join("app")).unwrap();
        assert_eq!(routes.len(), 2);
        assert!(routes.iter().any(|r| r.url_path == "/"), "root route");
        assert!(routes.iter().any(|r| r.url_path == "/about"), "about route");
    }

    #[test]
    fn dynamic_param() {
        let tmp = create_app_dir(&["blog/[slug]/page.gx"]);
        let routes = discover_routes(&tmp.path().join("app")).unwrap();
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].url_path, "/blog/:slug");
        assert_eq!(routes[0].params, vec!["slug"]);
    }

    #[test]
    fn catch_all() {
        let tmp = create_app_dir(&["docs/[...path]/page.gx"]);
        let routes = discover_routes(&tmp.path().join("app")).unwrap();
        assert_eq!(routes.len(), 1);
        assert!(routes[0].url_path.contains('*'));
        assert!(routes[0].is_catch_all);
    }

    #[test]
    fn route_group_no_url_segment() {
        let tmp = create_app_dir(&["(marketing)/about/page.gx"]);
        let routes = discover_routes(&tmp.path().join("app")).unwrap();
        assert_eq!(routes.len(), 1);
        // Route group doesn't add URL segment
        assert_eq!(routes[0].url_path, "/about");
    }

    #[test]
    fn layout_accumulation() {
        let tmp = create_app_dir(&[
            "layout.gx",
            "dashboard/layout.gx",
            "dashboard/settings/page.gx",
        ]);
        let routes = discover_routes(&tmp.path().join("app")).unwrap();
        assert_eq!(routes.len(), 1);
        assert_eq!(
            routes[0].layouts.len(),
            2,
            "should have root + dashboard layout"
        );
    }

    #[test]
    fn guard_accumulation() {
        let tmp = create_app_dir(&["dashboard/guard.gx", "dashboard/settings/page.gx"]);
        let routes = discover_routes(&tmp.path().join("app")).unwrap();
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].guards.len(), 1);
    }

    #[test]
    fn middleware_accumulation() {
        let tmp = create_app_dir(&["middleware.gx", "api/middleware.gx", "api/users/page.gx"]);
        let routes = discover_routes(&tmp.path().join("app")).unwrap();
        assert_eq!(routes.len(), 1);
        assert_eq!(
            routes[0].middleware.len(),
            2,
            "should have root + api middleware"
        );
    }

    #[test]
    fn module_name_derivation() {
        assert_eq!(path_to_module_name("/"), "home");
        assert_eq!(path_to_module_name("/about"), "about");
        assert_eq!(path_to_module_name("/blog/:slug"), "blog_slug");
        assert_eq!(path_to_module_name("/user-profile"), "user_profile");
    }

    #[test]
    fn nonexistent_dir_returns_error() {
        let result = discover_routes(Path::new("/nonexistent/app"));
        assert!(result.is_err());
    }

    #[test]
    fn conflicting_routes_detected() {
        // This is hard to create naturally since identical paths would need
        // the same directory. Test the check_conflicts function directly.
        let routes = vec![
            DiscoveredRoute {
                url_path: "/about".into(),
                page_file: "a/page.gx".into(),
                module_name: "about".into(),
                layouts: vec![],
                guards: vec![],
                middleware: vec![],
                params: vec![],
                is_catch_all: false,
            },
            DiscoveredRoute {
                url_path: "/about".into(),
                page_file: "b/page.gx".into(),
                module_name: "about".into(),
                layouts: vec![],
                guards: vec![],
                middleware: vec![],
                params: vec![],
                is_catch_all: false,
            },
        ];
        let mut errors = Vec::new();
        check_conflicts(&routes, &mut errors);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].message.contains("conflicting"));
    }

    #[test]
    fn dir_name_mapping() {
        assert_eq!(dir_name_to_url_segment("about"), "about");
        assert_eq!(dir_name_to_url_segment("[slug]"), ":slug");
        assert_eq!(dir_name_to_url_segment("[...rest]"), "*");
        assert_eq!(dir_name_to_url_segment("(admin)"), "");
        assert_eq!(dir_name_to_url_segment(""), "");
    }

    #[test]
    fn nested_dynamic_params() {
        let tmp = create_app_dir(&["users/[userId]/posts/[postId]/page.gx"]);
        let routes = discover_routes(&tmp.path().join("app")).unwrap();
        assert_eq!(routes.len(), 1);
        assert_eq!(routes[0].url_path, "/users/:userId/posts/:postId");
        assert_eq!(routes[0].params, vec!["userId", "postId"]);
    }
}
