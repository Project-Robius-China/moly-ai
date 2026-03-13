spec: task
name: "Octos Telegram base_url Support"
tags: [stage2, Octos, telegram, config]
---

## Intent

Add a `base_url` field to the Telegram channel configuration in Octos,
allowing teloxide to connect to a custom Telegram Bot API server (such as
Moly's local server) instead of api.telegram.org. This is the only change
required on the Octos side for the Moly Bot-Native Messaging approach.

## Constraints

- Minimal scope of changes: only modify TelegramChannel initialization logic
- Backward compatible: base_url is Optional; default behavior unchanged (connects to api.telegram.org)
- Use teloxide's built-in `Bot::set_api_url()` method to set the custom URL
- Add an optional API URL input field to the Web dashboard's TelegramTab

## Decided

- Config field name: `base_url` (consistent with LLM provider base_url naming)
- teloxide API: use `reqwest::Url::parse(base_url)` then `Bot::set_api_url(url)`
- Dashboard UI: add an "API URL (Optional)" input field below the Bot Token
  input, with placeholder "https://api.telegram.org"

## Boundary

### Allowed to Modify
- crates/octos-bus/src/telegram_channel.rs (add parameter to TelegramChannel::new)
- crates/octos-cli/src/commands/gateway/mod.rs (read base_url config)
- crates/octos-cli/src/config.rs (add new field to ChannelEntry settings)
- dashboard/src/components/tabs/TelegramTab.tsx (add URL input field)

### Forbidden
- Do not modify TelegramChannel's message processing logic
- Do not modify other channel types
- Do not break default behavior when base_url is absent

## Out of Scope

- TELOXIDE_TELEGRAM_API_URL environment variable support (already built into teloxide)
- URL validity validation (beyond URL parsing)
- Auto-discovery of Moly server

## Acceptance Criteria

Scenario: Behavior unchanged when base_url is not configured
  Test: test_default_telegram_url
  Given the channel config does not contain a "base_url" field
  When creating a TelegramChannel
  Then the teloxide Bot uses the default "https://api.telegram.org" URL

Scenario: Connects to custom server after configuring base_url
  Test: test_custom_base_url
  Given the channel config has base_url set to "http://localhost:8488"
  When creating a TelegramChannel and calling getMe
  Then the HTTP request is sent to "http://localhost:8488/bot{token}/getMe"

Scenario: Invalid base_url returns an error
  Test: test_invalid_base_url_returns_error
  Given the channel config has base_url set to "not-a-url"
  When creating a TelegramChannel
  Then a config error is returned containing URL parse failure information

Scenario: Dashboard displays API URL input field
  Test: test_dashboard_shows_api_url_input
  When viewing the Dashboard's Telegram configuration page
  Then an optional "API URL" input field is displayed
  And the placeholder is "https://api.telegram.org"
