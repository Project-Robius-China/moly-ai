spec: task
name: "Moly Telegram Server Integration Startup"
tags: [stage2, moly, integration, telegram-server]
---

## Intent

Integrate AITK's telegram-server module into the Moly App: update the AITK dependency
to the latest version (including the telegram-server feature), automatically start the
Telegram Bot API server on Moly startup, store the ServerState in the App Store, and
start the outbound event consumption loop.

This is a prerequisite for BotFather (task4) and Bot Chat Integration (task5).
Once completed, Moly will have the ability to accept crew-rs teloxide connections.

## Constraints

- AITK dependency uses a git rev pointing to the mainline commit after PR #2 is merged
- telegram-server feature is only enabled on native platforms; excluded on wasm32
- Server listens on 127.0.0.1 with a configurable port (default 8488)
- ServerState stored in the Store for use by subsequent BotFather/Chat features
- Outbound event loop runs asynchronously, logging events at info level (no UI display yet)
- Use AITK's spawn() function (cross-platform async); do not use tokio::spawn directly

## Decided

- AITK dependency in moly-kit/Cargo.toml: update rev + add "telegram-server" feature
- moly-kit's telegram-server feature is passed through via cfg gate
- ServerState stored as Option<Arc<ServerState>> in the Store (None on wasm)
- ServerHandle is owned by the Store; automatically shuts down the server on Drop
- Port number written to Preferences (default 8488), available for BotFather to inform
  users when creating Bots
- outbound_rx is consumed in a separately spawned task; currently only prints via log::info

## Boundaries

### Allowed to modify
- moly-kit/Cargo.toml (update aitk dependency)
- src/data/store.rs (add ServerState field and initialization)
- src/data/preferences.rs (add bot_server_port config option)
- src/app.rs (initialize server on startup)
- Cargo.toml (workspace level, if needed)

### Forbidden
- Do not modify AITK library code
- Do not modify existing Provider/Chat logic
- Do not add any Bot UI (that belongs to task4/task5)

## Out of scope

- BotFather dialog logic (-> task4)
- Bot chat UI integration (-> task5)
- Automatic Bot creation (-> task4)
- Media API endpoints (-> task3-media)

## Acceptance criteria

Scenario: Telegram server starts automatically after Moly launches
  Test: test_server_starts_on_app_init
  When the Moly App completes Store initialization
  Then ServerState is not None
  And the server is listening on 127.0.0.1:8488

Scenario: curl getMe returns 404 (no Bot)
  Test: test_server_responds_to_requests
  Given Moly has started and the server is running
  When a curl request is made to http://127.0.0.1:8488/botinvalid/getMe
  Then it returns HTTP 200 with ok=false in the body

Scenario: After creating a Bot, getMe returns correct information
  Test: test_create_bot_and_get_me
  Given Moly has started and the server is running
  When a Bot is created via ServerState.store.create_bot("Test", "test_bot")
  And a curl request is made to http://127.0.0.1:8488/bot{token}/getMe
  Then it returns HTTP 200 with ok=true and username="test_bot" in the body

Scenario: Server does not start on the wasm32 platform
  Test: test_no_server_on_wasm
  Given the compilation target is wasm32
  When Store initialization completes
  Then ServerState is None
  And there are no compilation errors

Scenario: Server shuts down automatically when the App exits
  Test: test_server_shuts_down_on_drop
  Given the server has been started
  When the Store is dropped
  Then ServerHandle triggers shutdown
  And the port is released
