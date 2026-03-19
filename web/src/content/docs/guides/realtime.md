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
| `->` | Server to client | Push notifications, live dashboards |
| `<-` | Client to server | Telemetry, input streaming |
| `<->` | Bidirectional | Chat, collaboration |

## Combining with actions

Use actions for durable operations alongside channels for real-time delivery:

```text
server {
  action sendMessage(room: string, text: string) -> ChatMessage {
    let msg = { user: currentUser(), text: text, timestamp: now() }
    db.insert("messages", msg)
    broadcast_to(room, msg)
    return msg
  }
}
```
