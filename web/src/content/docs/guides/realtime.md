# Realtime

WebSocket channels for bidirectional communication.

## Channel declaration

```text
shared {
  type ChatMessage = {
    user: string,
    text: string,
    timestamp: int
  }
}

channel chat(room: string) <-> ChatMessage {
  on connect {
    broadcast({ user: "system", text: "User joined", timestamp: now() })
  }

  on receive(msg: ChatMessage) {
    // Validate, transform, and rebroadcast
    broadcast(msg)
  }

  on disconnect {
    broadcast({ user: "system", text: "User left", timestamp: now() })
  }
}
```

## Direction modes

| Syntax | Direction | Use case |
|--------|-----------|----------|
| `->` | Server to client | Push notifications, live dashboards, event streams |
| `<-` | Client to server | Telemetry, input streaming, analytics |
| `<->` | Bidirectional | Chat, collaboration, real-time editing |

The compiler validates handler bodies against the channel direction. For example, a `->` (server-to-client) channel cannot have `on receive` handlers since the server only pushes.

## Client subscription

```text
client {
  signal messages: ChatMessage[] = []

  let ws = subscribe chat("general")

  effect {
    ws.on("message", fn(msg: ChatMessage) {
      messages = [...messages, msg]
    })
  }
}
```

The compiler generates:
- **Rust:** A WebSocket upgrade handler using Axum's WebSocket support with typed message serialization
- **JavaScript:** A client wrapper with auto-reconnect, typed message handling, and connection lifecycle

## Channel parameters

Channel parameters (like `room` in the example above) are validated at connection time. The compiler checks that parameter types are provided when subscribing.

## Combining with actions

Use actions for durable operations alongside channels for real-time updates:

```text
server {
  action sendMessage(room: string, text: string) -> ChatMessage {
    let msg = { user: currentUser(), text: text, timestamp: now() }
    // Persist to database
    db.insert("messages", msg)
    // Broadcast to channel subscribers
    broadcast_to(room, msg)
    return msg
  }

  action getHistory(room: string) -> ChatMessage[] {
    return db.query("SELECT * FROM messages WHERE room = $1 ORDER BY timestamp DESC LIMIT 100", room)
  }
}
```

This pattern gives you both real-time delivery (via the channel) and durable history (via the database action).
