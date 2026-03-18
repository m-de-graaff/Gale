//! Client-side router code generation.
//!
//! Generates a route manifest and client-side navigation script that:
//! - Intercepts internal `<a>` link clicks
//! - Fetches page data via `/__gx/pages/{name}.json`
//! - Swaps page content without full reload
//! - Handles back/forward navigation via `popstate`
//! - Prefetches on hover (when `data-prefetch` is present)
//! - Preserves layouts (only swaps the changed `data-gale-slot` region)

use crate::codegen::js_emitter::JsEmitter;

// ── Public entry point ─────────────────────────────────────────────────

/// Route entry for the manifest.
#[derive(Debug, Clone)]
pub struct RouteEntry {
    /// URL path pattern (e.g. `/`, `/about`, `/user/:id`).
    pub path: String,
    /// Module name for the page JS file (e.g. `home_page`, `about`).
    pub page: String,
    /// Whether the route has dynamic params.
    pub params: Vec<String>,
}

/// Generate the `public/_gale/router.js` client-side navigation module.
///
/// This is a self-contained ES module that:
/// 1. Builds a route table from the manifest
/// 2. Intercepts `<a>` clicks on internal links
/// 3. Fetches page data + HTML from the JSON endpoint
/// 4. Swaps `[data-gale-slot]` content
/// 5. Loads and executes the new page's hydration script
/// 6. Handles `popstate` for back/forward navigation
/// 7. Prefetches on hover for `[data-prefetch]` links
pub fn emit_router_js(routes: &[RouteEntry]) -> String {
    let mut e = JsEmitter::new();
    e.emit_file_header("GaleX client-side router.");
    e.newline();

    e.emit_import(&["navigate"], "/_gale/runtime.js");
    e.newline();

    // ── Route manifest ─────────────────────────────────────────
    e.writeln("const routes = [");
    e.indent();
    for route in routes {
        let params_js = if route.params.is_empty() {
            String::new()
        } else {
            let p: Vec<String> = route.params.iter().map(|p| format!("'{p}'")).collect();
            format!(", params: [{}]", p.join(", "))
        };
        e.writeln(&format!(
            "{{ path: '{}', page: '{}'{params_js} }},",
            route.path, route.page
        ));
    }
    e.dedent();
    e.writeln("];");
    e.newline();

    // ── Route matching ─────────────────────────────────────────
    e.emit_comment("Match a URL path to a route definition.");
    e.emit_fn("matchRoute", &["pathname"], |e| {
        e.emit_for_of("route", "routes", |e| {
            e.writeln("const parts = route.path.split('/').filter(Boolean);");
            e.writeln("const urlParts = pathname.split('/').filter(Boolean);");
            e.emit_if("parts.length !== urlParts.length", |e| {
                e.writeln("continue;");
            });
            e.writeln("let matched = true;");
            e.writeln("const matchedParams = {};");
            e.block("for (let i = 0; i < parts.length; i++)", |e| {
                e.emit_if_else(
                    "parts[i].startsWith(':')",
                    |e| {
                        e.writeln("matchedParams[parts[i].slice(1)] = urlParts[i];");
                    },
                    |e| {
                        e.emit_if("parts[i] !== urlParts[i]", |e| {
                            e.writeln("matched = false;");
                            e.writeln("break;");
                        });
                    },
                );
            });
            e.emit_if("matched", |e| {
                e.writeln("return { ...route, matchedParams };");
            });
        });
        e.writeln("return null;");
    });
    e.newline();

    // ── Page loading ───────────────────────────────────────────
    e.writeln("let _currentPage = null;");
    e.writeln("let _loadedScripts = new Set();");
    e.newline();

    e.emit_comment("Fetch page data and swap content.");
    e.block("async function loadPage(route)", |e| {
        e.writeln("const params = route.matchedParams || {};");
        e.writeln("const qs = new URLSearchParams(params).toString();");
        e.writeln(&format!(
            "const url = `/__gx/pages/${{route.page}}.json${{qs ? '?' + qs : ''}}`;",
        ));
        e.block("try", |e| {
            e.writeln("const res = await fetch(url);");
            e.emit_if("!res.ok", |e| {
                e.writeln("return;");
            });
            e.writeln("const { html } = await res.json();");
            e.newline();

            e.emit_comment("Swap the slot content.");
            e.writeln("const slot = document.querySelector('[data-gale-slot]');");
            e.emit_if("slot", |e| {
                e.writeln("slot.innerHTML = html;");
            });
            e.newline();

            e.emit_comment("Load the page's hydration script (once).");
            e.emit_if("!_loadedScripts.has(route.page)", |e| {
                e.block("try", |e| {
                    e.writeln("await import(`/_gale/pages/${route.page}.js`);");
                    e.writeln("_loadedScripts.add(route.page);");
                });
                e.writeln("catch (_) {}");
            });
            e.writeln("_currentPage = route.page;");
        });
        e.writeln("catch (_) {}");
    });
    e.newline();

    // ── Link interception ──────────────────────────────────────
    e.emit_comment("Intercept internal <a> clicks for client-side navigation.");
    e.block("document.addEventListener('click', function(e)", |e| {
        e.writeln("const a = e.target.closest('a[href]');");
        e.emit_if("!a", |e| {
            e.writeln("return;");
        });
        e.emit_comment("Skip external links, downloads, and modified clicks.");
        e.emit_if(
            "a.origin !== location.origin || a.hasAttribute('download') || e.ctrlKey || e.metaKey || e.shiftKey",
            |e| {
                e.writeln("return;");
            },
        );
        e.writeln("const route = matchRoute(a.pathname);");
        e.emit_if("!route", |e| {
            e.writeln("return;");
        });
        e.writeln("e.preventDefault();");
        e.writeln("navigate(a.href);");
    });
    e.newline();

    // ── popstate handler ───────────────────────────────────────
    e.emit_comment("Handle back/forward navigation.");
    e.block("window.addEventListener('popstate', function()", |e| {
        e.writeln("const route = matchRoute(location.pathname);");
        e.emit_if("route", |e| {
            e.writeln("loadPage(route);");
        });
    });
    e.newline();

    // ── Prefetch on hover ──────────────────────────────────────
    e.emit_comment("Prefetch page data on hover for links with data-prefetch.");
    e.writeln("const _prefetched = new Set();");
    e.block(
        "document.addEventListener('mouseover', function(e)",
        |e| {
            e.writeln("const a = e.target.closest('a[href][data-prefetch]');");
            e.emit_if("!a || _prefetched.has(a.pathname)", |e| {
                e.writeln("return;");
            });
            e.writeln("const route = matchRoute(a.pathname);");
            e.emit_if("route", |e| {
                e.writeln("_prefetched.add(a.pathname);");
                e.writeln("const qs = new URLSearchParams(route.matchedParams || {}).toString();");
                e.writeln("fetch(`/__gx/pages/${route.page}.json${qs ? '?' + qs : ''}`, { priority: 'low' }).catch(function() {});");
            });
        },
    );
    e.newline();

    e.emit_comment("Re-export navigate for programmatic use.");
    e.writeln("export { navigate, routes };");

    e.finish()
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_routes() -> Vec<RouteEntry> {
        vec![
            RouteEntry {
                path: "/".into(),
                page: "home_page".into(),
                params: vec![],
            },
            RouteEntry {
                path: "/about".into(),
                page: "about".into(),
                params: vec![],
            },
            RouteEntry {
                path: "/user/:id".into(),
                page: "user_by_id".into(),
                params: vec!["id".into()],
            },
        ]
    }

    #[test]
    fn router_has_route_manifest() {
        let out = emit_router_js(&make_routes());
        assert!(out.contains("path: '/'"), "root route: {out}");
        assert!(out.contains("page: 'home_page'"), "root page: {out}");
        assert!(out.contains("path: '/about'"), "about route: {out}");
        assert!(out.contains("path: '/user/:id'"), "param route: {out}");
        assert!(out.contains("params: ['id']"), "params list: {out}");
    }

    #[test]
    fn router_has_match_function() {
        let out = emit_router_js(&make_routes());
        assert!(
            out.contains("function matchRoute(pathname)"),
            "match fn: {out}"
        );
        assert!(out.contains("startsWith(':')"), "param detection: {out}");
    }

    #[test]
    fn router_has_load_page() {
        let out = emit_router_js(&make_routes());
        assert!(
            out.contains("async function loadPage(route)"),
            "load fn: {out}"
        );
        assert!(out.contains("/__gx/pages/"), "json endpoint: {out}");
        assert!(out.contains("data-gale-slot"), "slot swap: {out}");
    }

    #[test]
    fn router_intercepts_links() {
        let out = emit_router_js(&make_routes());
        assert!(out.contains("closest('a[href]')"), "link detection: {out}");
        assert!(out.contains("e.preventDefault()"), "prevent default: {out}");
        assert!(out.contains("navigate(a.href)"), "navigate call: {out}");
    }

    #[test]
    fn router_skips_external_and_modified() {
        let out = emit_router_js(&make_routes());
        assert!(
            out.contains("a.origin !== location.origin"),
            "external: {out}"
        );
        assert!(out.contains("e.ctrlKey"), "ctrl+click: {out}");
        assert!(out.contains("e.metaKey"), "cmd+click: {out}");
        assert!(out.contains("download"), "download: {out}");
    }

    #[test]
    fn router_handles_popstate() {
        let out = emit_router_js(&make_routes());
        assert!(out.contains("'popstate'"), "popstate: {out}");
        assert!(out.contains("loadPage(route)"), "load on back: {out}");
    }

    #[test]
    fn router_prefetches_on_hover() {
        let out = emit_router_js(&make_routes());
        assert!(out.contains("'mouseover'"), "hover event: {out}");
        assert!(out.contains("data-prefetch"), "prefetch attr: {out}");
        assert!(out.contains("priority: 'low'"), "low priority: {out}");
    }

    #[test]
    fn router_imports_navigate() {
        let out = emit_router_js(&make_routes());
        assert!(out.contains("import { navigate } from '/_gale/runtime.js'"));
        assert!(out.contains("export { navigate, routes }"));
    }

    #[test]
    fn router_loads_page_scripts() {
        let out = emit_router_js(&make_routes());
        assert!(
            out.contains("import(`/_gale/pages/"),
            "dynamic import: {out}"
        );
        assert!(out.contains("_loadedScripts"), "script cache: {out}");
    }

    #[test]
    fn router_empty_routes() {
        let out = emit_router_js(&[]);
        assert!(out.contains("const routes = ["));
        assert!(out.contains("];"));
        assert!(out.contains("function matchRoute"));
    }
}
