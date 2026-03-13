spec: task
name: "BotFather Settings Panel Redesign"
tags: [ui, settings, botfather, cleanup]
---

## Intent

The provider settings UI currently shows both "BotFather" and "Telegram Bot" as
separate providers. This is confusing because Telegram Bot is a derivative feature
of BotFather — enabling BotFather automatically activates Telegram Bot functionality
in the backend. Users cannot configure Telegram Bot independently, so showing it as
a separate provider is misleading.

Additionally, the BotFather settings panel has excessive blank space. The only
actionable configuration (server port) is stored in preferences but not exposed in
the UI.

## Decisions

- Hide TelegramBot from the provider settings list by filtering
  `ProviderType::TelegramBot` during rendering. Keep the internal registration
  logic, enum variant, and bot client creation unchanged.
- Add a "Server Configuration" section to the BotFather panel exposing
  `bot_server_port` with a text input and Save button.
- Save triggers a hot-restart of the bot server via a new
  `Store::restart_bot_server(new_port)` method (no app restart required).
- Update footer text to: "BotFather runs locally — created bots are accessible
  from the chat model selector."

## Boundaries

### Allowed to Modify
- src/settings/providers.rs (filter TelegramBot from provider list rendering)
- src/settings/botfather_view.rs (add port input, Save button, action, updated footer)
- src/settings/provider_view.rs (pass port to BotFatherView, handle Save action)
- src/data/store.rs (add `restart_bot_server(new_port)` method)
- src/data/preferences.rs (add port save method if needed)

### Must NOT
- Modify any AITK or moly-kit code
- Remove `ProviderType::TelegramBot` enum variant
- Remove TelegramBot auto-registration logic in store.rs
- Change bot client creation logic in chat_screen.rs
- Break telegram bot chat functionality

## Acceptance Criteria

Scenario: TelegramBot hidden from settings
  Test:
    Package: moly
    Filter: test_telegram_bot_hidden_from_settings
  Given the app is running with bot server active
  When the user opens provider settings
  Then only "BotFather" appears — no "Telegram Bot" entry in provider list
  And Telegram bot chat functionality still works via model selector

Scenario: Port configuration visible in BotFather panel
  Test:
    Package: moly
    Filter: test_port_config_in_botfather_panel
  Given the source code of botfather_view.rs
  When checking the live_design! macro
  Then a "Server Configuration" section exists with a port input
  And a Save button is present

Scenario: Port change triggers hot restart
  Test:
    Package: moly
    Filter: test_port_change_hot_restart
  Given the source code of store.rs
  When checking the `restart_bot_server` method
  Then it saves the new port to preferences
  And drops the old server handle
  And starts a new server with the new port
  And refreshes the bot name cache

Scenario: Invalid port handling
  Test:
    Package: moly
    Filter: test_invalid_port_rejected
  Given the user enters a non-numeric or out-of-range port value
  When clicking Save
  Then the save is rejected with no server restart
  And the previous port value is preserved

## Out of Scope

- Redesigning the model selector or discover page
- Adding bot list to BotFather settings panel
- Merging BotFather and TelegramBot into a single ProviderType
- Changing BotFather chat/dialog behavior
