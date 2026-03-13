spec: task
name: "Telegram Bot API Enhancements — Streaming, Reply Threading, and UX"
tags: [stage3, telegram, bot-api, streaming, ux]
references: ["https://docs.openclaw.ai/channels/telegram", "https://www.meta-intelligence.tech/insight-openclaw-telegram"]
---

## Intent

Enhance the Telegram Bot API server to support patterns critical for LLM
applications, informed by OpenClaw's production Telegram integration. The
current implementation covers core bot lifecycle and message delivery, but
lacks streaming response display, reply threading, typing indicators, and
long text handling — all essential for a good AI chat UX.

### Priority 1: Streaming Edit-in-Place

LLM responses arrive as a token stream. The bot framework (Octos via
teloxide) should be able to:

1. Call `sendMessage` with an initial placeholder (e.g., "Thinking...")
2. Call `editMessageText` repeatedly as tokens arrive
3. Call `editMessageText` one final time with the complete response

This pattern is already supported by our existing endpoints. The missing
piece is on the Moly app side: `OutboundEvent::EditMessage` must update
the chat UI message in-place rather than just logging.

### Priority 2: Reply-to-Message Support

Add `reply_to_message_id` field to `SendMessageRequest` and propagate it
through the message storage and `OutboundEvent`. This enables:
- Bot replies visually linked to the user message they respond to
- Multi-turn context display in the chat UI

### Priority 3: Typing Indicator (sendChatAction)

Add `sendChatAction` endpoint accepting `action: "typing"`. Emit an
`OutboundEvent::ChatAction` so the Moly UI can show a typing indicator
while the bot is processing.

### Priority 4: Long Text Chunking

LLM responses can exceed Telegram's 4096-character message limit. Add
automatic text chunking in `sendMessage`:
- Split at paragraph boundaries when possible
- Fall back to sentence boundaries, then hard split at limit
- Send as multiple sequential messages
- Configurable chunk size (default 4096)

### Priority 5: parse_mode Passthrough

Ensure `parse_mode` in `SendMessageRequest` and `EditMessageTextRequest`
is stored in the message record and included in `OutboundEvent`, so the
Moly UI can render Markdown or HTML formatted bot responses.

### Priority 6: setMyCommands Persistence

Upgrade `setMyCommands` from stub to functional: persist commands in
SQLite per bot, return them from `getMyCommands`. This lets bot frameworks
register their command menus.


## Constraints

- All changes must compile for desktop, mobile, and web targets
- AITK changes require a separate PR to Project-Robius-China/aitk
- Streaming edit-in-place must not block the UI thread
- Text chunking must preserve message ordering (sequential message_ids)
- parse_mode values: "Markdown", "MarkdownV2", "HTML", or absent (plain)
- sendChatAction is stateless — no persistence needed, just event forwarding
- reply_to_message_id is optional — omitting it sends a standalone message
- Chunk size must respect Telegram's real 4096-char limit for compatibility


## Decisions

- Streaming pattern: reuse existing sendMessage + editMessageText endpoints
  (no new endpoints needed on the AITK side)
- Text chunking: implemented in AITK handler, transparent to bot clients
- OutboundEvent: add ChatAction variant, add reply_to_message_id to
  SendMessage variant, add parse_mode to SendMessage and EditMessage
- Bot commands storage: add `bot_commands` table to SQLite schema
- Typing indicator: ephemeral event, not persisted in messages table


## Boundaries

### Allowed Changes

**AITK (Library Layer)**
- `telegram_server/types.rs` — add reply_to_message_id, parse_mode fields,
  ChatAction request type, OutboundEvent::ChatAction variant
- `telegram_server/api/handlers.rs` — add sendChatAction handler, text
  chunking in sendMessage, pass parse_mode through
- `telegram_server/api/mod.rs` — add sendChatAction route
- `telegram_server/store.rs` — add bot_commands CRUD methods
- `telegram_server/schema.sql` — add bot_commands table

**Moly App**
- `src/data/store.rs` — handle OutboundEvent::EditMessage (update UI),
  OutboundEvent::ChatAction (show typing)
- `src/chat/` — render reply-to references, typing indicators,
  formatted text (Markdown/HTML)

### Forbidden
- Do not add webhook mode
- Do not add group chat or multi-user support
- Do not add reaction support
- Do not modify existing endpoint behavior (only extend)


## Out of Scope

- Webhook mode (setWebhook)
- Reaction support (message_reaction updates)
- Forum topics and thread isolation
- Multi-account bot routing
- Sticker processing and caching
- Access control / pairing mechanisms
- Media endpoints (tracked separately in stage2-task3-media-api.spec)


## Acceptance Criteria

Scenario: Streaming edit-in-place updates chat UI
  Test: test_edit_message_updates_ui
  Given a bot has sent message_id 1 with text "Thinking..."
  When the bot calls editMessageText with message_id 1 and text "Hello world"
  Then the Moly chat UI updates message 1 in-place to "Hello world"
  And no duplicate message appears

Scenario: Multiple rapid edits converge to final text
  Test: test_rapid_edits_converge
  Given a bot has sent message_id 1
  When the bot calls editMessageText 5 times in quick succession
  Then the chat UI shows the final edit text
  And intermediate states do not cause flickering

Scenario: sendMessage with reply_to_message_id
  Test: test_send_message_with_reply
  Given the user sent a message with message_id 3
  When the bot calls sendMessage with reply_to_message_id 3
  Then the OutboundEvent::SendMessage includes reply_to_message_id 3
  And the stored message record contains the reply reference

Scenario: sendChatAction emits typing event
  Test: test_send_chat_action_typing
  Given a valid bot token
  When the bot calls sendChatAction with action "typing"
  Then an OutboundEvent::ChatAction is emitted with action "typing"
  And the response is {"ok": true, "result": true}

Scenario: Long text is automatically chunked
  Test: test_long_text_chunking
  Given a bot calls sendMessage with text of 8000 characters
  When the handler processes the request
  Then two messages are stored (each under 4096 characters)
  And two OutboundEvent::SendMessage events are emitted in order
  And the split occurs at a paragraph or sentence boundary

Scenario: parse_mode is stored and forwarded
  Test: test_parse_mode_passthrough
  Given a bot calls sendMessage with parse_mode "Markdown" and text "**bold**"
  When the message is stored and the OutboundEvent is emitted
  Then the message record contains parse_mode "Markdown"
  And the OutboundEvent includes parse_mode "Markdown"

Scenario: setMyCommands persists and getMyCommands returns them
  Test: test_set_and_get_my_commands
  Given a bot calls setMyCommands with commands [{"command":"help","description":"Show help"}]
  When another client calls getMyCommands for the same bot
  Then the response contains the registered command
  And the command has description "Show help"

Scenario: Reply-to reference absent when not specified
  Test: test_send_message_without_reply
  Given a bot calls sendMessage without reply_to_message_id
  When the message is stored
  Then reply_to_message_id is null in the message record
