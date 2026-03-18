# Chat App

A real-time chat application demonstrating GaleX channels (WebSocket).

## Features

- WebSocket channels for real-time communication
- Bidirectional messaging (client <-> server)
- Auto-reconnect with exponential backoff
- Reactive message list via signals

## Build

```bash
gale build --app-dir examples/chat/app --output-dir examples/chat/gale_build --name chat
```

## Project Structure

```
chat/
  galex.toml
  app/
    layout.gx         # HTML shell
    page.gx            # Chat room with channel declaration
```
