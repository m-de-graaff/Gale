//! Channel → JavaScript reactive wrapper code generation.
//!
//! For each GaleX [`ChannelDecl`], generates an ES module that exports a
//! factory function wrapping the runtime `channel()` with direction-aware
//! typed send/receive and auto-reconnect.
//!
//! Generated output for `channel Chat <-> string { ... }`:
//! ```js
//! import { channel } from '/_gale/runtime.js';
//!
//! export function Chat(params) {
//!   const conn = channel('Chat', params, { maxRetries: 5 });
//!   return {
//!     messages: conn.messages,
//!     connected: conn.connected,
//!     send: conn.send,
//!     close: conn.close,
//!     reconnect: conn.reconnect,
//!     dispose: conn.dispose,
//!   };
//! }
//! ```

use crate::ast::*;
use crate::codegen::js_emitter::JsEmitter;
use crate::codegen::types::to_module_name;

// ── Metadata ───────────────────────────────────────────────────────────

/// Metadata about a generated JS channel module.
#[derive(Debug, Clone)]
pub struct ChannelJsMeta {
    /// PascalCase channel name (e.g. `Chat`).
    pub channel_name: String,
    /// Snake_case module file name (e.g. `chat`).
    pub module_name: String,
    /// Direction string for documentation.
    pub direction: String,
    /// Whether the client can send messages.
    pub can_send: bool,
    /// Whether the client receives messages.
    pub can_receive: bool,
}

// ── Public entry point ─────────────────────────────────────────────────

/// Emit a complete channel JS wrapper module.
pub fn emit_channel_js_file(e: &mut JsEmitter, decl: &ChannelDecl) -> ChannelJsMeta {
    let direction_str = match decl.direction {
        ChannelDirection::ServerToClient => "server-to-client",
        ChannelDirection::ClientToServer => "client-to-server",
        ChannelDirection::Bidirectional => "bidirectional",
    };
    let can_send = matches!(
        decl.direction,
        ChannelDirection::ClientToServer | ChannelDirection::Bidirectional
    );
    let can_receive = matches!(
        decl.direction,
        ChannelDirection::ServerToClient | ChannelDirection::Bidirectional
    );

    e.emit_file_header(&format!("Channel: `{}` ({direction_str}).", decl.name));

    e.emit_import(&["channel"], "/_gale/runtime.js");
    e.newline();

    // Factory function — accepts connection params
    let has_params = !decl.params.is_empty();
    let fn_params = if has_params { &["params"][..] } else { &[][..] };
    let params_arg = if has_params { "params" } else { "null" };

    e.emit_export_fn(&decl.name, fn_params, |e| {
        e.writeln(&format!(
            "const conn = channel('{}', {params_arg}, {{ maxRetries: 5 }});",
            decl.name
        ));

        // Build return object based on direction
        e.writeln("return {");
        e.indent();

        if can_receive {
            e.writeln("messages: conn.messages,");
        }
        e.writeln("connected: conn.connected,");
        if can_send {
            e.writeln("send: conn.send,");
        }
        e.writeln("close: conn.close,");
        e.writeln("reconnect: conn.reconnect,");
        e.writeln("dispose: conn.dispose,");

        e.dedent();
        e.writeln("};");
    });

    ChannelJsMeta {
        channel_name: decl.name.to_string(),
        module_name: to_module_name(&decl.name),
        direction: direction_str.to_string(),
        can_send,
        can_receive,
    }
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codegen::js_emitter::JsEmitter;
    use crate::span::Span;

    fn s() -> Span {
        Span::dummy()
    }

    fn make_channel(name: &str, params: Vec<&str>, direction: ChannelDirection) -> ChannelDecl {
        ChannelDecl {
            name: name.into(),
            params: params
                .into_iter()
                .map(|n| Param {
                    name: n.into(),
                    ty_ann: None,
                    default: None,
                    span: s(),
                })
                .collect(),
            direction,
            msg_ty: TypeAnnotation::Named {
                name: "string".into(),
                span: s(),
            },
            handlers: vec![],
            span: s(),
        }
    }

    // ── Direction-aware exports ─────────────────────────────────

    #[test]
    fn bidirectional_has_send_and_messages() {
        let decl = make_channel("Chat", vec![], ChannelDirection::Bidirectional);
        let mut e = JsEmitter::new();
        let meta = emit_channel_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(out.contains("send: conn.send"), "send: {out}");
        assert!(out.contains("messages: conn.messages"), "messages: {out}");
        assert!(meta.can_send);
        assert!(meta.can_receive);
    }

    #[test]
    fn server_to_client_no_send() {
        let decl = make_channel("Events", vec![], ChannelDirection::ServerToClient);
        let mut e = JsEmitter::new();
        let meta = emit_channel_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(!out.contains("send:"), "no send: {out}");
        assert!(out.contains("messages: conn.messages"), "messages: {out}");
        assert!(!meta.can_send);
        assert!(meta.can_receive);
    }

    #[test]
    fn client_to_server_no_messages() {
        let decl = make_channel("Commands", vec![], ChannelDirection::ClientToServer);
        let mut e = JsEmitter::new();
        let meta = emit_channel_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(out.contains("send: conn.send"), "send: {out}");
        assert!(!out.contains("messages:"), "no messages: {out}");
        assert!(meta.can_send);
        assert!(!meta.can_receive);
    }

    // ── Common exports ─────────────────────────────────────────

    #[test]
    fn always_has_connected_close_reconnect_dispose() {
        let decl = make_channel("C", vec![], ChannelDirection::ServerToClient);
        let mut e = JsEmitter::new();
        emit_channel_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(
            out.contains("connected: conn.connected"),
            "connected: {out}"
        );
        assert!(out.contains("close: conn.close"), "close: {out}");
        assert!(
            out.contains("reconnect: conn.reconnect"),
            "reconnect: {out}"
        );
        assert!(out.contains("dispose: conn.dispose"), "dispose: {out}");
    }

    // ── Params ─────────────────────────────────────────────────

    #[test]
    fn channel_with_params() {
        let decl = make_channel("Room", vec!["roomId"], ChannelDirection::Bidirectional);
        let mut e = JsEmitter::new();
        emit_channel_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(
            out.contains("export function Room(params)"),
            "params: {out}"
        );
        assert!(
            out.contains("channel('Room', params,"),
            "passes params: {out}"
        );
    }

    #[test]
    fn channel_without_params() {
        let decl = make_channel("Global", vec![], ChannelDirection::Bidirectional);
        let mut e = JsEmitter::new();
        emit_channel_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(out.contains("export function Global()"), "no params: {out}");
        assert!(
            out.contains("channel('Global', null,"),
            "null params: {out}"
        );
    }

    // ── Imports & structure ─────────────────────────────────────

    #[test]
    fn channel_imports_runtime() {
        let decl = make_channel("C", vec![], ChannelDirection::Bidirectional);
        let mut e = JsEmitter::new();
        emit_channel_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(out.contains("import { channel } from '/_gale/runtime.js'"));
    }

    #[test]
    fn channel_meta() {
        let decl = make_channel("LiveFeed", vec!["userId"], ChannelDirection::ServerToClient);
        let mut e = JsEmitter::new();
        let meta = emit_channel_js_file(&mut e, &decl);

        assert_eq!(meta.channel_name, "LiveFeed");
        assert_eq!(meta.module_name, "live_feed");
        assert_eq!(meta.direction, "server-to-client");
    }

    #[test]
    fn channel_header() {
        let decl = make_channel("Chat", vec![], ChannelDirection::Bidirectional);
        let mut e = JsEmitter::new();
        emit_channel_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(out.contains("Channel: `Chat` (bidirectional)."));
    }

    #[test]
    fn channel_has_max_retries() {
        let decl = make_channel("C", vec![], ChannelDirection::Bidirectional);
        let mut e = JsEmitter::new();
        emit_channel_js_file(&mut e, &decl);
        let out = e.finish();

        assert!(out.contains("maxRetries: 5"), "reconnect config: {out}");
    }
}
