spec: task
name: "BotFather Interactive Responses"
tags: [botfather, ux, quick-reply]
---

## Intent

Enhance BotFather dialog responses to return structured `MessageContent`
instead of plain strings. Two concrete improvements: (1) `/mybots` returns
a clickable bot list via `quick_replies` so users tap a bot name instead of
typing a number; (2) bot creation success includes a copyable token rendered
as inline code via markdown backticks (MolyKit markdown already renders
code blocks with copy support).

## Decisions

- `dialog::process_input` and `dialog::resolve_bot_selection` return
  `MessageContent` instead of `String`, so they can attach `quick_replies`
- `/mybots` populates `quick_replies` with one button per bot, label =
  `"@{username}"`, action = `"{1-based-index}"` (matches existing
  selection logic), style = `ButtonStyle::Secondary`
- `/mybots` empty state uses a single `QuickReplyButton` for `/newbot`
- Token display stays in markdown backtick format (`` `token` ``) which
  MolyKit already renders as copyable code
- `client.rs` no longer wraps `response` in `MessageContent { text: response }`;
  it forwards the `MessageContent` directly
- `format_bot_created` returns `MessageContent` with the token in text and
  a `/mybots` quick reply button for next action

## Constraints

- Must not break existing dialog unit tests; update assertions as needed
- Must not add new dependencies
- All user-facing strings in English

## Boundaries

### Allowed Changes
- src/bot_manager/dialog.rs
- src/bot_manager/client.rs
- src/bot_manager/mod.rs

### Forbidden
- Do not modify AITK or moly-kit code
- Do not change the QuickReplyGroup widget
- Do not modify any settings or UI files

## Out of Scope

- Copy button as a dedicated widget (rely on markdown code rendering)
- Inline keyboard / callback buttons (future protocol extension)
- Bot management sub-menu quick replies (keep text-based for now)

## Completion Criteria

Scenario: /mybots with bots returns clickable list
  Test: test_mybots_returns_quick_replies
  Given the bot store contains 2 bots "alpha_bot" and "beta_bot"
  When the user sends "/mybots"
  Then the response text contains "Your bots (2 total)"
  And the response has 2 quick_replies
  And quick_reply[0].label is "@alpha_bot"
  And quick_reply[0].action is "1"
  And quick_reply[1].label is "@beta_bot"
  And quick_reply[1].action is "2"

Scenario: /mybots with no bots shows newbot button
  Test: test_mybots_empty_returns_newbot_button
  Given the bot store is empty
  When the user sends "/mybots"
  Then the response text contains "haven't created any bots"
  And the response has 1 quick_reply with label "Create a Bot" and action "/newbot"

Scenario: bot creation success includes token in backticks
  Test: test_newbot_token_in_backticks
  Given a new bot "demo_bot" is created with token "abc123"
  When format_bot_created is called
  Then the response text contains "`abc123`"
  And the response has 1 quick_reply with label "My Bots" and action "/mybots"

Scenario: /start and /help return quick replies
  Test: test_start_returns_quick_replies
  Given the dialog is in Idle state
  When the user sends "/start"
  Then the response text contains "Welcome to BotFather"
  And the response has 3 quick_replies for "/newbot", "/mybots", "/help"

Scenario: process_input returns MessageContent not String
  Test: test_process_input_returns_message_content
  Given the dialog is in Idle state
  When the user sends "/cancel"
  Then the return type is MessageContent
  And the text field contains "No active operation"

Scenario: resolve_bot_selection returns MessageContent
  Test: test_resolve_bot_selection_returns_message_content
  Given the bot store contains 1 bot "test_bot"
  When the user selects bot "1"
  Then the return type is MessageContent
  And the text field contains management menu for "test_bot"
