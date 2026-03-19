# Channels

Typed WebSocket communication with three direction modes.

```text
channel chat(room: string) <-> ChatMessage {
  on connect {
    broadcast({ text: "User joined", type: "system" })
  }

  on receive(msg: ChatMessage) {
    broadcast(msg)
  }

  on disconnect {
    broadcast({ text: "User left", type: "system" })
  }
}
```

## Direction modes

| Syntax | Direction | Use case |
|--------|-----------|----------|
| `->` | Server to client | Push notifications, live updates |
| `<-` | Client to server | Telemetry, input streaming |
| `<->` | Bidirectional | Chat, collaboration |

The compiler generates:

- **Rust:** WebSocket upgrade handler with typed message serialization
- **JavaScript:** Client wrapper with auto-reconnect and typed message handling

## Handlers

Channels support three lifecycle handlers:

- `on connect { }` — runs when a client connects
- `on receive(msg: T) { }` — runs when a message arrives
- `on disconnect { }` — runs when a client disconnects

## Channel parameters

Parameters (like `room`) are validated at connection time.

## Compiler checks

| Code | Condition |
|------|-----------|
| GX0513 | `channel` declared in client block |
