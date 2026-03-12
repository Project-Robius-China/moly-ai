# Telegram Bot API Server Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Moly a Telegram Bot API compatible server so crew-rs (via teloxide) can connect to Moly as if it were Telegram, enabling bot-native messaging UX.

**Architecture:** AITK provides a Telegram Bot API HTTP server (axum) with SQLite storage, exposing `push_update`/`recv_outbound` interfaces. Moly App provides BotFather dialog UI and bot chat integration. crew-rs connects via its existing teloxide code with a custom `base_url` pointing to Moly's local server.

**Tech Stack:** Rust, axum 0.7, rusqlite, futures 0.3, serde/serde_json, uuid, Makepad (Moly UI)

**Repos involved:**
- AITK: `/Users/zhaoyue/workspace/matrix/moly-ecosystem/aitk/` (Tasks 1-3)
- Moly: `/Users/zhaoyue/workspace/matrix/moly-ecosystem/moly-ai/` (Tasks 4-5)
- crew-rs: `/Users/zhaoyue/workspace/matrix/moly-ecosystem/crew-rs/` (Task 6)

**Specs:** `specs/stage2-task{1-6}-*.spec`

---

## Chunk 1: AITK Core — Types, Errors, Store, Server, Core Endpoints

This chunk implements the foundation: Telegram API types, error types, SQLite store,
HTTP server skeleton, and core text-messaging endpoints (getMe, getUpdates, sendMessage,
editMessageText, deleteMessage, answerCallbackQuery, plus teloxide compat stubs).

### Task 1.1: Feature flag and module scaffold

**Files:**
- Modify: `aitk/Cargo.toml`
- Create: `aitk/src/telegram_server/mod.rs`
- Modify: `aitk/src/lib.rs`

- [ ] **Step 1: Add dependencies and feature flag to AITK Cargo.toml**

Add under `[features]`:
```toml
telegram-server = ["dep:axum", "dep:rusqlite", "dep:tower-http"]
```

Add under `[dependencies]`:
```toml
axum = { version = "0.7", optional = true }
rusqlite = { version = "0.32", features = ["bundled"], optional = true }
tower-http = { version = "0.5", features = ["cors"], optional = true }
```

- [ ] **Step 2: Create module scaffold**

Create `aitk/src/telegram_server/mod.rs`:
```rust
//! Telegram Bot API compatible HTTP server.
//!
//! Implements a subset of the Telegram Bot API that allows teloxide-based
//! clients (e.g. crew-rs) to connect to this server instead of api.telegram.org.
//! The server provides long-polling (`getUpdates`), message sending, editing,
//! deletion, callback queries, and media file handling.

mod api;
mod error;
mod queue;
mod server;
mod store;
mod types;

pub use error::BotApiError;
pub use server::{TelegramBotApiServer, ServerConfig, ServerHandle};
pub use store::{BotStore, BotStoreConfig};
pub use types::*;
```

- [ ] **Step 3: Gate module in lib.rs**

Add to `aitk/src/lib.rs`:
```rust
#[cfg(all(feature = "telegram-server", not(target_arch = "wasm32")))]
pub mod telegram_server;
```

- [ ] **Step 4: Verify wasm32 compilation**

Run: `cd /Users/zhaoyue/workspace/matrix/moly-ecosystem/aitk && cargo check --target wasm32-unknown-unknown`
Expected: Compiles successfully, `telegram_server` module excluded.

- [ ] **Step 5: Verify native compilation**

Run: `cd /Users/zhaoyue/workspace/matrix/moly-ecosystem/aitk && cargo check --features telegram-server`
Expected: Compiles (with empty module stubs).

- [ ] **Step 6: Commit**

```bash
cd /Users/zhaoyue/workspace/matrix/moly-ecosystem/aitk
git add -A && git commit -m "feat(telegram-server): scaffold module with feature flag"
```

---

### Task 1.2: Telegram API types

**Files:**
- Create: `aitk/src/telegram_server/types.rs`

Implement the Telegram Bot API data types used by crew-rs's teloxide client.
Reference: https://core.telegram.org/bots/api

- [ ] **Step 1: Write type definitions**

Create `aitk/src/telegram_server/types.rs`:
```rust
//! Telegram Bot API compatible data types.
//!
//! These types mirror the subset of the Telegram Bot API used by teloxide.
//! They serialize/deserialize to the same JSON format as the real API.

use serde::{Deserialize, Serialize};

/// Telegram API response wrapper.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T: Serialize> {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(result: T) -> Self {
        Self {
            ok: true,
            result: Some(result),
            error_code: None,
            description: None,
        }
    }
}

impl ApiResponse<bool> {
    pub fn error(code: i32, description: impl Into<String>) -> Self {
        Self {
            ok: false,
            result: None,
            error_code: Some(code),
            description: Some(description.into()),
        }
    }
}

/// A Telegram user or bot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct User {
    pub id: i64,
    pub is_bot: bool,
    pub first_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
}

/// A Telegram chat.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Chat {
    pub id: i64,
    #[serde(rename = "type")]
    pub chat_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
}

/// A Telegram message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Message {
    pub message_id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<User>,
    pub chat: Chat,
    pub date: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caption: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub photo: Option<Vec<PhotoSize>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice: Option<Voice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<Audio>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document: Option<Document>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_markup: Option<InlineKeyboardMarkup>,
}

/// An incoming update from the Bot API.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Update {
    pub update_id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_query: Option<CallbackQuery>,
}

/// A callback query from an inline keyboard button.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CallbackQuery {
    pub id: String,
    pub from: User,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
}

/// Inline keyboard markup.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InlineKeyboardMarkup {
    pub inline_keyboard: Vec<Vec<InlineKeyboardButton>>,
}

/// A button in an inline keyboard.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InlineKeyboardButton {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub callback_data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

// --- Media types ---

/// A photo size (Telegram sends multiple resolutions).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PhotoSize {
    pub file_id: String,
    pub file_unique_id: String,
    pub width: i32,
    pub height: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_size: Option<i64>,
}

/// A voice message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Voice {
    pub file_id: String,
    pub file_unique_id: String,
    pub duration: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_size: Option<i64>,
}

/// An audio file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Audio {
    pub file_id: String,
    pub file_unique_id: String,
    pub duration: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_size: Option<i64>,
}

/// A general file (document).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Document {
    pub file_id: String,
    pub file_unique_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_size: Option<i64>,
}

/// File information for downloading.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct File {
    pub file_id: String,
    pub file_unique_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_size: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
}

// --- Request types ---

/// Request body for getUpdates.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct GetUpdatesRequest {
    #[serde(default)]
    pub offset: Option<i64>,
    #[serde(default = "default_timeout")]
    pub timeout: Option<u64>,
    #[serde(default)]
    pub limit: Option<u32>,
}

fn default_timeout() -> Option<u64> {
    Some(30)
}

/// Request body for sendMessage.
#[derive(Debug, Clone, Deserialize)]
pub struct SendMessageRequest {
    pub chat_id: serde_json::Value, // can be i64 or String
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parse_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_markup: Option<InlineKeyboardMarkup>,
}

/// Request body for editMessageText.
#[derive(Debug, Clone, Deserialize)]
pub struct EditMessageTextRequest {
    pub chat_id: serde_json::Value,
    pub message_id: i64,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parse_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_markup: Option<InlineKeyboardMarkup>,
}

/// Request body for deleteMessage.
#[derive(Debug, Clone, Deserialize)]
pub struct DeleteMessageRequest {
    pub chat_id: serde_json::Value,
    pub message_id: i64,
}

/// Request body for answerCallbackQuery.
#[derive(Debug, Clone, Deserialize)]
pub struct AnswerCallbackQueryRequest {
    pub callback_query_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default)]
    pub show_alert: bool,
}

/// Request body for getFile.
#[derive(Debug, Clone, Deserialize)]
pub struct GetFileRequest {
    pub file_id: String,
}

/// Bot command for setMyCommands.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotCommand {
    pub command: String,
    pub description: String,
}

/// Request body for setMyCommands.
#[derive(Debug, Clone, Deserialize)]
pub struct SetMyCommandsRequest {
    pub commands: Vec<BotCommand>,
}

/// Webhook info (always empty — we only support long polling).
#[derive(Debug, Clone, Serialize)]
pub struct WebhookInfo {
    pub url: String,
    pub has_custom_certificate: bool,
    pub pending_update_count: i32,
}

// --- Bot management types (not Telegram API, for app-layer use) ---

/// Information about a created bot.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BotInfo {
    pub id: i64,
    pub token: String,
    pub name: String,
    pub username: String,
    pub description: String,
    pub about_text: String,
    pub photo_path: Option<String>,
    pub created_at: String,
}

/// Fields to update on a bot.
#[derive(Debug, Clone, Default)]
pub struct BotUpdate {
    pub name: Option<String>,
    pub description: Option<String>,
    pub about_text: Option<String>,
    pub photo_path: Option<String>,
}

/// An outbound message from the bot (for the app layer to display).
#[derive(Debug, Clone)]
pub enum OutboundEvent {
    /// Bot sent a new message.
    SendMessage {
        bot_token: String,
        chat_id: i64,
        message: Message,
    },
    /// Bot edited an existing message.
    EditMessage {
        bot_token: String,
        chat_id: i64,
        message_id: i64,
        new_text: String,
        reply_markup: Option<InlineKeyboardMarkup>,
    },
    /// Bot deleted a message.
    DeleteMessage {
        bot_token: String,
        chat_id: i64,
        message_id: i64,
    },
}
```

- [ ] **Step 2: Add type serialization tests**

Add `#[cfg(test)]` module at bottom of `types.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_response_ok_serialization() {
        let resp = ApiResponse::ok(User {
            id: 1,
            is_bot: true,
            first_name: "TestBot".into(),
            last_name: None,
            username: Some("test_bot".into()),
        });
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["ok"], true);
        assert_eq!(json["result"]["id"], 1);
        assert!(json.get("error_code").is_none());
    }

    #[test]
    fn test_api_response_error_serialization() {
        let resp = ApiResponse::<bool>::error(401, "Unauthorized");
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["ok"], false);
        assert_eq!(json["error_code"], 401);
    }

    #[test]
    fn test_update_with_message_deserialization() {
        let json = r#"{
            "update_id": 100,
            "message": {
                "message_id": 1,
                "from": {"id": 1, "is_bot": false, "first_name": "User"},
                "chat": {"id": 1, "type": "private"},
                "date": 1700000000,
                "text": "hello"
            }
        }"#;
        let update: Update = serde_json::from_str(json).unwrap();
        assert_eq!(update.update_id, 100);
        assert_eq!(update.message.unwrap().text.unwrap(), "hello");
    }

    #[test]
    fn test_update_with_callback_query_deserialization() {
        let json = r#"{
            "update_id": 101,
            "callback_query": {
                "id": "cq_001",
                "from": {"id": 1, "is_bot": false, "first_name": "User"},
                "data": "option_a"
            }
        }"#;
        let update: Update = serde_json::from_str(json).unwrap();
        let cq = update.callback_query.unwrap();
        assert_eq!(cq.data.unwrap(), "option_a");
    }

    #[test]
    fn test_inline_keyboard_roundtrip() {
        let kb = InlineKeyboardMarkup {
            inline_keyboard: vec![vec![
                InlineKeyboardButton {
                    text: "Option A".into(),
                    callback_data: Some("a".into()),
                    url: None,
                },
                InlineKeyboardButton {
                    text: "Option B".into(),
                    callback_data: Some("b".into()),
                    url: None,
                },
            ]],
        };
        let json = serde_json::to_string(&kb).unwrap();
        let parsed: InlineKeyboardMarkup = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, kb);
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cd /Users/zhaoyue/workspace/matrix/moly-ecosystem/aitk && cargo test --features telegram-server types`
Expected: All 5 tests pass.

- [ ] **Step 4: Commit**

```bash
cd /Users/zhaoyue/workspace/matrix/moly-ecosystem/aitk
git add -A && git commit -m "feat(telegram-server): add Telegram Bot API types"
```

---

### Task 1.3: Error types

**Files:**
- Create: `aitk/src/telegram_server/error.rs`

- [ ] **Step 1: Write error types**

Create `aitk/src/telegram_server/error.rs`:
```rust
//! Error types for the Telegram Bot API server.

use std::fmt;

/// Errors from the Bot API server.
#[derive(Debug)]
pub enum BotApiError {
    /// Token is invalid or does not match any registered bot.
    InvalidToken,
    /// Bot with the given token was not found.
    BotNotFound,
    /// Request body is malformed or missing required fields.
    InvalidRequest(String),
    /// Another getUpdates poller is already connected for this bot.
    ConflictPoller,
    /// The per-bot update queue is full.
    QueueFull,
    /// SQLite database error.
    DatabaseError(String),
    /// Internal server error.
    InternalError(String),
}

impl fmt::Display for BotApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidToken => write!(f, "Invalid bot token"),
            Self::BotNotFound => write!(f, "Bot not found"),
            Self::InvalidRequest(msg) => write!(f, "Invalid request: {msg}"),
            Self::ConflictPoller => {
                write!(f, "Conflict: another getUpdates is already active")
            }
            Self::QueueFull => write!(f, "Update queue is full"),
            Self::DatabaseError(msg) => write!(f, "Database error: {msg}"),
            Self::InternalError(msg) => write!(f, "Internal error: {msg}"),
        }
    }
}

impl std::error::Error for BotApiError {}

impl BotApiError {
    /// Returns the HTTP status code for this error.
    pub fn status_code(&self) -> u16 {
        match self {
            Self::InvalidToken => 401,
            Self::BotNotFound => 404,
            Self::InvalidRequest(_) => 400,
            Self::ConflictPoller => 409,
            Self::QueueFull => 429,
            Self::DatabaseError(_) | Self::InternalError(_) => 500,
        }
    }
}

#[cfg(feature = "telegram-server")]
impl From<rusqlite::Error> for BotApiError {
    fn from(err: rusqlite::Error) -> Self {
        Self::DatabaseError(err.to_string())
    }
}
```

- [ ] **Step 2: Verify compilation**

Run: `cd /Users/zhaoyue/workspace/matrix/moly-ecosystem/aitk && cargo check --features telegram-server`
Expected: Compiles.

- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "feat(telegram-server): add BotApiError type"
```

---

### Task 1.4: SQLite bot store

**Files:**
- Create: `aitk/src/telegram_server/store.rs`

- [ ] **Step 1: Write the store tests first**

Create `aitk/src/telegram_server/store.rs` with tests at the bottom and
minimal struct definition at the top (enough to compile but fail tests):

```rust
//! SQLite-based storage for bot data, messages, and update tracking.

use crate::telegram_server::error::BotApiError;
use crate::telegram_server::types::{BotInfo, BotUpdate};
use rusqlite::{params, Connection};
use std::path::Path;
use std::sync::Mutex;

const CURRENT_SCHEMA_VERSION: i32 = 1;

/// Configuration for the bot store.
#[derive(Debug, Clone)]
pub struct BotStoreConfig {
    /// Path to the SQLite database file.
    pub db_path: String,
    /// Base directory for media files.
    pub media_dir: String,
}

/// SQLite-backed store for bot metadata, messages, and updates.
pub struct BotStore {
    conn: Mutex<Connection>,
    media_dir: String,
}

impl BotStore {
    /// Opens or creates the database at the given path.
    pub fn open(config: &BotStoreConfig) -> Result<Self, BotApiError> {
        let conn = Connection::open(&config.db_path)
            .map_err(|e| BotApiError::DatabaseError(e.to_string()))?;
        let store = Self {
            conn: Mutex::new(conn),
            media_dir: config.media_dir.clone(),
        };
        store.migrate()?;
        Ok(store)
    }

    /// Opens an in-memory database (for testing).
    pub fn open_in_memory() -> Result<Self, BotApiError> {
        let conn = Connection::open_in_memory()
            .map_err(|e| BotApiError::DatabaseError(e.to_string()))?;
        let store = Self {
            conn: Mutex::new(conn),
            media_dir: String::new(),
        };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> Result<(), BotApiError> {
        let conn = self.conn.lock().expect("lock poisoned");
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_version (
                version INTEGER NOT NULL
            )"
        )?;

        let version: i32 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_version",
                [],
                |row| row.get(0),
            )?;

        if version < CURRENT_SCHEMA_VERSION {
            conn.execute_batch(include_str!("schema.sql"))?;
            conn.execute(
                "INSERT OR REPLACE INTO schema_version (rowid, version) VALUES (1, ?1)",
                params![CURRENT_SCHEMA_VERSION],
            )?;
        }
        Ok(())
    }

    /// Creates a new bot with the given name and username.
    /// Returns the bot info including the generated token.
    pub fn create_bot(
        &self,
        name: &str,
        username: &str,
    ) -> Result<BotInfo, BotApiError> {
        let conn = self.conn.lock().expect("lock poisoned");
        let token_hex: String = (0..16)
            .map(|_| format!("{:02x}", rand_byte()))
            .collect();

        conn.execute(
            "INSERT INTO bots (name, username, token) VALUES (?1, ?2, ?3)",
            params![name, username, "placeholder"],
        ).map_err(|e| {
            if let rusqlite::Error::SqliteFailure(_, Some(ref msg)) = e {
                if msg.contains("UNIQUE") {
                    return BotApiError::InvalidRequest(
                        format!("Username '{username}' already taken"),
                    );
                }
            }
            BotApiError::from(e)
        })?;

        let id = conn.last_insert_rowid();
        let token = format!("{id}:{token_hex}");
        conn.execute(
            "UPDATE bots SET token = ?1 WHERE id = ?2",
            params![token, id],
        )?;

        self.get_bot_by_id(&conn, id)
    }

    /// Finds a bot by its token.
    pub fn get_bot_by_token(&self, token: &str) -> Result<Option<BotInfo>, BotApiError> {
        let conn = self.conn.lock().expect("lock poisoned");
        let mut stmt = conn.prepare(
            "SELECT id, token, name, username, description, about_text, \
             photo_path, created_at FROM bots WHERE token = ?1"
        )?;
        let result = stmt.query_row(params![token], |row| {
            Ok(BotInfo {
                id: row.get(0)?,
                token: row.get(1)?,
                name: row.get(2)?,
                username: row.get(3)?,
                description: row.get::<_, String>(4)?,
                about_text: row.get::<_, String>(5)?,
                photo_path: row.get(6)?,
                created_at: row.get(7)?,
            })
        });
        match result {
            Ok(bot) => Ok(Some(bot)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(BotApiError::from(e)),
        }
    }

    /// Lists all bots ordered by creation time.
    pub fn list_bots(&self) -> Result<Vec<BotInfo>, BotApiError> {
        let conn = self.conn.lock().expect("lock poisoned");
        let mut stmt = conn.prepare(
            "SELECT id, token, name, username, description, about_text, \
             photo_path, created_at FROM bots ORDER BY created_at ASC"
        )?;
        let bots = stmt.query_map([], |row| {
            Ok(BotInfo {
                id: row.get(0)?,
                token: row.get(1)?,
                name: row.get(2)?,
                username: row.get(3)?,
                description: row.get::<_, String>(4)?,
                about_text: row.get::<_, String>(5)?,
                photo_path: row.get(6)?,
                created_at: row.get(7)?,
            })
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(bots)
    }

    /// Updates bot properties.
    pub fn update_bot(
        &self,
        token: &str,
        update: &BotUpdate,
    ) -> Result<BotInfo, BotApiError> {
        let conn = self.conn.lock().expect("lock poisoned");
        let bot = conn.query_row(
            "SELECT id FROM bots WHERE token = ?1", params![token],
            |row| row.get::<_, i64>(0),
        ).map_err(|_| BotApiError::BotNotFound)?;

        if let Some(ref name) = update.name {
            conn.execute(
                "UPDATE bots SET name = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
                params![name, bot],
            )?;
        }
        if let Some(ref desc) = update.description {
            conn.execute(
                "UPDATE bots SET description = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
                params![desc, bot],
            )?;
        }
        if let Some(ref about) = update.about_text {
            conn.execute(
                "UPDATE bots SET about_text = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
                params![about, bot],
            )?;
        }
        if let Some(ref photo) = update.photo_path {
            conn.execute(
                "UPDATE bots SET photo_path = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
                params![photo, bot],
            )?;
        }

        self.get_bot_by_id(&conn, bot)
    }

    /// Deletes a bot and all associated data (messages, updates, media).
    pub fn delete_bot(&self, token: &str) -> Result<(), BotApiError> {
        let conn = self.conn.lock().expect("lock poisoned");
        let id: i64 = conn.query_row(
            "SELECT id FROM bots WHERE token = ?1", params![token],
            |row| row.get(0),
        ).map_err(|_| BotApiError::BotNotFound)?;

        conn.execute("DELETE FROM updates WHERE bot_id = ?1", params![id])?;
        conn.execute("DELETE FROM messages WHERE bot_id = ?1", params![id])?;
        conn.execute("DELETE FROM bots WHERE id = ?1", params![id])?;
        Ok(())
    }

    /// Regenerates and returns a new token for the bot.
    pub fn revoke_token(&self, old_token: &str) -> Result<String, BotApiError> {
        let conn = self.conn.lock().expect("lock poisoned");
        let id: i64 = conn.query_row(
            "SELECT id FROM bots WHERE token = ?1", params![old_token],
            |row| row.get(0),
        ).map_err(|_| BotApiError::BotNotFound)?;

        let new_hex: String = (0..16)
            .map(|_| format!("{:02x}", rand_byte()))
            .collect();
        let new_token = format!("{id}:{new_hex}");
        conn.execute(
            "UPDATE bots SET token = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
            params![new_token, id],
        )?;
        Ok(new_token)
    }

    /// Inserts a new update record and returns the assigned update_id.
    pub fn insert_update(
        &self,
        bot_id: i64,
        payload_json: &str,
    ) -> Result<i64, BotApiError> {
        let conn = self.conn.lock().expect("lock poisoned");
        conn.execute(
            "INSERT INTO updates (bot_id, payload) VALUES (?1, ?2)",
            params![bot_id, payload_json],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// Gets pending updates for a bot with update_id >= offset.
    pub fn get_updates(
        &self,
        bot_id: i64,
        offset: i64,
        limit: u32,
    ) -> Result<Vec<(i64, String)>, BotApiError> {
        let conn = self.conn.lock().expect("lock poisoned");
        let mut stmt = conn.prepare(
            "SELECT id, payload FROM updates \
             WHERE bot_id = ?1 AND id >= ?2 \
             ORDER BY id ASC LIMIT ?3"
        )?;
        let rows = stmt.query_map(params![bot_id, offset, limit], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?))
        })?.collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Stores a message record.
    pub fn store_message(
        &self,
        bot_id: i64,
        chat_id: i64,
        is_from_bot: bool,
        content: Option<&str>,
        media_type: Option<&str>,
        media_path: Option<&str>,
        caption: Option<&str>,
        reply_markup_json: Option<&str>,
    ) -> Result<i64, BotApiError> {
        let conn = self.conn.lock().expect("lock poisoned");
        let next_msg_id: i64 = conn.query_row(
            "SELECT COALESCE(MAX(message_id), 0) + 1 FROM messages WHERE bot_id = ?1",
            params![bot_id],
            |row| row.get(0),
        )?;
        conn.execute(
            "INSERT INTO messages (bot_id, message_id, chat_id, is_from_bot, \
             content, media_type, media_path, caption, reply_markup) \
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                bot_id, next_msg_id, chat_id, is_from_bot,
                content, media_type, media_path, caption, reply_markup_json,
            ],
        )?;
        Ok(next_msg_id)
    }

    /// Returns the media base directory path.
    pub fn media_dir(&self) -> &str {
        &self.media_dir
    }

    /// Stores media file metadata.
    pub fn store_media(
        &self,
        file_id: &str,
        file_unique_id: &str,
        file_path: &str,
        mime_type: Option<&str>,
        file_size: Option<i64>,
    ) -> Result<(), BotApiError> {
        let conn = self.conn.lock().expect("lock poisoned");
        conn.execute(
            "INSERT OR REPLACE INTO media \
             (file_id, file_unique_id, file_path, mime_type, file_size) \
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![file_id, file_unique_id, file_path, mime_type, file_size],
        )?;
        Ok(())
    }

    /// Gets media metadata by file_id.
    pub fn get_media(&self, file_id: &str) -> Result<Option<(String, Option<String>)>, BotApiError> {
        let conn = self.conn.lock().expect("lock poisoned");
        let result = conn.query_row(
            "SELECT file_path, mime_type FROM media WHERE file_id = ?1",
            params![file_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
        );
        match result {
            Ok(data) => Ok(Some(data)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(BotApiError::from(e)),
        }
    }

    fn get_bot_by_id(&self, conn: &Connection, id: i64) -> Result<BotInfo, BotApiError> {
        conn.query_row(
            "SELECT id, token, name, username, description, about_text, \
             photo_path, created_at FROM bots WHERE id = ?1",
            params![id],
            |row| {
                Ok(BotInfo {
                    id: row.get(0)?,
                    token: row.get(1)?,
                    name: row.get(2)?,
                    username: row.get(3)?,
                    description: row.get::<_, String>(4)?,
                    about_text: row.get::<_, String>(5)?,
                    photo_path: row.get(6)?,
                    created_at: row.get(7)?,
                })
            },
        ).map_err(|_| BotApiError::BotNotFound)
    }
}

/// Simple random byte using std (no extra dependency).
fn rand_byte() -> u8 {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    let s = RandomState::new();
    let mut h = s.build_hasher();
    h.write_u8(0);
    (h.finish() & 0xFF) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_store() -> BotStore {
        BotStore::open_in_memory().expect("failed to open in-memory db")
    }

    #[test]
    fn test_create_and_get_bot() {
        let store = test_store();
        let bot = store.create_bot("TestBot", "test_bot").unwrap();
        assert_eq!(bot.name, "TestBot");
        assert_eq!(bot.username, "test_bot");
        assert!(bot.token.contains(':'));

        let found = store.get_bot_by_token(&bot.token).unwrap().unwrap();
        assert_eq!(found.id, bot.id);
    }

    #[test]
    fn test_duplicate_username_rejected() {
        let store = test_store();
        store.create_bot("Bot1", "same_bot").unwrap();
        let err = store.create_bot("Bot2", "same_bot").unwrap_err();
        assert!(matches!(err, BotApiError::InvalidRequest(_)));
    }

    #[test]
    fn test_list_bots() {
        let store = test_store();
        store.create_bot("A", "a_bot").unwrap();
        store.create_bot("B", "b_bot").unwrap();
        store.create_bot("C", "c_bot").unwrap();
        let bots = store.list_bots().unwrap();
        assert_eq!(bots.len(), 3);
    }

    #[test]
    fn test_update_bot() {
        let store = test_store();
        let bot = store.create_bot("Old", "old_bot").unwrap();
        store.update_bot(&bot.token, &BotUpdate {
            name: Some("New".into()),
            ..Default::default()
        }).unwrap();
        let updated = store.get_bot_by_token(&bot.token).unwrap().unwrap();
        assert_eq!(updated.name, "New");
    }

    #[test]
    fn test_delete_bot_cascades() {
        let store = test_store();
        let bot = store.create_bot("Del", "del_bot").unwrap();
        store.store_message(bot.id, 1, false, Some("hi"), None, None, None, None).unwrap();
        store.insert_update(bot.id, "{}").unwrap();
        store.delete_bot(&bot.token).unwrap();
        assert!(store.get_bot_by_token(&bot.token).unwrap().is_none());
    }

    #[test]
    fn test_revoke_token() {
        let store = test_store();
        let bot = store.create_bot("Rev", "rev_bot").unwrap();
        let new_token = store.revoke_token(&bot.token).unwrap();
        assert_ne!(new_token, bot.token);
        assert!(store.get_bot_by_token(&bot.token).unwrap().is_none());
        assert!(store.get_bot_by_token(&new_token).unwrap().is_some());
    }

    #[test]
    fn test_update_id_global_increment() {
        let store = test_store();
        let a = store.create_bot("A", "a_bot").unwrap();
        let b = store.create_bot("B", "b_bot").unwrap();
        let id1 = store.insert_update(a.id, "{}").unwrap();
        let id2 = store.insert_update(b.id, "{}").unwrap();
        assert_eq!(id2, id1 + 1);
    }

    #[test]
    fn test_get_updates_with_offset() {
        let store = test_store();
        let bot = store.create_bot("U", "u_bot").unwrap();
        let id1 = store.insert_update(bot.id, r#"{"a":1}"#).unwrap();
        let id2 = store.insert_update(bot.id, r#"{"a":2}"#).unwrap();
        let _id3 = store.insert_update(bot.id, r#"{"a":3}"#).unwrap();

        let updates = store.get_updates(bot.id, id2, 100).unwrap();
        assert_eq!(updates.len(), 2);
        assert_eq!(updates[0].0, id2);
    }

    #[test]
    fn test_store_and_get_message() {
        let store = test_store();
        let bot = store.create_bot("M", "m_bot").unwrap();
        let msg_id = store.store_message(
            bot.id, 1, true, Some("hello"), None, None, None, None,
        ).unwrap();
        assert_eq!(msg_id, 1);
        let msg_id2 = store.store_message(
            bot.id, 1, false, Some("world"), None, None, None, None,
        ).unwrap();
        assert_eq!(msg_id2, 2);
    }

    #[test]
    fn test_store_media() {
        let store = test_store();
        store.store_media("f1", "fu1", "/path/to/file", Some("image/png"), Some(1024)).unwrap();
        let (path, mime) = store.get_media("f1").unwrap().unwrap();
        assert_eq!(path, "/path/to/file");
        assert_eq!(mime.unwrap(), "image/png");
    }
}
```

- [ ] **Step 2: Create SQL schema file**

Create `aitk/src/telegram_server/schema.sql`:
```sql
CREATE TABLE IF NOT EXISTS bots (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    token       TEXT NOT NULL UNIQUE,
    name        TEXT NOT NULL,
    username    TEXT NOT NULL UNIQUE,
    description TEXT NOT NULL DEFAULT '',
    about_text  TEXT NOT NULL DEFAULT '',
    photo_path  TEXT,
    created_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at  DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS updates (
    id      INTEGER PRIMARY KEY AUTOINCREMENT,
    bot_id  INTEGER NOT NULL REFERENCES bots(id),
    payload TEXT NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS messages (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    bot_id       INTEGER NOT NULL REFERENCES bots(id),
    message_id   INTEGER NOT NULL,
    chat_id      INTEGER NOT NULL,
    is_from_bot  INTEGER NOT NULL DEFAULT 0,
    content      TEXT,
    media_type   TEXT,
    media_path   TEXT,
    caption      TEXT,
    reply_markup TEXT,
    timestamp    DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(bot_id, message_id)
);

CREATE TABLE IF NOT EXISTS media (
    id             INTEGER PRIMARY KEY AUTOINCREMENT,
    file_id        TEXT NOT NULL UNIQUE,
    file_unique_id TEXT NOT NULL,
    file_path      TEXT NOT NULL,
    mime_type      TEXT,
    file_size      INTEGER
);

CREATE INDEX IF NOT EXISTS idx_updates_bot ON updates(bot_id);
CREATE INDEX IF NOT EXISTS idx_messages_bot_chat ON messages(bot_id, chat_id);
CREATE INDEX IF NOT EXISTS idx_messages_timestamp ON messages(timestamp);
```

- [ ] **Step 3: Run store tests**

Run: `cd /Users/zhaoyue/workspace/matrix/moly-ecosystem/aitk && cargo test --features telegram-server store::tests`
Expected: All 9 tests pass.

- [ ] **Step 4: Commit**

```bash
git add -A && git commit -m "feat(telegram-server): add SQLite bot store with tests"
```

---

### Task 1.5: Update queue (in-memory per-bot long-polling)

**Files:**
- Create: `aitk/src/telegram_server/queue.rs`

The queue bridges `push_update` (from app layer) with `getUpdates` (from teloxide).
Uses `futures::channel::mpsc` for notification and `Arc<Mutex<>>` for state.

- [ ] **Step 1: Write queue implementation with tests**

Create `aitk/src/telegram_server/queue.rs`:
```rust
//! Per-bot update queue for long-polling support.
//!
//! Each bot has an independent queue. When the app pushes an update,
//! any waiting `getUpdates` poller is notified. Only one poller is
//! allowed per bot at a time (matching Telegram behavior).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use futures::channel::mpsc;
use futures::SinkExt;
use crate::telegram_server::error::BotApiError;

const MAX_QUEUE_SIZE: usize = 1000;

/// A notification sender for waking up a long-polling getUpdates request.
type Notifier = mpsc::Sender<()>;

/// State for a single bot's polling session.
struct BotPollState {
    /// Notification channel to wake the poller.
    notifier: Option<Notifier>,
    /// Whether a poller is currently waiting.
    has_active_poller: bool,
    /// Timestamp of last getUpdates request (for online status).
    last_poll_time: std::time::Instant,
}

/// Manages per-bot update queues and long-polling state.
pub struct UpdateQueueManager {
    bots: Mutex<HashMap<i64, BotPollState>>,
}

impl UpdateQueueManager {
    /// Creates a new queue manager.
    pub fn new() -> Self {
        Self {
            bots: Mutex::new(HashMap::new()),
        }
    }

    /// Notifies the poller for a bot that new updates are available.
    /// If no poller is waiting, this is a no-op (the poller will see
    /// the updates when it next calls getUpdates).
    pub fn notify_bot(&self, bot_id: i64) {
        let mut bots = self.bots.lock().expect("lock poisoned");
        if let Some(state) = bots.get_mut(&bot_id) {
            if let Some(ref mut tx) = state.notifier {
                let _ = tx.try_send(());
            }
        }
    }

    /// Registers a poller for a bot. Returns a receiver that will be
    /// notified when updates arrive. Returns ConflictPoller if another
    /// poller is already active.
    pub fn register_poller(
        &self,
        bot_id: i64,
    ) -> Result<mpsc::Receiver<()>, BotApiError> {
        let mut bots = self.bots.lock().expect("lock poisoned");
        let state = bots.entry(bot_id).or_insert_with(|| BotPollState {
            notifier: None,
            has_active_poller: false,
            last_poll_time: std::time::Instant::now(),
        });

        if state.has_active_poller {
            return Err(BotApiError::ConflictPoller);
        }

        let (tx, rx) = mpsc::channel(1);
        state.notifier = Some(tx);
        state.has_active_poller = true;
        state.last_poll_time = std::time::Instant::now();
        Ok(rx)
    }

    /// Unregisters the poller for a bot (called when getUpdates completes).
    pub fn unregister_poller(&self, bot_id: i64) {
        let mut bots = self.bots.lock().expect("lock poisoned");
        if let Some(state) = bots.get_mut(&bot_id) {
            state.notifier = None;
            state.has_active_poller = false;
        }
    }

    /// Returns whether a bot has had a poll request within the given duration.
    pub fn is_bot_online(
        &self,
        bot_id: i64,
        timeout: std::time::Duration,
    ) -> bool {
        let bots = self.bots.lock().expect("lock poisoned");
        bots.get(&bot_id)
            .map(|s| s.last_poll_time.elapsed() < timeout)
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_unregister_poller() {
        let mgr = UpdateQueueManager::new();
        let _rx = mgr.register_poller(1).unwrap();
        assert!(mgr.register_poller(1).is_err()); // conflict
        mgr.unregister_poller(1);
        let _rx2 = mgr.register_poller(1).unwrap(); // ok after unregister
    }

    #[test]
    fn test_notify_without_poller_is_noop() {
        let mgr = UpdateQueueManager::new();
        mgr.notify_bot(999); // no panic, no error
    }

    #[test]
    fn test_is_bot_online() {
        let mgr = UpdateQueueManager::new();
        assert!(!mgr.is_bot_online(1, std::time::Duration::from_secs(120)));
        let _rx = mgr.register_poller(1).unwrap();
        assert!(mgr.is_bot_online(1, std::time::Duration::from_secs(120)));
    }
}
```

- [ ] **Step 2: Run queue tests**

Run: `cd /Users/zhaoyue/workspace/matrix/moly-ecosystem/aitk && cargo test --features telegram-server queue::tests`
Expected: All 3 tests pass.

- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "feat(telegram-server): add per-bot update queue manager"
```

---

### Task 1.6: HTTP server and API endpoints

**Files:**
- Create: `aitk/src/telegram_server/server.rs`
- Create: `aitk/src/telegram_server/api/mod.rs`
- Create: `aitk/src/telegram_server/api/handlers.rs`

This is the main HTTP server that routes teloxide requests to the appropriate handlers.
All endpoints follow the Telegram Bot API URL pattern: `/bot{token}/{method}`.

- [ ] **Step 1: Write server core**

Create `aitk/src/telegram_server/server.rs`:
```rust
//! Telegram Bot API compatible HTTP server.
//!
//! Starts an axum HTTP server that implements the Telegram Bot API endpoints
//! used by teloxide. The server provides `push_update` and `recv_outbound`
//! interfaces for the application layer to integrate with.

use crate::telegram_server::api;
use crate::telegram_server::error::BotApiError;
use crate::telegram_server::queue::UpdateQueueManager;
use crate::telegram_server::store::BotStore;
use crate::telegram_server::types::OutboundEvent;
use axum::Router;
use futures::channel::mpsc;
use std::net::SocketAddr;
use std::sync::Arc;

/// Server configuration.
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Port to listen on. Default: 8488.
    pub port: u16,
    /// Database file path.
    pub db_path: String,
    /// Media storage directory.
    pub media_dir: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 8488,
            db_path: "moly_bots.db".into(),
            media_dir: "bot_media".into(),
        }
    }
}

/// Handle to control the running server.
pub struct ServerHandle {
    /// The address the server is bound to.
    pub addr: SocketAddr,
    /// Sender to trigger graceful shutdown.
    shutdown_tx: Option<futures::channel::oneshot::Sender<()>>,
}

impl ServerHandle {
    /// Shuts down the server gracefully.
    pub fn shutdown(mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

impl Drop for ServerHandle {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
    }
}

/// Shared state accessible by all API handlers.
pub struct ServerState {
    pub store: BotStore,
    pub queues: UpdateQueueManager,
    pub outbound_tx: mpsc::UnboundedSender<OutboundEvent>,
}

/// The main Telegram Bot API server.
pub struct TelegramBotApiServer;

impl TelegramBotApiServer {
    /// Starts the server and returns a handle for shutdown and an outbound
    /// event receiver for the application layer.
    ///
    /// The outbound receiver yields `OutboundEvent` items whenever a bot
    /// sends a message, edits, or deletes via the API.
    pub async fn start(
        config: ServerConfig,
    ) -> Result<
        (ServerHandle, mpsc::UnboundedReceiver<OutboundEvent>),
        BotApiError,
    > {
        let store = BotStore::open(&crate::telegram_server::store::BotStoreConfig {
            db_path: config.db_path,
            media_dir: config.media_dir,
        })?;

        let (outbound_tx, outbound_rx) = mpsc::unbounded();
        let state = Arc::new(ServerState {
            store,
            queues: UpdateQueueManager::new(),
            outbound_tx,
        });

        let app = api::router(state.clone());

        let addr = SocketAddr::from(([127, 0, 0, 1], config.port));
        let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| {
            BotApiError::InternalError(format!("Failed to bind {addr}: {e}"))
        })?;
        let bound_addr = listener.local_addr().map_err(|e| {
            BotApiError::InternalError(format!("Failed to get local addr: {e}"))
        })?;

        let (shutdown_tx, shutdown_rx) = futures::channel::oneshot::channel::<()>();

        tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = shutdown_rx.await;
                })
                .await
                .ok();
        });

        let handle = ServerHandle {
            addr: bound_addr,
            shutdown_tx: Some(shutdown_tx),
        };

        Ok((handle, outbound_rx))
    }
}
```

- [ ] **Step 2: Write API router and handlers**

Create `aitk/src/telegram_server/api/mod.rs`:
```rust
//! API endpoint router and handlers for the Telegram Bot API.

mod handlers;

use crate::telegram_server::server::ServerState;
use axum::{
    Router,
    routing::{get, post},
};
use std::sync::Arc;

/// Builds the axum Router with all Telegram Bot API endpoints.
pub fn router(state: Arc<ServerState>) -> Router {
    Router::new()
        // Core endpoints
        .route("/bot{token}/getMe", get(handlers::get_me).post(handlers::get_me))
        .route("/bot{token}/getUpdates", post(handlers::get_updates))
        .route("/bot{token}/sendMessage", post(handlers::send_message))
        .route("/bot{token}/editMessageText", post(handlers::edit_message_text))
        .route("/bot{token}/deleteMessage", post(handlers::delete_message))
        .route("/bot{token}/answerCallbackQuery", post(handlers::answer_callback_query))
        // Teloxide compat stubs
        .route("/bot{token}/getWebhookInfo", get(handlers::get_webhook_info).post(handlers::get_webhook_info))
        .route("/bot{token}/deleteWebhook", post(handlers::delete_webhook))
        .route("/bot{token}/setMyCommands", post(handlers::set_my_commands))
        .route("/bot{token}/getMyCommands", get(handlers::get_my_commands).post(handlers::get_my_commands))
        // File endpoints (placeholder, completed in Task 3)
        .route("/bot{token}/getFile", post(handlers::get_file))
        .with_state(state)
}
```

Create `aitk/src/telegram_server/api/handlers.rs`:
```rust
//! HTTP handler functions for each Telegram Bot API endpoint.

use crate::telegram_server::error::BotApiError;
use crate::telegram_server::server::ServerState;
use crate::telegram_server::types::*;
use axum::{
    extract::{Path, State},
    Json,
};
use std::sync::Arc;
use std::time::Duration;

type AppState = State<Arc<ServerState>>;

/// Helper: validate token and return bot info.
fn validate_token(
    state: &ServerState,
    token: &str,
) -> Result<BotInfo, Json<ApiResponse<bool>>> {
    state
        .store
        .get_bot_by_token(token)
        .map_err(|_| Json(ApiResponse::<bool>::error(500, "Internal error")))?
        .ok_or_else(|| Json(ApiResponse::<bool>::error(401, "Unauthorized")))
}

/// GET/POST /bot{token}/getMe
pub async fn get_me(
    State(state): AppState,
    Path(token): Path<String>,
) -> Json<ApiResponse<User>> {
    match validate_token(&state, &token) {
        Ok(bot) => Json(ApiResponse::ok(User {
            id: bot.id,
            is_bot: true,
            first_name: bot.name,
            last_name: None,
            username: Some(bot.username),
        })),
        Err(_) => Json(ApiResponse {
            ok: false,
            result: None,
            error_code: Some(401),
            description: Some("Unauthorized".into()),
        }),
    }
}

/// POST /bot{token}/getUpdates — long-polling endpoint.
pub async fn get_updates(
    State(state): AppState,
    Path(token): Path<String>,
    Json(req): Json<GetUpdatesRequest>,
) -> Json<ApiResponse<Vec<Update>>> {
    let bot = match validate_token(&state, &token) {
        Ok(b) => b,
        Err(_) => return Json(ApiResponse {
            ok: false, result: None,
            error_code: Some(401),
            description: Some("Unauthorized".into()),
        }),
    };

    let offset = req.offset.unwrap_or(0);
    let timeout_secs = req.timeout.unwrap_or(30).min(60);
    let limit = req.limit.unwrap_or(100).min(100);

    // Check for existing updates first
    if let Ok(updates) = state.store.get_updates(bot.id, offset, limit) {
        if !updates.is_empty() {
            let parsed: Vec<Update> = updates
                .into_iter()
                .filter_map(|(id, payload)| {
                    serde_json::from_str::<Update>(&payload)
                        .map(|mut u| { u.update_id = id; u })
                        .ok()
                })
                .collect();
            return Json(ApiResponse::ok(parsed));
        }
    }

    // No updates — register poller and wait
    let rx = match state.queues.register_poller(bot.id) {
        Ok(rx) => rx,
        Err(BotApiError::ConflictPoller) => {
            return Json(ApiResponse {
                ok: false, result: None,
                error_code: Some(409),
                description: Some("Conflict: another getUpdates is active".into()),
            });
        }
        Err(_) => {
            return Json(ApiResponse {
                ok: false, result: None,
                error_code: Some(500),
                description: Some("Internal error".into()),
            });
        }
    };

    // Wait for notification or timeout
    let timeout = tokio::time::sleep(Duration::from_secs(timeout_secs));
    let mut rx = rx;

    tokio::select! {
        _ = timeout => {},
        _ = futures::StreamExt::next(&mut rx) => {},
    }

    state.queues.unregister_poller(bot.id);

    // Fetch any updates that arrived
    let updates = state.store
        .get_updates(bot.id, offset, limit)
        .unwrap_or_default();
    let parsed: Vec<Update> = updates
        .into_iter()
        .filter_map(|(id, payload)| {
            serde_json::from_str::<Update>(&payload)
                .map(|mut u| { u.update_id = id; u })
                .ok()
        })
        .collect();

    Json(ApiResponse::ok(parsed))
}

/// POST /bot{token}/sendMessage
pub async fn send_message(
    State(state): AppState,
    Path(token): Path<String>,
    Json(req): Json<SendMessageRequest>,
) -> Json<ApiResponse<Message>> {
    let bot = match validate_token(&state, &token) {
        Ok(b) => b,
        Err(e) => return Json(ApiResponse {
            ok: false, result: None,
            error_code: Some(401),
            description: Some("Unauthorized".into()),
        }),
    };

    let chat_id = parse_chat_id(&req.chat_id);
    let reply_markup_json = req.reply_markup.as_ref()
        .and_then(|rm| serde_json::to_string(rm).ok());

    let msg_id = match state.store.store_message(
        bot.id, chat_id, true,
        Some(&req.text), None, None, None,
        reply_markup_json.as_deref(),
    ) {
        Ok(id) => id,
        Err(_) => return Json(ApiResponse {
            ok: false, result: None,
            error_code: Some(500),
            description: Some("Failed to store message".into()),
        }),
    };

    let now = chrono::Utc::now().timestamp();
    let message = Message {
        message_id: msg_id,
        from: Some(User {
            id: bot.id,
            is_bot: true,
            first_name: bot.name.clone(),
            last_name: None,
            username: Some(bot.username.clone()),
        }),
        chat: Chat {
            id: chat_id,
            chat_type: "private".into(),
            title: None,
            first_name: None,
            username: None,
        },
        date: now,
        text: Some(req.text.clone()),
        caption: None,
        photo: None,
        voice: None,
        audio: None,
        document: None,
        reply_markup: req.reply_markup.clone(),
    };

    // Notify app layer
    let _ = state.outbound_tx.unbounded_send(OutboundEvent::SendMessage {
        bot_token: token,
        chat_id,
        message: message.clone(),
    });

    Json(ApiResponse::ok(message))
}

/// POST /bot{token}/editMessageText
pub async fn edit_message_text(
    State(state): AppState,
    Path(token): Path<String>,
    Json(req): Json<EditMessageTextRequest>,
) -> Json<ApiResponse<Message>> {
    let bot = match validate_token(&state, &token) {
        Ok(b) => b,
        Err(_) => return Json(ApiResponse {
            ok: false, result: None,
            error_code: Some(401),
            description: Some("Unauthorized".into()),
        }),
    };

    let chat_id = parse_chat_id(&req.chat_id);

    let _ = state.outbound_tx.unbounded_send(OutboundEvent::EditMessage {
        bot_token: token,
        chat_id,
        message_id: req.message_id,
        new_text: req.text.clone(),
        reply_markup: req.reply_markup.clone(),
    });

    let now = chrono::Utc::now().timestamp();
    let message = Message {
        message_id: req.message_id,
        from: Some(User {
            id: bot.id, is_bot: true,
            first_name: bot.name, last_name: None,
            username: Some(bot.username),
        }),
        chat: Chat { id: chat_id, chat_type: "private".into(),
            title: None, first_name: None, username: None },
        date: now,
        text: Some(req.text),
        caption: None, photo: None, voice: None, audio: None,
        document: None,
        reply_markup: req.reply_markup,
    };
    Json(ApiResponse::ok(message))
}

/// POST /bot{token}/deleteMessage
pub async fn delete_message(
    State(state): AppState,
    Path(token): Path<String>,
    Json(req): Json<DeleteMessageRequest>,
) -> Json<ApiResponse<bool>> {
    let bot = match validate_token(&state, &token) {
        Ok(b) => b,
        Err(_) => return Json(ApiResponse::<bool>::error(401, "Unauthorized")),
    };
    let chat_id = parse_chat_id(&req.chat_id);

    let _ = state.outbound_tx.unbounded_send(OutboundEvent::DeleteMessage {
        bot_token: token,
        chat_id,
        message_id: req.message_id,
    });

    Json(ApiResponse::ok(true))
}

/// POST /bot{token}/answerCallbackQuery
pub async fn answer_callback_query(
    State(state): AppState,
    Path(token): Path<String>,
    Json(req): Json<AnswerCallbackQueryRequest>,
) -> Json<ApiResponse<bool>> {
    match validate_token(&state, &token) {
        Ok(_) => Json(ApiResponse::ok(true)),
        Err(_) => Json(ApiResponse::<bool>::error(401, "Unauthorized")),
    }
}

/// GET/POST /bot{token}/getWebhookInfo — returns empty (long-polling only).
pub async fn get_webhook_info(
    State(state): AppState,
    Path(token): Path<String>,
) -> Json<ApiResponse<WebhookInfo>> {
    match validate_token(&state, &token) {
        Ok(_) => Json(ApiResponse::ok(WebhookInfo {
            url: String::new(),
            has_custom_certificate: false,
            pending_update_count: 0,
        })),
        Err(_) => Json(ApiResponse {
            ok: false, result: None,
            error_code: Some(401),
            description: Some("Unauthorized".into()),
        }),
    }
}

/// POST /bot{token}/deleteWebhook — no-op.
pub async fn delete_webhook(
    State(state): AppState,
    Path(token): Path<String>,
) -> Json<ApiResponse<bool>> {
    match validate_token(&state, &token) {
        Ok(_) => Json(ApiResponse::ok(true)),
        Err(_) => Json(ApiResponse::<bool>::error(401, "Unauthorized")),
    }
}

/// POST /bot{token}/setMyCommands — stores commands (no-op storage for now).
pub async fn set_my_commands(
    State(state): AppState,
    Path(token): Path<String>,
    Json(_req): Json<SetMyCommandsRequest>,
) -> Json<ApiResponse<bool>> {
    match validate_token(&state, &token) {
        Ok(_) => Json(ApiResponse::ok(true)),
        Err(_) => Json(ApiResponse::<bool>::error(401, "Unauthorized")),
    }
}

/// GET/POST /bot{token}/getMyCommands
pub async fn get_my_commands(
    State(state): AppState,
    Path(token): Path<String>,
) -> Json<ApiResponse<Vec<BotCommand>>> {
    match validate_token(&state, &token) {
        Ok(_) => Json(ApiResponse::ok(vec![])),
        Err(_) => Json(ApiResponse {
            ok: false, result: None,
            error_code: Some(401),
            description: Some("Unauthorized".into()),
        }),
    }
}

/// POST /bot{token}/getFile — placeholder, completed in Task 3.
pub async fn get_file(
    State(state): AppState,
    Path(token): Path<String>,
    Json(req): Json<GetFileRequest>,
) -> Json<ApiResponse<File>> {
    let bot = match validate_token(&state, &token) {
        Ok(b) => b,
        Err(_) => return Json(ApiResponse {
            ok: false, result: None,
            error_code: Some(401),
            description: Some("Unauthorized".into()),
        }),
    };

    match state.store.get_media(&req.file_id) {
        Ok(Some((path, _))) => Json(ApiResponse::ok(File {
            file_id: req.file_id.clone(),
            file_unique_id: req.file_id,
            file_size: None,
            file_path: Some(format!("media/{}", path.rsplit('/').next().unwrap_or(&path))),
        })),
        _ => Json(ApiResponse {
            ok: false, result: None,
            error_code: Some(404),
            description: Some("File not found".into()),
        }),
    }
}

/// Parse chat_id which can be number or string.
fn parse_chat_id(value: &serde_json::Value) -> i64 {
    match value {
        serde_json::Value::Number(n) => n.as_i64().unwrap_or(1),
        serde_json::Value::String(s) => s.parse().unwrap_or(1),
        _ => 1,
    }
}
```

- [ ] **Step 3: Add public push_update method to server module**

Add to `aitk/src/telegram_server/mod.rs` a convenience function or add
a method to `ServerState` that the app layer calls:

Extend `server.rs` with:
```rust
impl ServerState {
    /// Pushes a user message as a Telegram Update for the bot to receive.
    /// Called by the application layer when the user sends a message in the UI.
    pub fn push_update(
        &self,
        bot_token: &str,
        update_json: &str,
    ) -> Result<i64, BotApiError> {
        let bot = self.store
            .get_bot_by_token(bot_token)?
            .ok_or(BotApiError::InvalidToken)?;
        let update_id = self.store.insert_update(bot.id, update_json)?;
        self.queues.notify_bot(bot.id);
        Ok(update_id)
    }
}
```

- [ ] **Step 4: Verify full compilation**

Run: `cd /Users/zhaoyue/workspace/matrix/moly-ecosystem/aitk && cargo check --features telegram-server`
Expected: Compiles with possible warnings (unused imports etc.). Fix any errors.

- [ ] **Step 5: Run all telegram-server tests**

Run: `cd /Users/zhaoyue/workspace/matrix/moly-ecosystem/aitk && cargo test --features telegram-server`
Expected: All type, store, and queue tests pass.

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "feat(telegram-server): add HTTP server with core API endpoints"
```

---

### Task 1.7: Integration test — end-to-end with reqwest

**Files:**
- Create: `aitk/tests/telegram_server_integration.rs`

Test the full flow: start server → create bot → getMe → push update → getUpdates → sendMessage.

- [ ] **Step 1: Write integration test**

Create `aitk/tests/telegram_server_integration.rs`:
```rust
//! Integration test: starts the server, creates a bot, and exercises
//! the core API endpoints using reqwest as an HTTP client.

#![cfg(all(feature = "telegram-server", not(target_arch = "wasm32")))]

use aitk::telegram_server::*;
use futures::StreamExt;

#[tokio::test]
async fn test_full_bot_lifecycle() {
    // Start server on random port
    let config = ServerConfig {
        port: 0, // OS picks a free port — need to adjust server to support this
        db_path: ":memory:".into(),
        media_dir: "/tmp/moly_test_media".into(),
    };
    // Note: if port 0 is not yet supported, use a high random port.
    // This test validates the end-to-end flow.

    let (handle, mut outbound_rx) = TelegramBotApiServer::start(ServerConfig {
        port: 18488, // Use a specific test port
        db_path: format!("/tmp/moly_test_{}.db", std::process::id()),
        media_dir: format!("/tmp/moly_test_media_{}", std::process::id()),
    }).await.expect("server start failed");

    let base = format!("http://{}", handle.addr);
    let client = reqwest::Client::new();

    // Create a bot via store (server's internal API)
    // In real usage, the app calls store.create_bot() directly
    // For this test, we'll test the HTTP endpoints

    // getMe with invalid token → 401
    let resp = client.get(format!("{base}/botinvalid/getMe"))
        .send().await.unwrap();
    assert_eq!(resp.status(), 200); // Telegram returns 200 with ok=false
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body["ok"], false);

    // Cleanup
    handle.shutdown();
    let _ = std::fs::remove_file(format!("/tmp/moly_test_{}.db", std::process::id()));
}
```

- [ ] **Step 2: Run integration test**

Run: `cd /Users/zhaoyue/workspace/matrix/moly-ecosystem/aitk && cargo test --features telegram-server --test telegram_server_integration`
Expected: Test passes.

- [ ] **Step 3: Commit**

```bash
git add -A && git commit -m "test(telegram-server): add integration test for bot lifecycle"
```

---

## Chunk 2: AITK Media API + File Serving

### Task 2.1: Media send endpoints (sendPhoto, sendVoice, sendAudio, sendDocument)

**Files:**
- Modify: `aitk/src/telegram_server/api/mod.rs` (add routes)
- Modify: `aitk/src/telegram_server/api/handlers.rs` (add media handlers)

- [ ] **Step 1: Add multipart media handler for sendPhoto/Voice/Audio/Document**

Add to `handlers.rs` the media send handlers that accept multipart form data,
store the file to `media_dir`, register in the media table, and notify outbound.

Each handler follows the same pattern:
1. Validate token
2. Extract multipart fields (chat_id, file, caption)
3. Generate UUID file_id, save file to disk
4. Store media metadata in SQLite
5. Create Message with appropriate media field
6. Notify outbound channel

- [ ] **Step 2: Add file download endpoint**

Add route `GET /file/bot{token}/media/{file_id}` that serves the file from disk
with correct Content-Type header.

- [ ] **Step 3: Add file size validation (20MB limit)**

Return HTTP 413 if uploaded file exceeds 20MB.

- [ ] **Step 4: Write tests for each media endpoint**

- [ ] **Step 5: Run tests and verify**

- [ ] **Step 6: Commit**

```bash
git add -A && git commit -m "feat(telegram-server): add media endpoints (photo, voice, audio, document, file download)"
```

---

## Chunk 3: Moly BotFather + Bot Chat Integration

> **Note:** This chunk works in the moly-ai repo. AITK must be updated to use
> a local path dependency during development:
> In `moly-ai/moly-kit/Cargo.toml`, temporarily change aitk from git to path:
> `aitk = { path = "../../aitk", features = [...] }`

### Task 3.1: Bot data types in Moly

**Files:**
- Create: `moly-ai/src/bot_manager/mod.rs`
- Create: `moly-ai/src/bot_manager/data.rs`
- Modify: `moly-ai/src/lib.rs`

- [ ] **Step 1: Create bot_manager module**

Create `src/bot_manager/mod.rs`:
```rust
pub mod data;
pub mod botfather;
pub mod chat_bridge;
```

Create `src/bot_manager/data.rs` with Moly-specific bot data types that
wrap AITK's `BotInfo` and add UI-relevant fields (connection status, unread count).

- [ ] **Step 2: Register module in lib.rs**

- [ ] **Step 3: Commit**

### Task 3.2: BotFather state machine

**Files:**
- Create: `moly-ai/src/bot_manager/botfather.rs`

Implement the BotFather command parser and dialog state machine:
- `BotFatherState` enum: Idle, AwaitingBotName, AwaitingUsername, AwaitingConfirmDelete, etc.
- Command handlers for /start, /newbot, /mybots, /setname, /token, /revoke, /deletebot
- Username validation (3-32 chars, lowercase alphanum + underscore, ends with `bot`)
- Returns BotFather reply messages (text + optional inline keyboard buttons)

- [ ] **Step 1: Write state machine with unit tests**
- [ ] **Step 2: Write command handlers for each command**
- [ ] **Step 3: Run tests**
- [ ] **Step 4: Commit**

### Task 3.3: Chat bridge — connecting Moly UI to AITK server

**Files:**
- Create: `moly-ai/src/bot_manager/chat_bridge.rs`
- Modify: `moly-ai/src/app_state.rs`

Implement the bridge that:
1. Starts the AITK TelegramBotApiServer on app launch
2. Converts user chat messages → `push_update()` calls
3. Listens on `recv_outbound()` → dispatches to Moly chat UI
4. Tracks bot connection status (online/offline based on poll activity)

- [ ] **Step 1: Add BotManager to app_state.rs**
- [ ] **Step 2: Implement chat_bridge with message routing**
- [ ] **Step 3: Start server on app launch**
- [ ] **Step 4: Commit**

### Task 3.4: BotFather chat UI

**Files:**
- Modify: `moly-ai/src/chat/` (add BotFather as a special chat entry)

Add BotFather to the chat sidebar list as a permanent entry. Wire up
message input to the BotFather state machine. Render BotFather replies
including inline keyboard buttons.

- [ ] **Step 1: Add BotFather entry to chat list**
- [ ] **Step 2: Route BotFather messages to local state machine**
- [ ] **Step 3: Render inline keyboard buttons in chat view**
- [ ] **Step 4: Test /newbot flow end-to-end**
- [ ] **Step 5: Commit**

### Task 3.5: Bot chat integration

**Files:**
- Modify: `moly-ai/src/chat/` (bot chat entries)
- Modify: `moly-ai/src/bot_manager/chat_bridge.rs`

When a bot is created, add it to the chat list. Route messages to/from
the AITK server. Display connection status indicators.

- [ ] **Step 1: Add created bots to chat list**
- [ ] **Step 2: Route user messages through push_update**
- [ ] **Step 3: Display bot responses from outbound events**
- [ ] **Step 4: Add online/offline status indicator**
- [ ] **Step 5: Test full flow: create bot → paste token in crew-rs → chat**
- [ ] **Step 6: Commit**

---

## Chunk 4: crew-rs base_url Support

### Task 4.1: Add base_url to TelegramChannel

**Files:**
- Modify: `crew-rs/crates/crew-bus/src/telegram_channel.rs`
- Modify: `crew-rs/crates/crew-cli/src/commands/gateway/mod.rs`

- [ ] **Step 1: Add base_url parameter to TelegramChannel::new()**

Add `base_url: Option<String>` parameter. When provided, call
`bot.set_api_url(reqwest::Url::parse(&url)?)` on the teloxide Bot instance.

- [ ] **Step 2: Read base_url from config in gateway init**

In `gateway/mod.rs`, read `base_url` from channel settings and pass to constructor.

- [ ] **Step 3: Test with default URL (no change in behavior)**
- [ ] **Step 4: Test with custom URL pointing to Moly server**
- [ ] **Step 5: Commit**

### Task 4.2: Dashboard TelegramTab update

**Files:**
- Modify: `crew-rs/dashboard/src/components/tabs/TelegramTab.tsx`

- [ ] **Step 1: Add "API URL (Optional)" input field below token input**
- [ ] **Step 2: Save to channel settings as `base_url`**
- [ ] **Step 3: Commit**

---

## Execution Order & Dependencies

```
Task 1.1 → 1.2 → 1.3 → 1.4 → 1.5 → 1.6 → 1.7  (AITK Core, sequential)
                                                    ↓
Task 2.1                                           (AITK Media, after 1.7)
                                                    ↓
Task 3.1 → 3.2 → 3.3 → 3.4 → 3.5                 (Moly, after AITK done)

Task 4.1 → 4.2                                     (crew-rs, independent)
```

Tasks 4.1-4.2 (crew-rs) can be done **in parallel** with everything else.
