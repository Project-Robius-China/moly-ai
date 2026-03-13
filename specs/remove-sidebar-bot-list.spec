spec: task
name: "Remove Bot List from Sidebar"
tags: [ui, sidebar, cleanup]
---

## Intent

The sidebar currently displays a "BOTS" section listing all Telegram bots created
via BotFather. As users create more bots, this flat list grows unbounded and pushes
chat history down, degrading the sidebar UX.

Bots are already selectable via the chat interface's model selector dropdown.
Remove the redundant sidebar bot list and return the sidebar to a clean
chat-history-only design.

## Decisions

- Remove the "BOTS" heading and `BotButton` entries from the sidebar `PortalList`
- Remove `bot_sidebar_cache` from `Store` (no longer consumed by rendering)
- Migrate `is_bot_available()` in `chat_view.rs` from `bot_sidebar_cache` to
  `bot_name_cache` (both populated by the same `refresh_bot_name_cache()` call)
- Keep `bot_name_cache` and `refresh_bot_name_cache()` — still needed for
  `get_bot_display_name()`
- Keep `entity_button.rs` module — still used by `landing/model_list.rs`

## Boundaries

### Allowed to Modify
- src/chat/chat_history.rs (remove BOTS rendering and bot click handler)
- src/data/store.rs (remove `bot_sidebar_cache` field and its population)
- src/chat/chat_view.rs (update `is_bot_available()` to use `bot_name_cache`)

### Must NOT
- Delete src/chat/entity_button.rs (still used by landing/model_list.rs)
- Remove `bot_name_cache` or `refresh_bot_name_cache()` from store.rs
- Break telegram bot chat functionality (bots must still be usable via model selector)
- Modify any moly-kit or AITK code

## Acceptance Criteria

Scenario: Sidebar shows only chat history, no bot entries
  Test:
    Package: moly
    Filter: test_sidebar_has_no_bot_section
  Given the app is running with bots registered in the bot server
  When the sidebar is rendered
  Then no "BOTS" heading or bot entry appears in the sidebar
  And only "CHATS" heading and chat history cards are rendered

Scenario: Bot sidebar cache field is removed from Store
  Test:
    Package: moly
    Filter: test_bot_sidebar_cache_removed
  Given the source code of store.rs
  When searching for "bot_sidebar_cache"
  Then no occurrence is found

Scenario: Telegram bots remain available via model selector
  Test:
    Package: moly
    Filter: test_telegram_bot_still_available
  Given the source code of chat_view.rs
  When checking `is_bot_available()` for a telegram bot
  Then it uses `bot_name_cache` to determine availability
  And returns true for bots registered in the bot server

Scenario: EntityButton module is preserved
  Test:
    Package: moly
    Filter: test_entity_button_module_exists
  Given the source code of chat/mod.rs
  When checking module declarations
  Then `pub mod entity_button` still exists

## Out of Scope

- Redesigning the model selector or discover page
- Adding bot grouping, filtering, or search
- Changing BotFather startup behavior (separate task)
