//! Bundle optimization — runtime feature tracking and tree-shaking.
//!
//! Tracks which runtime features are used across all pages and generates
//! a custom runtime bundle that only includes what's needed.

use crate::ast::*;

/// Set of runtime features used by the project.
///
/// Each page's usage is OR'd together to produce the project-wide set.
/// The runtime is then tree-shaken to only include used features.
#[derive(Debug, Clone, Default)]
pub struct RuntimeFeatures {
    pub signal: bool,
    pub derive: bool,
    pub effect: bool,
    pub watch: bool,
    pub bind: bool,
    pub hydrate: bool,
    pub query: bool,
    pub channel: bool,
    pub transition: bool,
    pub router: bool,
    pub list: bool,
    pub show: bool,
    pub action: bool,
}

impl RuntimeFeatures {
    /// Scan a component declaration and record which runtime features it uses.
    pub fn scan_component(&mut self, decl: &ComponentDecl) {
        for stmt in &decl.body.stmts {
            match stmt {
                Stmt::Signal { .. } => self.signal = true,
                Stmt::Derive { .. } => self.derive = true,
                Stmt::Effect { .. } => self.effect = true,
                Stmt::Watch { .. } => self.watch = true,
                Stmt::RefDecl { .. } => {}
                _ => {}
            }
        }
        self.scan_template(&decl.body.template);
    }

    /// Scan template nodes for directive usage.
    fn scan_template(&mut self, nodes: &[TemplateNode]) {
        for node in nodes {
            match node {
                TemplateNode::Element {
                    directives,
                    children,
                    ..
                } => {
                    for d in directives {
                        match d {
                            Directive::Bind { .. } => self.bind = true,
                            Directive::Transition { .. } => self.transition = true,
                            _ => {}
                        }
                    }
                    self.scan_template(children);
                }
                TemplateNode::When {
                    body, else_branch, ..
                } => {
                    self.show = true;
                    self.scan_template(body);
                    if let Some(WhenElse::Else(nodes)) = else_branch {
                        self.scan_template(nodes);
                    }
                }
                TemplateNode::Each { body, empty, .. } => {
                    self.list = true;
                    self.scan_template(body);
                    if let Some(nodes) = empty {
                        self.scan_template(nodes);
                    }
                }
                _ => {}
            }
        }
    }

    /// Mark that actions are used.
    pub fn mark_actions(&mut self) {
        self.action = true;
    }

    /// Mark that queries are used.
    pub fn mark_queries(&mut self) {
        self.query = true;
    }

    /// Mark that channels are used.
    pub fn mark_channels(&mut self) {
        self.channel = true;
    }

    /// Mark that the router is needed.
    pub fn mark_router(&mut self) {
        self.router = true;
    }

    /// Return a list of runtime export names that should be included.
    pub fn required_exports(&self) -> Vec<&'static str> {
        let mut exports = Vec::new();
        if self.signal {
            exports.push("signal");
        }
        if self.derive {
            exports.push("derive");
        }
        if self.effect {
            exports.push("effect");
        }
        if self.watch {
            exports.push("watch");
        }
        if self.bind {
            exports.push("bind");
        }
        if self.hydrate {
            exports.push("hydrate");
        }
        if self.query {
            exports.push("query");
        }
        if self.channel {
            exports.push("channel");
        }
        if self.transition {
            exports.push("transition");
        }
        if self.list {
            exports.push("list");
            exports.push("reconcileList");
        }
        if self.show {
            exports.push("show");
            exports.push("replaceRegion");
        }
        if self.action {
            exports.push("action");
            exports.push("__gx_fetch");
        }
        if self.router {
            exports.push("navigate");
        }
        // Always include these base utilities
        exports.push("_readData");
        exports.push("_readEnv");
        exports.push("batch");
        exports
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::span::Span;

    fn s() -> Span {
        Span::dummy()
    }

    #[test]
    fn empty_features() {
        let f = RuntimeFeatures::default();
        assert!(!f.signal);
        assert!(!f.query);
        let exports = f.required_exports();
        assert!(exports.contains(&"_readData"));
        assert!(!exports.contains(&"signal"));
    }

    #[test]
    fn scan_component_with_signal() {
        let mut f = RuntimeFeatures::default();
        let decl = ComponentDecl {
            name: "Test".into(),
            props: vec![],
            body: ComponentBody {
                stmts: vec![Stmt::Signal {
                    name: "count".into(),
                    ty_ann: None,
                    init: Expr::IntLit {
                        value: 0,
                        span: s(),
                    },
                    span: s(),
                }],
                template: vec![],
                head: None,
                span: s(),
            },
            span: s(),
        };
        f.scan_component(&decl);
        assert!(f.signal);
        assert!(f.required_exports().contains(&"signal"));
    }

    #[test]
    fn mark_helpers() {
        let mut f = RuntimeFeatures::default();
        f.mark_actions();
        f.mark_queries();
        f.mark_channels();
        f.mark_router();
        assert!(f.action);
        assert!(f.query);
        assert!(f.channel);
        assert!(f.router);
        let exports = f.required_exports();
        assert!(exports.contains(&"__gx_fetch"));
        assert!(exports.contains(&"query"));
        assert!(exports.contains(&"channel"));
        assert!(exports.contains(&"navigate"));
    }
}
