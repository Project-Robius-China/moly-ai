spec: task
name: "Stage1-Task2: SSE Integration with Octos"
tags: [stage1, sse, Octos]
---

## Intent

Integrate with the Octos gateway via SSE (Server-Sent Events). moly-ai already supports SSE-based
bot integration, and the Octos gateway also supports SSE. This task connects both ends so that
moly-ai can conduct streaming conversations with Octos over SSE.

## Decided

- Reuse the existing OpenAI SSE streaming parser (parse_sse)
- The Octos gateway's SSE endpoint is compatible with the OpenAI format
- Do not introduce a new HTTP client library; use the existing reqwest

## Boundary

### Allowed Changes
- src/clients/**
- examples/Octos-sse/**

### Forbidden
- Do not modify the core trait (BotClient) signatures
- Do not introduce tokio runtime dependencies into library code

## Out of Scope

- WebSocket duplex communication (Task 3)
- Tool approval UI (Task 4)
- Complex authentication/handshake implementations

## Acceptance Criteria

Scenario: SSE streaming connection to Octos
  Test: test_sse_octos_connection
  Given Octos gateway is running locally
  When moly-ai sends a chat request via SSE
  Then streaming response chunks are received
  And they are assembled into a complete MessageContent

Scenario: SSE error handling
  Test: test_sse_connection_error
  Given Octos gateway is unreachable
  When moly-ai attempts an SSE connection
  Then a ClientError::Network error is returned
