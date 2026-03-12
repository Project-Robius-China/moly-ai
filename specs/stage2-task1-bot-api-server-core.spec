spec: task
name: "AITK Telegram Bot API Server Core"
tags: [stage2, aitk, telegram-api, server]
---

## Intent

Implement a Telegram Bot API-compatible HTTP server core module in AITK. This server receives
long-polling requests from teloxide clients (e.g., crew-rs), forwards user messages, and receives
Bot replies. This is the infrastructure layer for Stage 2 Bot-Native Messaging, providing Moly
with the ability to communicate with any Telegram Bot framework.

The module must compile on all platforms via feature flags and cfg gates (disabled on wasm32).

## Constraints

- The entire module is gated under `cfg(not(target_arch = "wasm32"))` and uses the feature
  flag `telegram-server`
- Async primitives use `futures::channel::mpsc` instead of `tokio::mpsc`; long-polling waits
  use `futures::future::select` instead of `tokio::select!`
- Library code must not use `.unwrap()`; use `.expect()` only for invariant violations with
  a descriptive message
- All public types, functions, and methods must have doc comments
- Token format is `{numeric_bot_id}:{moly_random_hex}`, compatible with teloxide's token
  parsing (teloxide splits on `:` to extract bot_id)
- Only one `getUpdates` long-polling connection is allowed per Bot at a time (subsequent ones
  return 409 Conflict), consistent with real Telegram behavior
- Server state uses `Arc<ServerState>` for sharing; per-bot queues use independent locks
- Single-user mode: user_id is fixed at 1, each Bot corresponds to one chat_id (equal to bot_id)

## Decided

- HTTP framework: axum (consistent with existing moly-sync patterns)
- Token format: `{bot_id}:{moly_hex32}` (32-character random hex, compatible with teloxide)
- Default port: 8488, automatically selects a random port if the port is occupied
- Server starts lazily when Moly launches (on first Bot creation)
- Returns `ServerHandle` supporting graceful shutdown (following the moly-sync pattern)
- Per-bot update queue capacity limit: 1000 entries; oldest entries are dropped when exceeded
- getUpdates timeout range: 0-60 seconds, default 30 seconds
- All API responses follow the standard Telegram format: `{"ok": true, "result": ...}`
  or `{"ok": false, "error_code": N, "description": "..."}`
- teloxide calls `getWebhookInfo` and `deleteWebhook` at startup; these must be implemented
  as no-ops that return an empty webhook

## Boundary

### Allowed Changes
- moly-aitk/src/telegram_server/** (new)
- moly-aitk/Cargo.toml (add feature flag and dependencies)
- moly-aitk/src/lib.rs (export new module)

### Forbidden
- Do not modify existing moly-aitk client code (OpenAI client, CrewRs client, etc.)
- Do not add Makepad dependencies
- Do not use tokio channel primitives (use futures channels)
- Do not compile HTTP server code under wasm32

## Out of Scope

- Media file send/receive (sendPhoto, sendVoice, etc. -> Task 3)
- SQLite persistent storage (-> Task 2)
- BotFather dialog logic (-> Task 4)
- Moly UI integration (-> Task 5)
- crew-rs base_url modification (-> Task 6)

## Acceptance Criteria

Scenario: Server starts and listens on the configured port
  Test: test_server_starts_on_configured_port
  Given port is configured as "8488"
  When `TelegramBotApiServer::start(config)` is called to start the server
  Then the server listens on "http://127.0.0.1:8488"
  And a `ServerHandle` is returned that can be used for graceful shutdown

Scenario: getMe returns Bot information
  Test: test_get_me_returns_bot_info
  Given a Bot has been created with name "Weather Assistant", username "weather_bot", token "1:moly_abc123"
  When teloxide calls `GET /bot1:moly_abc123/getMe`
  Then the response status code is 200
  And the response body is:
    | Field            | Value              |
    | ok               | true               |
    | result.id        | 1                  |
    | result.is_bot    | true               |
    | result.first_name| Weather Assistant  |
    | result.username  | weather_bot        |

Scenario: Invalid token returns 401
  Test: test_invalid_token_returns_401
  When `GET /botinvalid_token/getMe` is called
  Then the response status code is 401
  And the response body `ok` is false
  And the response body `error_code` is 401

Scenario: getUpdates long-polling - returns immediately when messages are available
  Test: test_get_updates_returns_pending_messages
  Given Bot token "1:moly_abc123" has been created
  And the update queue contains "1" text message "Hello" with offset "100"
  When `POST /bot1:moly_abc123/getUpdates` is called with body `{"offset": 100, "timeout": 30}`
  Then the response is returned within "1" second
  And the result array length is "1"
  And result[0].update_id is "100"
  And result[0].message.text is "Hello"

Scenario: getUpdates long-polling - waits until timeout when no messages are available
  Test: test_get_updates_waits_until_timeout
  Given Bot token "1:moly_abc123" has been created
  And the update queue is empty
  When `POST /bot1:moly_abc123/getUpdates` is called with body `{"offset": 0, "timeout": 2}`
  Then the response is returned within "2" to "3" seconds
  And the result is an empty array

Scenario: getUpdates long-polling - returns immediately when a new message arrives during the wait
  Test: test_get_updates_returns_on_new_message
  Given Bot token "1:moly_abc123" has been created
  And the update queue is empty
  When `POST /bot1:moly_abc123/getUpdates` is called with body `{"offset": 0, "timeout": 30}`
  And after "1" second a message is pushed via push_update
  Then the response is returned within "2" seconds
  And the result array length is "1"

Scenario: getUpdates concurrent rejection
  Test: test_get_updates_rejects_concurrent_poller
  Given Bot token "1:moly_abc123" has been created
  And a getUpdates long-polling request is in progress
  When a second client calls `POST /bot1:moly_abc123/getUpdates`
  Then the second request response status code is 409

Scenario: sendMessage receives Bot reply
  Test: test_send_message_stores_and_notifies
  Given Bot token "1:moly_abc123" has been created
  When `POST /bot1:moly_abc123/sendMessage` is called with body:
    | Field     | Value   |
    | chat_id   | 1       |
    | text      | Hello   |
  Then the response status code is 200
  And the response body result.message_id is a positive integer
  And the outbound channel receives a message containing "Hello"

Scenario: sendMessage with inline keyboard
  Test: test_send_message_with_inline_keyboard
  Given Bot token "1:moly_abc123" has been created
  When `POST /bot1:moly_abc123/sendMessage` is called with body containing reply_markup:
    | Field                               | Value                                  |
    | chat_id                             | 1                                      |
    | text                                | Choose an option                       |
    | reply_markup.inline_keyboard[0][0]  | {"text":"A","callback_data":"a"}       |
  Then the response status code is 200
  And the outbound channel receives a message containing inline_keyboard data

Scenario: editMessageText edits a sent message
  Test: test_edit_message_text
  Given Bot token "1:moly_abc123" has been created
  And the Bot has sent a message with message_id "5"
  When `POST /bot1:moly_abc123/editMessageText` is called with body:
    | Field      | Value        |
    | chat_id    | 1            |
    | message_id | 5            |
    | text       | Updated text |
  Then the response status code is 200
  And the outbound channel receives an edit-type message

Scenario: deleteMessage deletes a message
  Test: test_delete_message
  Given Bot token "1:moly_abc123" has been created
  And the Bot has sent a message with message_id "5"
  When `POST /bot1:moly_abc123/deleteMessage` is called with body:
    | Field      | Value |
    | chat_id    | 1     |
    | message_id | 5     |
  Then the response status code is 200
  And the outbound channel receives a delete-type message

Scenario: answerCallbackQuery responds to callback
  Test: test_answer_callback_query
  Given Bot token "1:moly_abc123" has been created
  When `POST /bot1:moly_abc123/answerCallbackQuery` is called with body:
    | Field             | Value    |
    | callback_query_id | cq_001   |
  Then the response status code is 200
  And the response body result is true

Scenario: getWebhookInfo returns empty webhook (teloxide startup compatibility)
  Test: test_get_webhook_info_returns_empty
  Given Bot token "1:moly_abc123" has been created
  When `GET /bot1:moly_abc123/getWebhookInfo` is called
  Then the response status code is 200
  And result.url is an empty string

Scenario: deleteWebhook no-op (teloxide startup compatibility)
  Test: test_delete_webhook_noop
  Given Bot token "1:moly_abc123" has been created
  When `POST /bot1:moly_abc123/deleteWebhook` is called
  Then the response status code is 200
  And result is true

Scenario: setMyCommands stores and returns success
  Test: test_set_my_commands
  Given Bot token "1:moly_abc123" has been created
  When `POST /bot1:moly_abc123/setMyCommands` is called with body containing a commands list
  Then the response status code is 200
  And result is true

Scenario: push_update correctly enqueues messages
  Test: test_push_update_enqueues_message
  Given Bot token "1:moly_abc123" has been created
  When a text message is pushed via `server.push_update("1:moly_abc123", update)`
  Then the message can be retrieved via getUpdates

Scenario: Module compiles under wasm32 (without server code)
  Test: test_wasm32_compilation
  When moly-aitk is compiled with `cargo check --target wasm32-unknown-unknown`
  Then compilation succeeds
  But the `telegram_server` module is not included

Scenario: Error types cover all failure scenarios
  Test: test_error_types_are_defined
  When the `BotApiError` enum definition is inspected
  Then it contains the following variants:
    | Variant          | Description                        |
    | InvalidToken     | Token is invalid or does not exist |
    | BotNotFound      | Bot does not exist                 |
    | InvalidRequest   | Request format is invalid          |
    | ConflictPoller   | Concurrent polling conflict        |
    | QueueFull        | Update queue is full               |
    | InternalError    | Internal error                     |
