spec: project
name: "Telegram Bot API — Implementation Status"
tags: [telegram, bot-api, botfather, status]
---

## Intent

Moly implements a Telegram Bot API compatible server locally, enabling any
teloxide-based framework (e.g., crew-rs, python-telegram-bot) to connect to
Moly as if it were Telegram. Users create and manage bots through an in-app
BotFather dialog, obtain API tokens, and configure external bot frameworks
to connect to `http://localhost:{port}`. No real Telegram account is needed.

This spec documents the complete implementation status across AITK (library)
and Moly (app), combining deep research on real Telegram Bot API with what
is currently built, what remains, and what is out of scope.

### Layer Diagram

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
│  crew-rs / teloxide / python-telegram-bot / any HTTP    │
└─────────────────────────────────────────────────────────┘
```

### Message Flow

1. User sends a message in Moly UI
2. Moly constructs a Telegram `Update` JSON and calls `ServerState::push_update()`
3. `push_update()` stores the update in SQLite, wakes the bot's long-poller
4. External client (crew-rs via teloxide) calls `getUpdates` and receives the update
5. External client responds via `sendMessage` (or editMessageText, deleteMessage)
6. AITK pushes an `OutboundEvent` through the unbounded channel
7. Moly app receives the event and displays it in the chat UI

### Bot API Endpoints Status

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
| `sendPhoto`          | -        | Not implemented | Type definitions exist (PhotoSize)             |
| `sendVoice`          | -        | Not implemented | Type definitions exist (Voice)                 |
| `sendAudio`          | -        | Not implemented | Type definitions exist (Audio)                 |
| `sendDocument`       | -        | Not implemented | Type definitions exist (Document)              |
| `sendChatAction`     | -        | Not implemented | Typing indicator                               |
| `getChat`            | -        | Not implemented |                                                 |
| `setWebhook`         | -        | Out of scope  | Long-polling only design                        |

### BotFather Commands Status

| Command   | Status      | Description                                        |
|-----------|-------------|----------------------------------------------------|
| `/start`  | Implemented | Show welcome message with command list              |
| `/help`   | Implemented | Same as /start                                      |
| `/newbot`  | Implemented | 2-step wizard: name → username → token              |
| `/mybots`  | Implemented | List all bots with numbered selection               |
| `/cancel`  | Implemented | Cancel active operation, context-aware messaging    |
| `/setname` | Implemented | Via management menu option 2 (Edit Name)            |
| `/token`   | Implemented | Via management menu option 1 (View Token)           |
| `/deletebot`| Implemented | Via management menu option 4 (requires `confirm delete`) |
| `/revoke`  | Implemented | Via management menu option 3 (Revoke Token)         |
| `/setdescription` | Not implemented | BotUpdate.description field exists in store |
| `/setabouttext`   | Not implemented | BotUpdate.about_text field exists in store  |
| `/setuserpic`     | Not implemented | BotUpdate.photo_path field exists in store  |
| `/setcommands`    | Not implemented | setMyCommands endpoint is a stub            |
| `/setjoingroups`  | Not applicable  | No group chat support                       |
| `/setprivacy`     | Not applicable  | No group chat support                       |
| `/setinline`      | Not applicable  | No inline mode                              |

### Dialog State Machine

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

### SQLite Schema (4 tables)

**bots** — Bot metadata (id, token, name, username, description, about_text, photo_path, timestamps)
**updates** — Pending updates for long-polling (id=update_id, bot_id, payload JSON)
**messages** — Message history (bot_id, message_id, chat_id, is_from_bot, content, media fields, reply_markup JSON)
**media** — File metadata for getFile (file_id, file_unique_id, file_path, mime_type, file_size)

Indexes: `idx_updates_bot(bot_id)`, `idx_messages_bot_chat(bot_id, chat_id)`, `idx_messages_timestamp(timestamp)`

### Data Types Implemented

Telegram-compatible: ApiResponse, User, Chat, Message, Update, CallbackQuery,
InlineKeyboardMarkup, InlineKeyboardButton, PhotoSize, Voice, Audio, Document,
File, WebhookInfo, BotCommand, OutboundEvent (SendMessage, EditMessage, DeleteMessage).

Not implemented: ReplyKeyboardMarkup, ForceReply, ReplyKeyboardRemove.

### Real Telegram vs Moly

| Aspect                  | Real Telegram                              | Moly                                   |
|-------------------------|--------------------------------------------|----------------------------------------|
| Server                  | api.telegram.org (cloud)                   | localhost:{port} (local)               |
| Token format            | `{bot_id}:{alphanumeric}`                  | `{bot_id}:{uuid_hex_32}`              |
| Updates                 | Long-polling or Webhooks                   | Long-polling only                      |
| Chat types              | Private, group, supergroup, channel        | Private only (single-user)             |
| Inline keyboards        | Full callback support                      | Types exist, answerCallbackQuery stubs |
| Reply keyboards         | Full support                               | Not implemented                        |
| Media                   | Full upload/download                       | Type definitions only                  |
| Rate limiting           | 30 msg/sec, etc.                           | None                                   |
| TLS                     | Required (HTTPS)                           | Not needed (localhost)                 |

### Roadmap

**Implemented:**
Core Bot API server (11 routes), BotFather dialog (full lifecycle), SQLite persistence,
long-polling with single-poller enforcement, outbound event channel, provider registration,
markdown-formatted responses, text + number interaction, context-aware /cancel.

**In Progress:**
Outbound event routing to chat UI (currently logging only), bot chat view integration.

**Planned:**
Media endpoints (sendPhoto, sendVoice, sendAudio, sendDocument), file download serving,
sendChatAction, /setdescription, /setabouttext, bot avatars, token copy-to-clipboard,
online/offline indicators, InlineKeyboardMarkup rendering.


## Decisions

- HTTP framework: axum (async, tower-compatible, Rust-native)
- Persistence: SQLite via rusqlite (single-file, zero-config, suitable for local app)
- Token format: `{bot_id}:{uuid_hex_32}` (teloxide splits on `:` to extract bot_id)
- Async channels: `futures::channel::mpsc` (cross-platform, not tied to tokio)
- Update delivery: long-polling only (no webhooks — localhost does not need them)
- User model: single-user (user_id always 1, private chat only)
- Default port: 8488 (configurable via `bot_server_port` preference)
- Platform gating: all server code behind `#[cfg(not(target_arch = "wasm32"))]`
- BotFather interaction: pure text + numbered menus (no quick-reply buttons)
- Outbound events: `futures::channel::mpsc::unbounded` channel from AITK to app
- Username rules: 3-32 chars, lowercase ASCII + digits + underscores, must end with `bot` or `_bot`
- Error responses: standard Telegram format `{"ok": false, "error_code": N, "description": "..."}`
- BotStore API: all methods synchronized via `Mutex<Connection>`, in-memory mode for tests


## Boundaries

### Allowed Changes

#### AITK (Library Layer)
- `aitk/src/telegram_server/server.rs` — ServerState, ServerConfig, ServerHandle
- `aitk/src/telegram_server/api/mod.rs` — Router with 11 endpoint routes
- `aitk/src/telegram_server/api/handlers.rs` — HTTP handler implementations
- `aitk/src/telegram_server/store.rs` — BotStore, BotStoreConfig, SQLite operations
- `aitk/src/telegram_server/queue.rs` — UpdateQueueManager, BotPollState
- `aitk/src/telegram_server/types.rs` — API types, request/response structs
- `aitk/src/telegram_server/error.rs` — BotApiError enum (7 variants)
- `aitk/src/telegram_server/schema.sql` — SQLite schema

#### Moly App
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

### Error Handling

| Variant          | HTTP Code | Description                                  |
|------------------|-----------|----------------------------------------------|
| `InvalidToken`   | 401       | Token doesn't match any registered bot       |
| `BotNotFound`    | 404       | Bot ID not found in store                    |
| `InvalidRequest` | 400       | Malformed request or constraint violation    |
| `ConflictPoller` | 409       | Another getUpdates long-poller already active|
| `QueueFull`      | 429       | Update queue at capacity limit               |
| `DatabaseError`  | 500       | SQLite operation failed                      |
| `InternalError`  | 500       | Other unexpected failure                     |


## Out of Scope

- Real Telegram API proxy (no forwarding to api.telegram.org)
- Webhook mode (setWebhook / deleteWebhook as functional endpoints)
- Group / supergroup / channel chat support
- Multi-user support
- Payments, Stickers, Games, Web Apps API
- Bot store / discovery / marketplace
- Internationalization of BotFather responses
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
