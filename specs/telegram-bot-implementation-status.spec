spec: project
name: "Telegram Bot API — Implementation Status"
tags: [telegram, bot-api, botfather, status]
---

## Intent

Moly implements a Telegram Bot API compatible server locally, enabling any
teloxide-based framework (e.g., Octos, python-telegram-bot) to connect to
Moly as if it were Telegram. Users create and manage bots through an in-app
BotFather dialog, obtain API tokens, and configure external bot frameworks
to connect to `http://localhost:{port}`. No real Telegram account is needed.

This spec documents the complete implementation status across AITK (library)
and Moly (app), combining deep research on real Telegram Bot API with what
is currently built, what remains, and what is out of scope.


### Architecture

#### Layer Diagram

```
┌─────────────────────────────────────────────────────────┐
│                    Moly App (Makepad)                    │
│                                                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │ BotFather    │  │ Bot Chat     │  │ Bot Chat     │  │
│  │ Chat View    │  │ (Weather)    │  │ (Code)       │  │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  │
│         │                 │                 │           │
│  ┌──────▼─────────────────▼─────────────────▼────────┐  │
│  │             Moly Bot Manager (App Layer)           │  │
│  │  • BotFather dialog (/newbot, /mybots, etc.)      │  │
│  │  • User messages → push_update()                  │  │
│  │  • recv_outbound() → display bot replies in UI    │  │
│  └──────────────────────┬────────────────────────────┘  │
└─────────────────────────┼───────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────┐
│                      AITK (Library)                      │
│                                                         │
│  ┌─────────────────────────────────────────────────┐    │
│  │      Telegram Bot API Server (axum HTTP)         │    │
│  │  http://localhost:{port}/bot{token}/{method}     │    │
│  │                                                  │    │
│  │  Core interface (for App layer):                 │    │
│  │  • push_update(bot_token, Update)                │    │
│  │  • recv OutboundEvent (sendMessage, edit, delete)│    │
│  └─────────────────────┬───────────────────────────┘    │
│  ┌────────────────┐    │    ┌────────────────────────┐  │
│  │ BotStore       │◄───┘    │ UpdateQueueManager     │  │
│  │ (SQLite)       │         │ (per-bot long-polling)  │  │
│  └────────────────┘         └────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────┐
│              External Bot Clients (HTTP)                 │
│  Octos / teloxide / python-telegram-bot / any HTTP    │
└─────────────────────────────────────────────────────────┘
```

#### Message Flow

1. User sends a message in Moly UI
2. Moly constructs a Telegram `Update` JSON and calls `ServerState::push_update()`
3. `push_update()` stores the update in SQLite, wakes the bot's long-poller
4. External client (Octos via teloxide) calls `getUpdates` and receives the update
5. External client responds via `sendMessage` (or editMessageText, deleteMessage)
6. AITK pushes an `OutboundEvent` through the unbounded channel
7. Moly app receives the event and displays it in the chat UI


### Implementation Status

#### Bot API Endpoints (11 routes: 7 functional, 4 stubs)

| Endpoint             | Method   | Status        | Notes                                           |
|----------------------|----------|---------------|-------------------------------------------------|
| `getMe`              | GET/POST | Implemented   | Returns bot identity from SQLite                |
| `getUpdates`         | POST     | Implemented   | Long-polling, configurable timeout (max 60s), single-poller enforcement (409 on conflict) |
| `sendMessage`        | POST     | Implemented   | Text + optional InlineKeyboardMarkup, stores message, emits OutboundEvent |
| `editMessageText`    | POST     | Implemented   | Emits OutboundEvent::EditMessage                |
| `deleteMessage`      | POST     | Implemented   | Emits OutboundEvent::DeleteMessage              |
| `answerCallbackQuery`| POST     | Implemented   | Always returns true (no popup text forwarding)  |
| `getFile`            | POST     | Implemented   | Looks up media metadata in SQLite, returns relative path |
| `getWebhookInfo`     | GET/POST | Stub          | Returns empty WebhookInfo (teloxide startup compatibility) |
| `deleteWebhook`      | POST     | Stub          | No-op, returns true (teloxide startup compatibility) |
| `setMyCommands`      | POST     | Stub          | Accepts and discards commands (not persisted)   |
| `getMyCommands`      | GET/POST | Stub          | Always returns empty list                       |
| `sendPhoto`          | -        | Not impl      | Type definitions exist (PhotoSize)              |
| `sendVoice`          | -        | Not impl      | Type definitions exist (Voice)                  |
| `sendAudio`          | -        | Not impl      | Type definitions exist (Audio)                  |
| `sendDocument`       | -        | Not impl      | Type definitions exist (Document)               |
| `sendChatAction`     | -        | Not impl      | Typing indicator                                |
| `getChat`            | -        | Not impl      |                                                  |
| `setWebhook`         | -        | Out of scope  | Long-polling only design                        |

#### BotFather Commands (5 direct + management menu)

| Command              | Status         | Description                                     |
|----------------------|----------------|-------------------------------------------------|
| `/start`             | Implemented    | Show welcome message with command list           |
| `/help`              | Implemented    | Same as /start                                   |
| `/newbot`            | Implemented    | 2-step wizard: name → username → token           |
| `/mybots`            | Implemented    | List all bots with numbered selection            |
| `/cancel`            | Implemented    | Cancel active operation, context-aware messaging |
| View Token           | Implemented    | Management menu option 1                         |
| Edit Name            | Implemented    | Management menu option 2                         |
| Revoke Token         | Implemented    | Management menu option 3                         |
| Delete Bot           | Implemented    | Management menu option 4, requires "confirm delete" |
| Back                 | Implemented    | Management menu option 5                         |
| `/setdescription`    | Not impl       | BotUpdate.description field exists in store      |
| `/setabouttext`      | Not impl       | BotUpdate.about_text field exists in store       |
| `/setuserpic`        | Not impl       | BotUpdate.photo_path field exists in store       |
| `/setcommands`       | Not impl       | setMyCommands endpoint is a stub                 |
| `/setjoingroups`     | N/A            | No group chat support                            |
| `/setprivacy`        | N/A            | No group chat support                            |
| `/setinline`         | N/A            | No inline mode                                   |

#### Dialog State Machine

```
Idle ──/newbot──→ AwaitingBotName ──name──→ AwaitingUsername ──valid──→ Idle (bot created)
                                                            ──invalid──→ AwaitingUsername (retry)

Idle ──/mybots──→ Idle (list displayed)
                  ──number/username──→ ManagingBot
                                       ├── "1" → View Token → Idle
                                       ├── "2" → AwaitingNewName ──name──→ Idle (updated)
                                       ├── "3" → Revoke Token → Idle
                                       ├── "4" → ConfirmingDelete ──"confirm delete"──→ Idle (deleted)
                                       │                          ──other──→ Idle (cancelled)
                                       └── "5" → Idle (exited menu)

Any state ──/cancel──→ Idle
Any state ──/command──→ (command handler, resets state)
```

#### Data Types Implemented

**Telegram-compatible response/entity types:**
ApiResponse, User, Chat, Message, Update, CallbackQuery, InlineKeyboardMarkup,
InlineKeyboardButton, PhotoSize, Voice, Audio, Document, File, WebhookInfo,
BotCommand.

**Request body types:**
GetUpdatesRequest, SendMessageRequest, EditMessageTextRequest,
DeleteMessageRequest, AnswerCallbackQueryRequest, GetFileRequest,
SetMyCommandsRequest.

**App-layer types:**
BotInfo, BotUpdate, OutboundEvent (SendMessage, EditMessage, DeleteMessage).

**Not implemented:**
ReplyKeyboardMarkup, ForceReply, ReplyKeyboardRemove.

#### SQLite Schema (4 tables + version tracking)

**bots** — Bot metadata
- id (INTEGER PK), token (TEXT UNIQUE), name, username (TEXT UNIQUE),
  description, about_text, photo_path, created_at, updated_at

**updates** — Pending updates for long-polling
- id (INTEGER PK = update_id), bot_id (FK → bots), payload (JSON TEXT),
  created_at
- Index: `idx_updates_bot(bot_id)`

**messages** — Message history
- id (INTEGER PK), bot_id (FK → bots), message_id, chat_id, is_from_bot,
  content, media_type, media_path, caption, reply_markup (JSON TEXT),
  timestamp
- Constraint: UNIQUE(bot_id, message_id)
- Indexes: `idx_messages_bot_chat(bot_id, chat_id)`,
  `idx_messages_timestamp(timestamp)`

**media** — File metadata for getFile
- id (INTEGER PK), file_id (TEXT UNIQUE), file_unique_id, file_path,
  mime_type, file_size

**schema_version** — Internal migration tracking
- version (INTEGER)

#### BotStore API Surface

| Method                     | Returns                   | Description                            |
|----------------------------|---------------------------|----------------------------------------|
| `open(config)`             | `Result<Self>`            | Open/create DB, run migrations         |
| `create_bot(name, user)`   | `Result<BotInfo>`         | Create bot, generate token             |
| `get_bot_by_token(token)`  | `Result<Option<BotInfo>>` | Look up bot by API token               |
| `list_bots()`              | `Result<Vec<BotInfo>>`    | All bots ordered by created_at ASC     |
| `update_bot(token, upd)`   | `Result<BotInfo>`         | Selective field update                 |
| `delete_bot(token)`        | `Result<()>`              | Cascade delete (messages, updates)     |
| `revoke_token(old_token)`  | `Result<String>`          | Generate new token, return it          |
| `insert_update(bot_id, j)` | `Result<i64>`             | Store update JSON, return update_id    |
| `get_updates(bot_id, o, l)`| `Result<Vec<(i64, Str)>>` | Get updates with offset, limit         |
| `store_message(...)`       | `Result<i64>`             | Store message, auto-increment msg_id   |
| `media_dir()`              | `&str`                    | Configured media directory path        |
| `store_media(...)`         | `Result<()>`              | Insert or replace file metadata        |
| `get_media(file_id)`       | `Result<Option<(S, O<S>)>>`| Get (file_path, mime_type)           |

Configuration: `BotStoreConfig { db_path: String, media_dir: String }`
Thread safety: `Mutex<Connection>` wrapping, supports `:memory:` for tests.

#### Error Handling (BotApiError)

| Variant          | HTTP Code | Description                                  |
|------------------|-----------|----------------------------------------------|
| `InvalidToken`   | 401       | Token doesn't match any registered bot       |
| `BotNotFound`    | 404       | Bot ID not found in store                    |
| `InvalidRequest` | 400       | Malformed request or constraint violation    |
| `ConflictPoller` | 409       | Another getUpdates long-poller already active|
| `QueueFull`      | 429       | Update queue at capacity limit               |
| `DatabaseError`  | 500       | SQLite operation failed                      |
| `InternalError`  | 500       | Other unexpected failure                     |

Implements `Display`, `std::error::Error`, and `From<rusqlite::Error>`.
Error responses use standard Telegram format:
`{"ok": false, "error_code": N, "description": "..."}`.


## Decisions

- HTTP framework: axum (async, tower-compatible, Rust-native)
- Persistence: SQLite via rusqlite (single-file, zero-config, suitable for local app)
- Token format: `{bot_id}:{uuid_hex_32}` (teloxide splits on `:` to extract bot_id)
- Async channels: `futures::channel::mpsc` (cross-platform, not tied to tokio)
- Update delivery: long-polling only (no webhooks — localhost does not need them)
- User model: single-user (user_id always 1, private chat only)
- Default port: 8488 (configurable via `bot_server_port` preference)
- Platform gating: all server code behind `#[cfg(not(target_arch = "wasm32"))]`
- BotFather interaction: text + numbered menus; MessageContent supports quick_replies
- Outbound events: `futures::channel::mpsc::unbounded` channel from AITK to app
- Username rules: 3-32 chars, lowercase ASCII + digits + underscores, ends with `bot` or `_bot`
- BotStore API: all methods synchronized via `Mutex<Connection>`, `:memory:` for tests
- Server binding: localhost only (127.0.0.1), graceful shutdown via oneshot channel
- Long-polling constraints: max timeout 60s, max limit 100, one poller per bot


## Boundaries

### Allowed Changes

**AITK (Library Layer)**
- `aitk/src/telegram_server/server.rs` — ServerState, ServerConfig, ServerHandle
- `aitk/src/telegram_server/api/mod.rs` — Router with 11 endpoint routes
- `aitk/src/telegram_server/api/handlers.rs` — HTTP handler implementations
- `aitk/src/telegram_server/store.rs` — BotStore, BotStoreConfig, SQLite operations
- `aitk/src/telegram_server/queue.rs` — UpdateQueueManager, BotPollState
- `aitk/src/telegram_server/types.rs` — API types, request/response structs
- `aitk/src/telegram_server/error.rs` — BotApiError enum (7 variants)
- `aitk/src/telegram_server/schema.sql` — SQLite schema

**Moly App**
- `src/bot_manager/dialog.rs` — DialogState state machine, COMMAND_LIST
- `src/bot_manager/client.rs` — BotFatherClient (BotClient trait impl)
- `src/data/store.rs` — Server startup, outbound event loop, provider registration
- `src/data/preferences.rs` — bot_server_port preference
- `src/chat/chats_deck.rs` — BotFather welcome message pre-population

### Forbidden
- Do not forward any traffic to real api.telegram.org
- Do not implement webhook mode (setWebhook)
- Do not implement group chat or multi-user support
- Do not implement Payments, Stickers, Games, or Web Apps API


## Constraints

### Telegram Bot API Reference

#### Long Polling vs Webhooks

Real Telegram supports both long polling (`getUpdates`) and webhooks
(`setWebhook`). Moly implements long polling only — webhooks are unnecessary
for localhost communication. The `getWebhookInfo` and `deleteWebhook`
endpoints exist as no-op stubs for teloxide startup compatibility (teloxide
calls these on boot to ensure clean state).

#### Keyboard Types

Real Telegram has two distinct keyboard types:

- **InlineKeyboardMarkup** — buttons attached below a specific message,
  trigger `callback_query` updates. Moly: types fully implemented,
  `answerCallbackQuery` returns true but discards text/alert parameters.
- **ReplyKeyboardMarkup** — persistent keyboard replacing the device
  keyboard, sends text messages. Moly: not implemented (out of scope).

#### Bot Management (BotFather Comparison)

Real Telegram's @BotFather offers ~20 commands for bot configuration. Moly
implements the core lifecycle (create, list, rename, view/revoke token,
delete). Commands like `/setdescription`, `/setabouttext`, and `/setuserpic`
have storage fields prepared but no dialog handlers yet.

#### Key Differences from Real Telegram

| Aspect                  | Real Telegram                        | Moly                               |
|-------------------------|--------------------------------------|-------------------------------------|
| Server                  | api.telegram.org (cloud)             | localhost:{port} (local)            |
| Token format            | `{bot_id}:{alphanumeric}`            | `{bot_id}:{uuid_hex_32}`           |
| Updates                 | Long-polling or Webhooks             | Long-polling only                   |
| Chat types              | Private, group, supergroup, channel  | Private only (single-user)          |
| Inline keyboards        | Full callback + URL + web app        | Callback data only                  |
| Reply keyboards         | Full support                         | Not implemented                     |
| Media                   | Full upload/download with CDN        | Type definitions only               |
| Rate limiting           | 30 msg/sec, etc.                     | None                                |
| TLS                     | Required (HTTPS)                     | Not needed (localhost)              |
| Bot discovery           | @BotFather, t.me/botname             | In-app only                         |
| User identity           | Real Telegram user IDs               | Always user_id=1                    |


### Testing

#### Test Locations

**AITK — telegram_server module:**

| File                   | Tests                                        |
|------------------------|----------------------------------------------|
| `api/mod.rs`           | test_get_me_returns_bot_info, test_invalid_token_returns_401 |
| `store.rs`             | test_create_and_get_bot, test_duplicate_username_rejected, test_list_bots, test_update_bot, test_delete_bot_cascades, test_revoke_token, test_update_id_global_increment, test_get_updates_with_offset, test_store_message_auto_increment, test_store_and_get_media |
| `queue.rs`             | test_register_and_unregister_poller, test_notify_without_poller_is_noop, test_is_bot_online |
| `types.rs`             | test_api_response_ok_serialization, test_api_response_error_serialization, test_update_with_message_roundtrip, test_callback_query_deserialization, test_inline_keyboard_roundtrip, test_parse_chat_id_number, test_parse_chat_id_string |

**Moly — bot_manager module:**

| File                   | Tests                                        |
|------------------------|----------------------------------------------|
| `dialog.rs`            | test_botfather_start_command, test_start_has_no_quick_replies, test_botfather_newbot_flow, test_newbot_token_in_code_block, test_botfather_newbot_invalid_username, test_botfather_newbot_duplicate_username, test_mybots_lists_bots, test_mybots_empty, test_botfather_token_command, test_botfather_revoke_token, test_botfather_setname, test_botfather_deletebot_confirms, test_botfather_unknown_command, test_process_input_returns_message_content, test_resolve_bot_selection_returns_message_content, test_username_validation |
| `client.rs`            | test_botfather_welcome_message                |

#### Test Commands

AITK tests (run from aitk repo root):
`cargo test -p aitk --lib telegram_server`

Moly tests (run from moly-ai repo root):
`cargo test -p moly --lib bot_manager`


### Roadmap

#### Implemented
- Core Bot API server (11 routes: 7 functional, 4 teloxide-compatibility stubs)
- BotFather dialog (full lifecycle: create, list, rename, view/revoke token, delete)
- SQLite persistence (4 tables with indexes and cascade delete)
- Long-polling with single-poller enforcement (409 ConflictPoller)
- Outbound event channel (SendMessage, EditMessage, DeleteMessage)
- Provider registration (BotFather auto-registered on startup)
- Dialog state machine (6 states, context-aware /cancel)
- Username validation (3-32 chars, lowercase, must end with bot/_bot)
- MessageContent return type with quick_replies support

#### In Progress
- Outbound event routing to chat UI (currently logging only)
- Bot chat view integration

#### Planned
- Media endpoints (sendPhoto, sendVoice, sendAudio, sendDocument)
- File download serving (static file route for media_dir)
- sendChatAction (typing indicator)
- /setdescription, /setabouttext dialog handlers
- Bot avatars (/setuserpic)
- Token copy-to-clipboard in BotFather UI
- Online/offline status indicators
- InlineKeyboardMarkup rendering in chat UI

#### Out of Scope
- Real Telegram API proxy (no forwarding to api.telegram.org)
- Webhook mode (setWebhook / deleteWebhook as functional endpoints)
- Group / supergroup / channel chat support
- Multi-user support
- Payments, Stickers, Games, Web Apps API
- Bot store / discovery / marketplace
- Reply keyboards (ReplyKeyboardMarkup)
- Inline mode (inline queries, /setinline)
- Rate limiting


## Acceptance Criteria

Scenario: Bot API server starts and serves getMe
  Test: test_get_me_returns_bot_info
  Given a bot "TestBot" with username "testbot" is registered
  When an HTTP GET request is sent to `/bot{token}/getMe`
  Then the response status is 200
  And the response body contains `"ok": true` with the bot identity

Scenario: Invalid token returns error
  Test: test_invalid_token_returns_401
  Given no bot is registered with token "invalid:token"
  When an HTTP GET request is sent to `/botinvalid:token/getMe`
  Then the response contains `"ok": false` with error_code 401

Scenario: BotFather /newbot creates a bot
  Test: test_botfather_newbot_flow
  Given the dialog state is Idle
  When the user sends "/newbot", then "Weather Bot", then "weather_bot"
  Then the response contains "Done" and "Token"
  And the store contains a bot named "Weather Bot" with username "weather_bot"

Scenario: BotFather /mybots lists bots
  Test: test_mybots_lists_bots
  Given bots "Alpha" and "Beta" exist
  When the user sends "/mybots"
  Then the response contains "Your bots" with "2 total"
  And each bot is listed with its number and username

Scenario: Username validation rejects invalid input
  Test: test_username_validation
  Given usernames "ab" (too short), "UPPER_bot" (uppercase), "noending" (no bot suffix)
  When each username is validated
  Then all are rejected with descriptive error messages

Scenario: Duplicate username is rejected
  Test: test_botfather_newbot_duplicate_username
  Given a bot with username "weather_bot" already exists
  When the user tries to create another bot with the same username
  Then the response contains "already taken"
  And the dialog remains in AwaitingUsername state

Scenario: Bot deletion requires confirmation
  Test: test_botfather_deletebot_confirms
  Given a bot exists and the user is in ManagingBot state
  When the user selects option "4" (Delete Bot)
  Then the response asks "Are you sure"
  And when the user types "confirm delete", the bot is deleted

Scenario: Token revocation generates new token
  Test: test_botfather_revoke_token
  Given a bot exists with a known token
  When the user revokes the token via management option "3"
  Then the response contains "revoked"
  And the old token no longer resolves to any bot

Scenario: Single-poller enforcement
  Test: test_register_and_unregister_poller
  Given a poller is registered for bot_id 1
  When another poller tries to register for the same bot
  Then it fails with ConflictPoller error
  And after unregistering, a new poller can register successfully

Scenario: Bot online detection
  Test: test_is_bot_online
  Given no poller is registered for bot_id 1
  When checking is_bot_online with 120s timeout
  Then it returns false
  And after registering a poller, it returns true

Scenario: Store CRUD operations
  Test: test_create_and_get_bot
  Given an empty bot store
  When creating a bot "TestBot" with username "test_bot"
  Then the returned BotInfo has matching name and username
  And the token contains ":"
  And the bot can be retrieved by its token

Scenario: Cascade delete removes all bot data
  Test: test_delete_bot_cascades
  Given a bot with stored messages and updates
  When the bot is deleted
  Then the bot token no longer resolves
  And messages and updates are removed

Scenario: /cancel is context-aware
  Test: test_process_input_returns_message_content
  Given the dialog state is Idle
  When the user sends "/cancel"
  Then the response contains "No active operation"

Scenario: Message auto-increment per bot
  Test: test_store_message_auto_increment
  Given a bot exists
  When two messages are stored sequentially
  Then message_ids are 1 and 2 respectively

Scenario: Media metadata storage and retrieval
  Test: test_store_and_get_media
  Given media with file_id "f1" is stored
  When retrieving by file_id "f1"
  Then the file_path and mime_type match the stored values
