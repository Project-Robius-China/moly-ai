spec: task
name: "Stage1-Task3: WebSocket Gateway"
status: delayed
tags: [stage1, websocket, crew-rs]
---

> **Note:** Delayed: SSE is sufficient for crew-rs integration.

## Intent

Implement a WebSocket client to connect to the crew-rs WebSocket Gateway, supporting duplex
communication. The server can proactively push status updates, and the client can send chat
messages and receive streaming responses. moly-ai already has WebSocket support for openclaw
that can serve as a reference.

## Decided

- Use futures channels instead of tokio channels (cross-platform compatibility)
- Use tungstenite or a similar library for the WebSocket protocol layer
- Message format is JSON, compatible with the crew-rs frame protocol
- Authentication is passed via the WebSocket handshake header with an API key

## Boundary

### Allowed Changes
- src/clients/**
- src/utils/**

### Forbidden
- Do not modify the behavior of the existing OpenAI client
- Do not use tokio::spawn in library code
- Do not hardcode WebSocket addresses

## Out of Scope

- Tool approval UI (implemented separately in Task 4)
- Memory frame protocol (Stage 3)
- Advanced reconnection strategies

## Acceptance Criteria

Scenario: WebSocket handshake and authentication
  Test: test_ws_handshake_with_auth
  Given crew-rs WebSocket gateway address and API key
  When the client initiates a WebSocket connection
  Then the handshake succeeds and a connection is established

Scenario: Streaming chat message send/receive
  Test: test_ws_chat_streaming
  Given an established WebSocket connection
  When a chat message is sent
  Then multiple streaming chunks are received
  And they are assembled into a complete MessageContent

Scenario: Server-initiated push
  Test: test_ws_server_push
  Given an established WebSocket connection
  When the server pushes a status update frame
  Then the client correctly parses it and triggers the callback

Scenario: Connection error handling
  Test: test_ws_connection_error
  Given an invalid WebSocket address
  When the client attempts to connect
  Then a meaningful error message is returned
