# Stage 2: Telegram Bot API Compatible Server
# Moly Bot-Native Messaging — BotFather + Telegram Bot API Compatible Server

Status: design
Created: 2026-03-10
Target: 5/3 Demo

## Intent

Transform Moly from a "developer configures Provider" model to a "Telegram-style
Bot management" model. Users create Bots through a BotFather conversation within
Moly, obtain a token, paste the token into Octos's Telegram configuration,
Octos connects to Moly's Bot API server via teloxide, and users chat with
Bots directly in Moly.

Core innovation: Moly implements a Telegram Bot API compatible server, so any
framework supporting the Telegram Bot API (Octos, python-telegram-bot, etc.)
can connect.

## Architecture

### Layer Breakdown

```
┌─────────────────────────────────────────────────────────┐
│                      Moly App (Makepad)                 │
│                                                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │ BotFather    │  │ Bot Chat     │  │ Bot Chat     │  │
│  │ Chat View    │  │ "Weather     │  │ "Code        │  │
│  │              │  │  Assistant"  │  │  Assistant"  │  │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  │
│         │                 │                 │           │
│  ┌──────▼─────────────────▼─────────────────▼────────┐  │
│  │           Moly Bot Manager (App Layer)             │  │
│  │  • BotFather dialog logic (/newbot, /mybots, etc.) │  │
│  │  • User message → push_update()                    │  │
│  │  • recv_outbound() → Display Bot reply in UI       │  │
│  └──────────────────────┬────────────────────────────┘  │
└─────────────────────────┼───────────────────────────────┘
                          │
┌─────────────────────────▼───────────────────────────────┐
│                    AITK (Library Layer)                   │
│                                                         │
│  ┌─────────────────────────────────────────────────┐    │
│  │      Telegram Bot API Server (New Module)        │    │
│  │  http://localhost:{port}/bot{token}/{method}     │    │
│  │                                                  │    │
│  │  Endpoint implementations:                       │    │
│  │  • getMe              — Bot identity info        │    │
│  │  • getUpdates         — Long polling for messages│    │
│  │  • sendMessage        — Send text (+inline kbd)  │    │
│  │  • sendPhoto          — Send image               │    │
│  │  • sendVoice          — Send voice               │    │
│  │  • sendAudio          — Send audio               │    │
│  │  • sendDocument       — Send document            │    │
│  │  • editMessageText    — Edit message             │    │
│  │  • deleteMessage      — Delete message           │    │
│  │  • answerCallbackQuery — Callback query response │    │
│  │  • getFile / file download — File download       │    │
│  │                                                  │    │
│  │  Core interfaces (for App layer to call):        │    │
│  │  • push_update(bot_token, Update)                │    │
│  │  • recv_outbound(bot_token) -> OutboundMessage   │    │
│  │  • create_bot(name, ...) -> BotInfo + token      │    │
│  │  • list_bots() -> Vec<BotInfo>                   │    │
│  │  • update_bot(token, BotUpdate)                  │    │
│  │  • delete_bot(token)                             │    │
│  └────────────────────┬────────────────────────────┘    │
│                       │                                  │
│  ┌────────────────────▼────────────────────────────┐    │
│  │              Bot Store (SQLite)                  │    │
│  │  Tables:                                         │    │
│  │  • bots:     id, token, name, description,       │    │
│  │              username, photo, created_at          │    │
│  │  • messages: id, bot_id, chat_id, sender,        │    │
│  │              content, media_type, media_path,     │    │
│  │              reply_markup, timestamp              │    │
│  │  • media:    id, file_id, file_path, mime_type   │    │
│  └─────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────┘
                          ▲
                          │ teloxide long polling
┌─────────────────────────┴───────────────────────────────┐
│                   Octos Gateway                        │
│  Configuration:                                          │
│  • TELOXIDE_TELEGRAM_API_URL=http://localhost:{port}     │
│  • TELEGRAM_BOT_TOKEN=moly_{random_token}                │
│  • Other config unchanged (LLM provider, tools, etc.)    │
└─────────────────────────────────────────────────────────┘
```

### Message Flow

1. **User sends a message in Moly**
   → Moly Bot Manager constructs a Telegram `Update` object
   → Calls AITK `push_update(bot_token, update)`
   → Update enters the per-bot queue

2. **Octos pulls messages**
   → teloxide calls `POST /bot{token}/getUpdates` (long polling)
   → AITK server retrieves Updates from the queue and returns them
   → If no messages, blocks and waits (returns empty array after timeout)

3. **Octos processes and replies**
   → Agent reasoning + tool calls
   → teloxide calls `POST /bot{token}/sendMessage`
   → AITK server receives the message, stores it in the messages table
   → Notifies Moly via `recv_outbound()` that a new message is available

4. **Moly displays the reply**
   → Bot Manager receives OutboundMessage
   → Updates Chat UI to display Bot reply
   → Supports inline keyboard button rendering

## BotFather Dialog Design

BotFather is a built-in special Bot in Moly that appears in the chat list.

### Command System

```
Basic commands:
/start     — Welcome message, shows available commands
/newbot    — Create a new Bot (interactive wizard)
/mybots    — List all Bots (inline button selection)

Edit Bot:
/setname        — Change Bot name
/setdescription — Change Bot description
/setabouttext   — Change Bot about text
/setuserpic     — Change Bot avatar
/deletebot      — Delete Bot

Bot settings:
/token     — View Bot token (for pasting into Octos)
/revoke    — Regenerate token
```

### /newbot Dialog Flow

```
User: /newbot
BotFather: Alright, let's create a new Bot. Please give it a name:

User: Weather Assistant
BotFather: Great. Now give it a username (must end with bot):

User: weather_bot
BotFather: Done! Your new Bot "Weather Assistant" has been created.

Token: moly_a1b2c3d4e5f6...

How to use:
1. Copy the Token above
2. Paste it into the Telegram config on the Octos web page
3. Set the API URL to: http://localhost:{port}
4. Start the Gateway

You can now find "Weather Assistant" in the chat list and start chatting.

Use /mybots to manage your Bots, use /token to view the Token at any time.
```

### /mybots Interaction

```
User: /mybots
BotFather: Choose a Bot to manage:
[Weather Assistant] [Code Assistant] [Translator Bot]

User: (clicks Weather Assistant)
BotFather: Weather Assistant (@weather_bot)
Choose an action:
[Edit Name] [Edit Description] [Edit Avatar]
[View Token] [Reset Token] [Delete Bot]
[Back]
```

## Telegram Bot API Implementation Details

### Token Format

Telegram original format: `{bot_id}:{random_string}`
Moly format: `moly_{bot_id}_{random_hex}` (prefix to distinguish origin)

### Long Polling Implementation (getUpdates)

```
Request: POST /bot{token}/getUpdates
Body: { "offset": 12345, "timeout": 30, "limit": 100 }

Logic:
1. Validate token → find corresponding Bot
2. Check update_queue for messages after offset
3. If messages exist → return immediately
4. If no messages → wait using tokio::select!:
   a. New message arrives → return
   b. After timeout seconds → return empty array []
5. Return in standard Telegram Response<Vec<Update>> format
```

### Telegram Data Types (subset to implement)

```rust
// Core types (corresponding to Telegram Bot API)
struct User { id: i64, is_bot: bool, first_name: String, username: Option<String> }
struct Chat { id: i64, chat_type: String, title: Option<String> }
struct Message {
    message_id: i64,
    from: Option<User>,
    chat: Chat,
    date: i64,
    text: Option<String>,
    photo: Option<Vec<PhotoSize>>,
    voice: Option<Voice>,
    audio: Option<Audio>,
    document: Option<Document>,
    caption: Option<String>,
    reply_markup: Option<InlineKeyboardMarkup>,
}
struct Update { update_id: i64, message: Option<Message>, callback_query: Option<CallbackQuery> }
struct CallbackQuery { id: String, from: User, message: Option<Message>, data: Option<String> }
struct InlineKeyboardMarkup { inline_keyboard: Vec<Vec<InlineKeyboardButton>> }
struct InlineKeyboardButton { text: String, callback_data: Option<String>, url: Option<String> }

// Media types
struct PhotoSize { file_id: String, file_unique_id: String, width: i32, height: i32 }
struct Voice { file_id: String, file_unique_id: String, duration: i32 }
struct Audio { file_id: String, file_unique_id: String, duration: i32, title: Option<String> }
struct Document { file_id: String, file_unique_id: String, file_name: Option<String> }
struct File { file_id: String, file_unique_id: String, file_path: Option<String> }
```

### API Endpoint Details

| Endpoint | Method | Description | Octos Usage |
|----------|--------|-------------|---------------|
| /bot{token}/getMe | GET/POST | Return Bot info | Startup verification |
| /bot{token}/getUpdates | POST | Long polling for messages | Core message receiving |
| /bot{token}/sendMessage | POST | Send text + keyboard | Core message sending |
| /bot{token}/sendPhoto | POST | Send image | Media sending |
| /bot{token}/sendVoice | POST | Send voice | Voice reply |
| /bot{token}/sendAudio | POST | Send audio | Audio reply |
| /bot{token}/sendDocument | POST | Send document | File sending |
| /bot{token}/editMessageText | POST | Edit message | Message update |
| /bot{token}/deleteMessage | POST | Delete message | Message deletion |
| /bot{token}/answerCallbackQuery | POST | Callback response | Button interaction |
| /bot{token}/getFile | GET/POST | Get file info | Media download |
| /file/bot{token}/{file_path} | GET | Download file | Media download |

## Data Storage (SQLite)

### Schema

```sql
CREATE TABLE bots (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    token       TEXT NOT NULL UNIQUE,
    name        TEXT NOT NULL,
    username    TEXT NOT NULL UNIQUE,
    description TEXT DEFAULT '',
    about_text  TEXT DEFAULT '',
    photo_path  TEXT,
    created_at  DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at  DATETIME DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE messages (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    bot_id       INTEGER NOT NULL REFERENCES bots(id),
    message_id   INTEGER NOT NULL,  -- per-bot auto-increment
    chat_id      INTEGER NOT NULL,
    sender_id    INTEGER NOT NULL,
    sender_name  TEXT,
    is_from_bot  BOOLEAN NOT NULL DEFAULT FALSE,
    content      TEXT,
    media_type   TEXT,  -- photo, voice, audio, document
    media_path   TEXT,
    caption      TEXT,
    reply_markup TEXT,  -- JSON serialized InlineKeyboardMarkup
    timestamp    DATETIME DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(bot_id, message_id)
);

CREATE TABLE media (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    file_id        TEXT NOT NULL UNIQUE,
    file_unique_id TEXT NOT NULL,
    file_path      TEXT NOT NULL,
    mime_type      TEXT,
    file_size      INTEGER
);

CREATE INDEX idx_messages_bot_chat ON messages(bot_id, chat_id);
CREATE INDEX idx_messages_timestamp ON messages(timestamp);
```

## Octos Side Configuration

Octos only needs to support a custom API URL in the Telegram channel config:

```json
{
  "gateway": {
    "channels": [
      {
        "type": "telegram",
        "token_env": "TELEGRAM_BOT_TOKEN",
        "base_url": "http://localhost:8088",
        "allowed_senders": ""
      }
    ]
  }
}
```

teloxide supports setting a custom base URL via `Bot::from_env_with_client()` or
the `TELOXIDE_TELEGRAM_API_URL` environment variable.
The changes required in Octos are minimal: pass the base_url config in `TelegramChannel::new()`.

## Moly App Layer Design

### BotFather as a Built-in Bot

- Automatically appears first in the chat list on startup
- Has a special icon/badge (gear or robot + blue verification mark)
- Cannot be deleted or renamed
- Dialog logic is handled entirely locally (no network involved)

### Bot Chat List

Each created Bot automatically appears in the chat list:
- Displays Bot name + avatar
- Shows connection status (online/offline, depending on whether Octos is polling)
- Click to enter chat view
- Chat view reuses the existing Chat interface

### Connection Status Detection

- When Octos calls getUpdates, mark the Bot as "online"
- If no getUpdates request for more than 2 minutes, mark as "offline"
- Display green/gray dot in the UI to indicate connection status

## Boundary & Constraints

- AITK's Bot API server does not depend on Makepad; pure Rust + axum
- AITK does not handle UI logic; it only provides message queue interfaces
- Tokens are only valid locally; they cannot be used with the real Telegram
- Server only listens on localhost; not exposed to the network
- SQLite database file is stored in the user data directory

## Out of Scope (Not in This Phase)

- Real Telegram proxy (no forwarding to api.telegram.org)
- Webhook mode (only long polling is supported)
- Group chat support (only 1:1 conversations)
- Payments / Stickers / Games API
- Bot store / Bot discovery
- Multi-user support (single-user desktop application)

## Decision Log

| # | Decision | Rationale |
|---|----------|-----------|
| 1 | BotFather as a dialog rather than a settings page | Replicates the Telegram experience; zero UI learning curve |
| 2 | Approach A: Full Telegram Bot API server | Maximum compatibility; zero changes to Octos |
| 3 | Implement API server in the AITK layer | Generic and reusable; not tied to Makepad |
| 4 | SQLite persistence | Supports message history and complex queries |
| 5 | Full feature replication | Complete support for text/media/keyboard/edit/delete |
| 6 | Long polling (not Webhook) | Desktop app has no public IP; consistent with Octos's existing mode |

## Success Criteria

1. User creates a Bot via BotFather `/newbot` within 30 seconds
2. After pasting the token into Octos, Octos can successfully connect and chat
3. Supports bidirectional text message communication
4. Supports inline keyboard button interactions sent by Octos
5. Supports media messages (images, voice, documents)
6. Bot connection status is displayed in real time
7. Message history is persistently stored and recoverable after restart
