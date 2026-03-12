spec: task
name: "Stage1-Task2: SSE Integration with crew-rs"
tags: [stage1, sse, crew-rs]
---

## Intent

Integrate with the crew-rs gateway via SSE (Server-Sent Events). moly-ai already supports SSE-based
bot integration, and the crew-rs gateway also supports SSE. This task connects both ends so that
moly-ai can conduct streaming conversations with crew-rs over SSE.

## Decided

- Reuse the existing OpenAI SSE streaming parser (parse_sse)
- The crew-rs gateway's SSE endpoint is compatible with the OpenAI format
- Do not introduce a new HTTP client library; use the existing reqwest

## Boundary

### Allowed Changes
- src/clients/**
- examples/crew-rs-sse/**

### Forbidden
- Do not modify the core trait (BotClient) signatures
- Do not introduce tokio runtime dependencies into library code

## Out of Scope

- WebSocket duplex communication (Task 3)
- Tool approval UI (Task 4)
- Complex authentication/handshake implementations

## Acceptance Criteria

Scenario: SSE streaming connection to crew-rs
  Test: test_sse_crew_rs_connection
  Given crew-rs gateway is running locally
  When moly-ai sends a chat request via SSE
  Then streaming response chunks are received
  And they are assembled into a complete MessageContent

Scenario: SSE error handling
  Test: test_sse_connection_error
  Given crew-rs gateway is unreachable
  When moly-ai attempts an SSE connection
  Then a ClientError::Network error is returned
