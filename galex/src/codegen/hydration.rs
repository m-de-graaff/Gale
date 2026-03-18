//! Hydration marker system for SSR-rendered pages.
//!
//! Tracks interactive elements during server-side rendering and emits
//! minimal hydration data that a future client-side runtime can use to
//! attach event listeners, two-way bindings, and reactive state.
//!
//! Each interactive element gets a `data-gx-id="N"` attribute. At the
//! end of the page, a `<script type="gale-data">` block is emitted
//! containing the marker descriptors and serialized server data.
//!
//! Additionally, `when` and `each` template blocks get comment markers
//! (`<!--gx-when:N-->` / `<!--gx-each:N-->`) that the client-side
//! runtime uses to locate regions for reactive re-rendering.

use super::rust_emitter::RustEmitter;

/// Tracks hydration state during SSR template rendering.
pub struct HydrationCtx {
    next_id: u32,
    markers: Vec<HydrationMarker>,
    /// Server data keys to serialize into the hydration script.
    server_data_keys: Vec<String>,
}

/// A single hydration marker — one interactive element or template region.
#[derive(Debug, Clone)]
pub struct HydrationMarker {
    /// Unique marker ID.
    pub id: u32,
    /// What kind of interactivity this marker represents.
    pub kind: MarkerKind,
}

/// What kind of interactivity an element or region has.
#[derive(Debug, Clone)]
pub enum MarkerKind {
    /// `bind:field` — two-way binding on an input element.
    Bind { field: String },
    /// `on:event` — event handler attached to an element.
    Event {
        event: String,
        modifiers: Vec<String>,
    },
    /// `ref:name` — DOM element reference.
    Ref { name: String },
    /// `transition:kind` — CSS transition hooks.
    Transition { kind: String },
    /// `class:name` — reactive class toggle.
    ClassToggle { name: String },
    /// `when condition { ... }` — conditional template block.
    When,
    /// `each item in list { ... }` — reactive list block.
    Each,
    /// `{expression}` — reactive text interpolation.
    TextExpr,
}

impl HydrationCtx {
    /// Create a new empty hydration context.
    pub fn new() -> Self {
        Self {
            next_id: 0,
            markers: Vec::new(),
            server_data_keys: Vec::new(),
        }
    }

    /// Allocate the next hydration ID.
    fn alloc_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Allocate a new hydration ID and record a bind: marker.
    pub fn mark_bind(&mut self, field: &str) -> u32 {
        let id = self.alloc_id();
        self.markers.push(HydrationMarker {
            id,
            kind: MarkerKind::Bind {
                field: field.to_string(),
            },
        });
        id
    }

    /// Allocate a new hydration ID and record an on: event marker.
    pub fn mark_event(&mut self, event: &str, modifiers: &[smol_str::SmolStr]) -> u32 {
        let id = self.alloc_id();
        self.markers.push(HydrationMarker {
            id,
            kind: MarkerKind::Event {
                event: event.to_string(),
                modifiers: modifiers.iter().map(|m| m.to_string()).collect(),
            },
        });
        id
    }

    /// Allocate a new hydration ID and record a ref: marker.
    pub fn mark_ref(&mut self, name: &str) -> u32 {
        let id = self.alloc_id();
        self.markers.push(HydrationMarker {
            id,
            kind: MarkerKind::Ref {
                name: name.to_string(),
            },
        });
        id
    }

    /// Allocate a new hydration ID and record a transition: marker.
    pub fn mark_transition(&mut self, kind: &str) -> u32 {
        let id = self.alloc_id();
        self.markers.push(HydrationMarker {
            id,
            kind: MarkerKind::Transition {
                kind: kind.to_string(),
            },
        });
        id
    }

    /// Allocate a new hydration ID and record a class: toggle marker.
    pub fn mark_class_toggle(&mut self, name: &str) -> u32 {
        let id = self.alloc_id();
        self.markers.push(HydrationMarker {
            id,
            kind: MarkerKind::ClassToggle {
                name: name.to_string(),
            },
        });
        id
    }

    /// Allocate a new hydration ID and record a `when` block marker.
    ///
    /// The SSR emitter wraps the rendered block in comment markers:
    /// `<!--gx-when:N-->` ... `<!--/gx-when:N-->`
    pub fn mark_when(&mut self) -> u32 {
        let id = self.alloc_id();
        self.markers.push(HydrationMarker {
            id,
            kind: MarkerKind::When,
        });
        id
    }

    /// Allocate a new hydration ID and record an `each` block marker.
    ///
    /// The SSR emitter wraps the rendered block in comment markers:
    /// `<!--gx-each:N-->` ... `<!--/gx-each:N-->`
    pub fn mark_each(&mut self) -> u32 {
        let id = self.alloc_id();
        self.markers.push(HydrationMarker {
            id,
            kind: MarkerKind::Each,
        });
        id
    }

    /// Allocate a new hydration ID and record a text expression marker.
    pub fn mark_text_expr(&mut self) -> u32 {
        let id = self.alloc_id();
        self.markers.push(HydrationMarker {
            id,
            kind: MarkerKind::TextExpr,
        });
        id
    }

    /// Record a server data key to be serialized into the hydration script.
    pub fn add_server_data(&mut self, key: &str) {
        if !self.server_data_keys.contains(&key.to_string()) {
            self.server_data_keys.push(key.to_string());
        }
    }

    /// Whether any interactive elements were found.
    pub fn has_markers(&self) -> bool {
        !self.markers.is_empty()
    }

    /// Return a read-only view of the collected markers.
    ///
    /// Used by the JS emitter to generate client-side hydration code
    /// with the same ID assignments.
    pub fn markers(&self) -> &[HydrationMarker] {
        &self.markers
    }

    /// Return the server data keys.
    pub fn server_data_keys(&self) -> &[String] {
        &self.server_data_keys
    }

    /// Emit the hydration `<script>` block as Rust code that appends to `html`.
    ///
    /// Generates code like:
    /// ```ignore
    /// html.push_str("<script type=\"gale-data\">");
    /// html.push_str(&serde_json::json!({ "markers": [...], "data": {...} }).to_string());
    /// html.push_str("</script>");
    /// ```
    pub fn emit_script(&self, e: &mut RustEmitter) {
        if self.markers.is_empty() && self.server_data_keys.is_empty() {
            return;
        }

        e.writeln("// Hydration data for client-side pickup");
        e.writeln("html.push_str(\"<script type=\\\"gale-data\\\">\");");

        // Build the JSON object
        e.writeln("let mut gale_hydration = serde_json::json!({});");

        if !self.markers.is_empty() {
            e.writeln("let mut markers = serde_json::Map::new();");
            for marker in &self.markers {
                let (kind_str, detail) = match &marker.kind {
                    MarkerKind::Bind { field } => ("bind", field.clone()),
                    MarkerKind::Event { event, .. } => ("event", event.clone()),
                    MarkerKind::Ref { name } => ("ref", name.clone()),
                    MarkerKind::Transition { kind } => ("transition", kind.clone()),
                    MarkerKind::ClassToggle { name } => ("class", name.clone()),
                    MarkerKind::When => ("when", String::new()),
                    MarkerKind::Each => ("each", String::new()),
                    MarkerKind::TextExpr => ("text", String::new()),
                };
                e.writeln(&format!(
                    "markers.insert({:?}.into(), serde_json::json!({{\"type\": {:?}, \"detail\": {:?}}}));",
                    marker.id.to_string(),
                    kind_str,
                    detail,
                ));
            }
            e.writeln("gale_hydration[\"markers\"] = serde_json::Value::Object(markers);");
        }

        if !self.server_data_keys.is_empty() {
            e.writeln("let mut server_data = serde_json::Map::new();");
            for key in &self.server_data_keys {
                e.writeln(&format!(
                    "server_data.insert({key:?}.into(), serde_json::to_value(&{key}).unwrap_or_default());",
                ));
            }
            e.writeln("gale_hydration[\"data\"] = serde_json::Value::Object(server_data);");
        }

        e.writeln("html.push_str(&gale_hydration.to_string());");
        e.writeln("html.push_str(\"</script>\");");
    }
}

impl Default for HydrationCtx {
    fn default() -> Self {
        Self::new()
    }
}
