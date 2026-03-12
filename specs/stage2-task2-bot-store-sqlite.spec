spec: task
name: "AITK Bot Store (SQLite)"
tags: [stage2, aitk, sqlite, storage]
---

## Intent

Implement a SQLite-based Bot data persistence layer in AITK, managing Bot metadata,
message history, and update tracking. This module provides a storage backend for the
Telegram Bot API Server, supporting restoration of Bot lists and message history
after application restarts.

## Constraints

- Module gated under `cfg(not(target_arch = "wasm32"))` with feature flag
  `telegram-server`
- Use the `rusqlite` crate (consistent with AITK's lightweight positioning)
- Perform schema version check on startup with automatic migration support
- No `.unwrap()` in library code
- All public types must have doc comments and `Debug, Clone` derives
- SQLite file storage path configured by the caller via `BotStoreConfig`

## Decided

- ORM: No ORM; use rusqlite prepared statements directly
- schema_version table records the current version number; checked and migrated on startup
- updates table is separate from messages table: updates tracks long-polling offset,
  messages stores complete message history
- Media files stored in `{data_dir}/bot_media/{file_id}` directory; media table records metadata
- update_id is globally incrementing (across Bots) to simplify offset logic

## Boundaries

### Allowed to modify
- moly-aitk/src/telegram_server/store.rs (new file)
- moly-aitk/src/telegram_server/models.rs (new file or extend)
- moly-aitk/Cargo.toml (add rusqlite dependency)

### Forbidden
- Do not introduce diesel, sea-orm, or other heavyweight ORMs
- Do not compile SQLite code under wasm32

## Out of scope

- Full-text search (to be added later)
- Message encryption
- Database backup/export

## Acceptance criteria

Scenario: Create a Bot and persist it
  Test: test_create_bot_persists
  When `store.create_bot("Weather Assistant", "weather_bot")` is called to create a Bot
  Then it returns a `BotInfo` containing id, token, name, username
  And after reopening the database, `store.get_bot_by_token(token)` returns the same Bot

Scenario: Creating a Bot with a duplicate username is rejected
  Test: test_create_bot_rejects_duplicate_username
  Given a Bot with username "weather_bot" already exists
  When `store.create_bot("Another One", "weather_bot")` is called again
  Then it returns `BotStoreError::DuplicateUsername`

Scenario: List all Bots
  Test: test_list_bots
  Given "3" Bots have been created
  When `store.list_bots()` is called
  Then the returned list has length "3"
  And is sorted by creation time

Scenario: Update Bot properties
  Test: test_update_bot
  Given a Bot with token "1:moly_abc" has been created
  When `store.update_bot(token, BotUpdate { name: Some("New Name"), .. })` is called
  Then `store.get_bot_by_token(token).name` is "New Name"

Scenario: Deleting a Bot also clears associated messages
  Test: test_delete_bot_cascades
  Given a Bot has been created with "5" stored messages
  When `store.delete_bot(token)` is called
  Then `store.get_bot_by_token(token)` returns None
  And the message count for that Bot is "0"

Scenario: Regenerate Token
  Test: test_revoke_token
  Given a Bot has been created with old token "1:moly_old"
  When `store.revoke_token("1:moly_old")` is called
  Then a new token is returned in the format `{id}:{moly_hex32}`
  And the old token is no longer valid

Scenario: Store messages and query by chat
  Test: test_store_and_query_messages
  Given Bot "1:moly_abc" has "10" messages with chat_id "1"
  When `store.get_messages(bot_id, chat_id, limit=5, before_id=None)` is called
  Then the most recent "5" messages are returned
  And they are sorted in chronological order

Scenario: update_id is globally incrementing
  Test: test_update_id_global_increment
  Given Bot A pushes an update and receives update_id "1"
  When Bot B pushes an update
  Then it receives update_id "2"

Scenario: Offset acknowledges and removes consumed updates
  Test: test_offset_acknowledges_updates
  Given Bot "1:moly_abc" has "3" updates with ids "1", "2", "3"
  When `store.get_updates(bot_id, offset=3, limit=100)` is called
  Then updates with update_id >= "3" are returned
  And updates with update_id < "3" can be cleaned up

Scenario: Schema migration
  Test: test_schema_migration
  Given the database schema_version is "0" (empty database)
  When `store.open(path)` is called to open the database
  Then all tables are automatically created
  And schema_version is updated to the current version

Scenario: Media file metadata storage
  Test: test_store_media_metadata
  When `store.store_media(file_id, file_path, mime_type, file_size)` is called
  Then `store.get_media(file_id)` returns the correct file path and MIME type
