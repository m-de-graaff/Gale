//! WebSocket channel handler generator with broadcast state.
//!
//! For each GaleX [`ChannelDecl`], generates a Rust file containing:
//! - A shared broadcast state struct (`{Name}State`) using `tokio::sync::broadcast`
//! - An optional connection params struct (from channel params)
//! - An HTTP → WebSocket upgrade handler (Axum GET endpoint)
//! - A per-connection socket handler with direction-aware `tokio::select!` loop
//! - Lifecycle hooks (connect, receive, disconnect)
//!
//! The broadcast pattern ensures messages sent by any client are relayed
//! to all other connected clients on the same channel.

use std::collections::HashSet;

use crate::ast::*;
use crate::codegen::emit_stmt::{annotation_to_rust, emit_block_body};
use crate::codegen::rust_emitter::RustEmitter;
use crate::codegen::types::{collect_shared_type_refs, to_module_name, to_snake_case};

/// Emit a complete WebSocket channel handler Rust file.
pub fn emit_channel_file(
    e: &mut RustEmitter,
    decl: &ChannelDecl,
    known_shared_types: &HashSet<String>,
) {
    let channel_name = &decl.name;
    let state_name = format!("{channel_name}State");
    e.emit_file_header(&format!("WebSocket channel: `{channel_name}`."));
    e.newline();

    let has_params = !decl.params.is_empty();

    // ── Imports ────────────────────────────────────────────────
    e.emit_use("axum::extract::ws::{Message, WebSocket, WebSocketUpgrade}");
    e.emit_use("axum::extract::State");
    e.emit_use("axum::response::IntoResponse");
    e.emit_use("std::sync::Arc");
    e.emit_use("tokio::sync::broadcast");
    if has_params {
        e.emit_use("axum::extract::Query");
    }

    // Shared type imports (enums, type aliases referenced in msg_ty or params)
    if !known_shared_types.is_empty() {
        let mut annotations: Vec<&TypeAnnotation> = vec![&decl.msg_ty];
        let param_anns: Vec<&TypeAnnotation> = decl
            .params
            .iter()
            .filter_map(|p| p.ty_ann.as_ref())
            .collect();
        annotations.extend(param_anns);
        for name in collect_shared_type_refs(&annotations, known_shared_types) {
            let mod_name = to_module_name(&name);
            e.emit_use(&format!("crate::shared::{mod_name}::{name}"));
        }
    }
    e.newline();

    // ── Broadcast state struct ─────────────────────────────────
    emit_state_struct(e, &state_name);
    e.newline();

    // ── Params struct (if any) ─────────────────────────────────
    if has_params {
        emit_params_struct(e, decl);
        e.newline();
    }

    // ── Upgrade handler ────────────────────────────────────────
    emit_upgrade_handler(e, decl, &state_name);
    e.newline();

    // ── Socket handler ─────────────────────────────────────────
    emit_socket_handler(e, decl, &state_name);
}

// ── State struct ───────────────────────────────────────────────────────

/// Emit the shared broadcast state struct + constructor.
fn emit_state_struct(e: &mut RustEmitter, state_name: &str) {
    e.emit_doc_comment("Shared broadcast state for this channel.");
    e.emit_doc_comment("");
    e.emit_doc_comment("Each connected client subscribes to the broadcast channel.");
    e.emit_doc_comment("Messages sent by any client are relayed to all subscribers.");
    e.block(&format!("pub struct {state_name}"), |e| {
        e.writeln("pub tx: broadcast::Sender<String>,");
    });
    e.newline();

    e.emit_impl(state_name, |e| {
        e.emit_doc_comment("Create a new channel state with a broadcast buffer of 256 messages.");
        e.block("pub fn new() -> Arc<Self>", |e| {
            e.writeln("let (tx, _) = broadcast::channel(256);");
            e.writeln("Arc::new(Self { tx })");
        });
    });
}

// ── Params struct ──────────────────────────────────────────────────────

/// Emit the params struct for channel connection parameters.
fn emit_params_struct(e: &mut RustEmitter, decl: &ChannelDecl) {
    let struct_name = format!("{}Params", pascal_case(&decl.name));
    e.emit_attribute("derive(Debug, serde::Deserialize)");
    e.block(&format!("pub struct {struct_name}"), |e| {
        for p in &decl.params {
            let field_name = to_snake_case(&p.name);
            let ty = if let Some(ann) = &p.ty_ann {
                annotation_to_rust(ann)
            } else {
                "String".into()
            };
            e.writeln(&format!("pub {field_name}: {ty},"));
        }
    });
}

// ── Upgrade handler ────────────────────────────────────────────────────

/// Emit the HTTP → WebSocket upgrade handler.
fn emit_upgrade_handler(e: &mut RustEmitter, decl: &ChannelDecl, state_name: &str) {
    let has_params = !decl.params.is_empty();
    let params_type = format!("{}Params", pascal_case(&decl.name));
    let channel_name = &decl.name;

    e.emit_doc_comment(&format!("GET /ws/__gx/channels/{channel_name}"));
    e.emit_doc_comment("");
    e.emit_doc_comment("Upgrades the HTTP connection to a WebSocket.");

    let sig = if has_params {
        format!(
            "pub async fn handler(\n    ws: WebSocketUpgrade,\n    State(state): State<Arc<{state_name}>>,\n    Query(params): Query<{params_type}>,\n) -> impl IntoResponse"
        )
    } else {
        format!(
            "pub async fn handler(\n    ws: WebSocketUpgrade,\n    State(state): State<Arc<{state_name}>>,\n) -> impl IntoResponse"
        )
    };

    e.block(&sig, |e| {
        if has_params {
            e.writeln("ws.on_upgrade(move |socket| handle_socket(socket, state, params))");
        } else {
            e.writeln("ws.on_upgrade(move |socket| handle_socket(socket, state))");
        }
    });
}

// ── Socket handler ─────────────────────────────────────────────────────

/// Emit the per-connection socket handler.
///
/// The structure varies based on channel direction:
/// - **Bidirectional**: `tokio::select!` with recv from client + broadcast relay
/// - **ServerToClient**: broadcast relay loop only
/// - **ClientToServer**: recv from client loop only
fn emit_socket_handler(e: &mut RustEmitter, decl: &ChannelDecl, state_name: &str) {
    let has_params = !decl.params.is_empty();
    let params_type = format!("{}Params", pascal_case(&decl.name));

    let sig = if has_params {
        format!(
            "async fn handle_socket(mut socket: WebSocket, state: Arc<{state_name}>, params: {params_type})"
        )
    } else {
        format!("async fn handle_socket(mut socket: WebSocket, state: Arc<{state_name}>)")
    };

    e.block(&sig, |e| {
        // Subscribe to broadcast for relay (bidirectional and server-to-client)
        let needs_rx = matches!(
            decl.direction,
            ChannelDirection::Bidirectional | ChannelDirection::ServerToClient
        );
        if needs_rx {
            e.writeln("let mut rx = state.tx.subscribe();");
            e.newline();
        }

        // on connect
        emit_event_handler(e, decl, "connect");

        // Main loop — varies by direction
        match decl.direction {
            ChannelDirection::Bidirectional => emit_bidirectional_loop(e, decl),
            ChannelDirection::ServerToClient => emit_server_to_client_loop(e),
            ChannelDirection::ClientToServer => emit_client_to_server_loop(e, decl),
        }

        e.newline();

        // on disconnect
        emit_event_handler(e, decl, "disconnect");
    });
}

/// Emit the bidirectional `tokio::select!` loop.
fn emit_bidirectional_loop(e: &mut RustEmitter, decl: &ChannelDecl) {
    e.emit_comment("--- bidirectional message loop ---");
    e.block("loop", |e| {
        e.block("tokio::select!", |e| {
            // Branch 1: incoming message from this client
            e.block("msg = socket.recv() =>", |e| {
                e.block("match msg", |e| {
                    e.block("Some(Ok(Message::Text(text))) =>", |e| {
                        emit_receive_body(e, decl);
                    });
                    e.writeln("Some(Ok(Message::Close(_))) | None => break,");
                    e.writeln("_ => {}");
                });
            });
            // Branch 2: broadcast from other clients
            e.block("msg = rx.recv() =>", |e| {
                e.block("if let Ok(text) = msg", |e| {
                    e.block(
                        "if socket.send(Message::Text(text.into())).await.is_err()",
                        |e| {
                            e.writeln("break;");
                        },
                    );
                });
            });
        });
    });
}

/// Emit the server-to-client loop (broadcast relay only).
fn emit_server_to_client_loop(e: &mut RustEmitter) {
    e.emit_comment("--- server-to-client relay loop ---");
    e.block("loop", |e| {
        e.block("match rx.recv().await", |e| {
            e.block("Ok(text) =>", |e| {
                e.block(
                    "if socket.send(Message::Text(text.into())).await.is_err()",
                    |e| {
                        e.writeln("break;");
                    },
                );
            });
            e.writeln("Err(_) => break,");
        });
    });
}

/// Emit the client-to-server recv loop (no relay to client).
fn emit_client_to_server_loop(e: &mut RustEmitter, decl: &ChannelDecl) {
    e.emit_comment("--- client-to-server receive loop ---");
    e.block("while let Some(msg) = socket.recv().await", |e| {
        e.block("match msg", |e| {
            e.block("Ok(Message::Text(text)) =>", |e| {
                emit_receive_body(e, decl);
            });
            e.writeln("Ok(Message::Close(_)) => break,");
            e.writeln("Err(_) => break,");
            e.writeln("_ => {}");
        });
    });
}

// ── Event handler emission ─────────────────────────────────────────────

/// Emit the body of the receive handler + broadcast.
fn emit_receive_body(e: &mut RustEmitter, decl: &ChannelDecl) {
    if let Some(handler) = find_handler(decl, "receive") {
        // Deserialize the message
        if let Some(param) = handler.params.first() {
            let param_name = to_snake_case(&param.name);
            emit_msg_deserialize(e, &decl.msg_ty, &param_name);
        }
        // Emit handler body
        emit_block_body(e, &handler.body);
    }
    // Broadcast to all subscribers
    e.writeln("let _ = state.tx.send(text.to_string());");
}

/// Emit an event handler block (connect or disconnect).
fn emit_event_handler(e: &mut RustEmitter, decl: &ChannelDecl, event: &str) {
    if let Some(handler) = find_handler(decl, event) {
        e.emit_comment(&format!("--- on {event} ---"));
        emit_block_body(e, &handler.body);
        e.newline();
    }
}

// ── Message deserialization ────────────────────────────────────────────

/// Emit the message deserialization expression.
///
/// - `string` → use text directly
/// - `int` / `float` → parse from text
/// - Named types → `serde_json::from_str`
fn emit_msg_deserialize(e: &mut RustEmitter, msg_ty: &TypeAnnotation, param_name: &str) {
    if is_string_type(msg_ty) {
        e.writeln(&format!("let {param_name} = text.to_string();"));
    } else if let Some(rust_ty) = is_primitive_parse_type(msg_ty) {
        e.writeln(&format!(
            "let {param_name} = match text.parse::<{rust_ty}>() {{"
        ));
        e.indent();
        e.writeln("Ok(v) => v,");
        e.writeln("Err(_) => continue,");
        e.dedent();
        e.writeln("};");
    } else {
        let rust_ty = annotation_to_rust(msg_ty);
        e.writeln(&format!(
            "let {param_name} = match serde_json::from_str::<{rust_ty}>(&text) {{"
        ));
        e.indent();
        e.writeln("Ok(v) => v,");
        e.writeln("Err(_) => continue,");
        e.dedent();
        e.writeln("};");
    }
}

// ── Helpers ────────────────────────────────────────────────────────────

fn find_handler<'a>(decl: &'a ChannelDecl, event: &str) -> Option<&'a ChannelHandler> {
    decl.handlers.iter().find(|h| h.event == event)
}

fn is_string_type(ty: &TypeAnnotation) -> bool {
    matches!(ty, TypeAnnotation::Named { name, .. } if name == "string")
}

fn is_primitive_parse_type(ty: &TypeAnnotation) -> Option<&'static str> {
    match ty {
        TypeAnnotation::Named { name, .. } => match name.as_str() {
            "int" => Some("i64"),
            "float" => Some("f64"),
            _ => None,
        },
        _ => None,
    }
}

fn pascal_case(name: &str) -> String {
    let mut result = String::with_capacity(name.len());
    let mut next_upper = true;
    for ch in name.chars() {
        if ch == '_' {
            next_upper = true;
        } else if next_upper {
            result.push(ch.to_uppercase().next().unwrap_or(ch));
            next_upper = false;
        } else {
            result.push(ch);
        }
    }
    result
}

// ── Tests ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::span::Span;

    fn s() -> Span {
        Span::dummy()
    }

    fn emit(decl: &ChannelDecl) -> String {
        let mut e = RustEmitter::new();
        let no_shared = HashSet::new();
        emit_channel_file(&mut e, decl, &no_shared);
        e.finish()
    }

    fn basic_channel(direction: ChannelDirection) -> ChannelDecl {
        ChannelDecl {
            name: "Chat".into(),
            params: vec![],
            direction,
            msg_ty: TypeAnnotation::Named {
                name: "string".into(),
                span: s(),
            },
            handlers: vec![],
            span: s(),
        }
    }

    #[test]
    fn channel_has_broadcast_imports() {
        let out = emit(&basic_channel(ChannelDirection::Bidirectional));
        assert!(out.contains("use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade}"));
        assert!(out.contains("use tokio::sync::broadcast"));
        assert!(out.contains("use std::sync::Arc"));
        assert!(out.contains("use axum::extract::State"));
    }

    #[test]
    fn channel_has_state_struct() {
        let out = emit(&basic_channel(ChannelDirection::Bidirectional));
        assert!(out.contains("pub struct ChatState {"));
        assert!(out.contains("pub tx: broadcast::Sender<String>,"));
    }

    #[test]
    fn channel_state_has_new() {
        let out = emit(&basic_channel(ChannelDirection::Bidirectional));
        assert!(out.contains("pub fn new() -> Arc<Self>"));
        assert!(out.contains("broadcast::channel(256)"));
    }

    #[test]
    fn channel_handler_accepts_state() {
        let out = emit(&basic_channel(ChannelDirection::Bidirectional));
        assert!(out.contains("State(state): State<Arc<ChatState>>"));
    }

    #[test]
    fn channel_has_route_doc() {
        let out = emit(&basic_channel(ChannelDirection::Bidirectional));
        assert!(out.contains("/// GET /ws/__gx/channels/Chat"));
    }

    #[test]
    fn bidirectional_has_select() {
        let out = emit(&basic_channel(ChannelDirection::Bidirectional));
        assert!(out.contains("tokio::select!"), "should use select!");
        assert!(out.contains("socket.recv()"), "should recv from client");
        assert!(out.contains("rx.recv()"), "should recv from broadcast");
    }

    #[test]
    fn bidirectional_broadcasts() {
        let out = emit(&basic_channel(ChannelDirection::Bidirectional));
        assert!(
            out.contains("state.tx.send(text.to_string())"),
            "should broadcast"
        );
    }

    #[test]
    fn server_to_client_no_client_recv() {
        let out = emit(&basic_channel(ChannelDirection::ServerToClient));
        assert!(
            !out.contains("socket.recv()"),
            "S->C should not recv from client"
        );
        assert!(out.contains("rx.recv()"), "S->C should recv from broadcast");
    }

    #[test]
    fn client_to_server_no_broadcast_rx() {
        let out = emit(&basic_channel(ChannelDirection::ClientToServer));
        assert!(
            out.contains("socket.recv()"),
            "C->S should recv from client"
        );
        assert!(
            !out.contains("let mut rx"),
            "C->S should not subscribe to broadcast"
        );
    }

    #[test]
    fn client_to_server_broadcasts_received() {
        let out = emit(&basic_channel(ChannelDirection::ClientToServer));
        assert!(
            out.contains("state.tx.send"),
            "C->S should still broadcast received msgs"
        );
    }

    #[test]
    fn channel_with_receive_handler() {
        let out = emit(&ChannelDecl {
            name: "Echo".into(),
            params: vec![],
            direction: ChannelDirection::Bidirectional,
            msg_ty: TypeAnnotation::Named {
                name: "string".into(),
                span: s(),
            },
            handlers: vec![ChannelHandler {
                event: "receive".into(),
                params: vec![Param {
                    name: "msg".into(),
                    ty_ann: None,
                    default: None,
                    span: s(),
                }],
                body: Block {
                    stmts: vec![],
                    span: s(),
                },
                span: s(),
            }],
            span: s(),
        });
        assert!(
            out.contains("let msg = text.to_string()"),
            "should bind msg"
        );
    }

    #[test]
    fn channel_with_connect_handler() {
        let out = emit(&ChannelDecl {
            name: "Live".into(),
            params: vec![],
            direction: ChannelDirection::Bidirectional,
            msg_ty: TypeAnnotation::Named {
                name: "string".into(),
                span: s(),
            },
            handlers: vec![ChannelHandler {
                event: "connect".into(),
                params: vec![],
                body: Block {
                    stmts: vec![Stmt::ExprStmt {
                        expr: Expr::IntLit {
                            value: 1,
                            span: s(),
                        },
                        span: s(),
                    }],
                    span: s(),
                },
                span: s(),
            }],
            span: s(),
        });
        assert!(out.contains("// --- on connect ---"));
    }

    #[test]
    fn channel_with_disconnect_handler() {
        let out = emit(&ChannelDecl {
            name: "Tracker".into(),
            params: vec![],
            direction: ChannelDirection::Bidirectional,
            msg_ty: TypeAnnotation::Named {
                name: "string".into(),
                span: s(),
            },
            handlers: vec![ChannelHandler {
                event: "disconnect".into(),
                params: vec![],
                body: Block {
                    stmts: vec![Stmt::ExprStmt {
                        expr: Expr::IntLit {
                            value: 0,
                            span: s(),
                        },
                        span: s(),
                    }],
                    span: s(),
                },
                span: s(),
            }],
            span: s(),
        });
        assert!(out.contains("// --- on disconnect ---"));
    }

    #[test]
    fn channel_with_params() {
        let out = emit(&ChannelDecl {
            name: "Room".into(),
            params: vec![
                Param {
                    name: "roomId".into(),
                    ty_ann: Some(TypeAnnotation::Named {
                        name: "string".into(),
                        span: s(),
                    }),
                    default: None,
                    span: s(),
                },
                Param {
                    name: "userId".into(),
                    ty_ann: Some(TypeAnnotation::Named {
                        name: "int".into(),
                        span: s(),
                    }),
                    default: None,
                    span: s(),
                },
            ],
            direction: ChannelDirection::Bidirectional,
            msg_ty: TypeAnnotation::Named {
                name: "string".into(),
                span: s(),
            },
            handlers: vec![],
            span: s(),
        });
        assert!(out.contains("pub struct RoomParams"));
        assert!(out.contains("pub room_id: String,"));
        assert!(out.contains("pub user_id: i64,"));
        assert!(out.contains("Query(params): Query<RoomParams>"));
    }

    #[test]
    fn channel_no_params() {
        let out = emit(&basic_channel(ChannelDirection::Bidirectional));
        assert!(!out.contains("Params"));
        assert!(!out.contains("Query"));
    }

    #[test]
    fn channel_named_msg_type() {
        let out = emit(&ChannelDecl {
            name: "Data".into(),
            params: vec![],
            direction: ChannelDirection::Bidirectional,
            msg_ty: TypeAnnotation::Named {
                name: "ChatMessage".into(),
                span: s(),
            },
            handlers: vec![ChannelHandler {
                event: "receive".into(),
                params: vec![Param {
                    name: "msg".into(),
                    ty_ann: None,
                    default: None,
                    span: s(),
                }],
                body: Block {
                    stmts: vec![],
                    span: s(),
                },
                span: s(),
            }],
            span: s(),
        });
        assert!(out.contains("serde_json::from_str::<ChatMessage>(&text)"));
    }

    #[test]
    fn channel_int_msg_type() {
        let out = emit(&ChannelDecl {
            name: "Counter".into(),
            params: vec![],
            direction: ChannelDirection::ClientToServer,
            msg_ty: TypeAnnotation::Named {
                name: "int".into(),
                span: s(),
            },
            handlers: vec![ChannelHandler {
                event: "receive".into(),
                params: vec![Param {
                    name: "value".into(),
                    ty_ann: None,
                    default: None,
                    span: s(),
                }],
                body: Block {
                    stmts: vec![],
                    span: s(),
                },
                span: s(),
            }],
            span: s(),
        });
        assert!(out.contains("text.parse::<i64>()"));
    }

    #[test]
    fn channel_with_shared_msg_type() {
        let shared: HashSet<String> = ["ChatMessage"].iter().map(|s| s.to_string()).collect();
        let mut e = RustEmitter::new();
        emit_channel_file(
            &mut e,
            &ChannelDecl {
                name: "Chat".into(),
                params: vec![],
                direction: ChannelDirection::Bidirectional,
                msg_ty: TypeAnnotation::Named {
                    name: "ChatMessage".into(),
                    span: s(),
                },
                handlers: vec![],
                span: s(),
            },
            &shared,
        );
        let out = e.finish();
        assert!(out.contains("use crate::shared::chat_message::ChatMessage;"));
    }
}
