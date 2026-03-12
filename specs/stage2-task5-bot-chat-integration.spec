spec: task
name: "Moly Bot Chat UI Integration"
tags: [stage2, moly, ui, chat]
---

## Intent

Integrate the AITK Telegram Bot API Server with Moly's chat UI, allowing users
to chat with Bots directly within Moly. Messages sent by users are passed to
crew-rs via push_update, and crew-rs replies are displayed in the Moly chat
interface via recv_outbound. Supports text messages, inline keyboard button
interactions, media message rendering, and Bot connection status display.

## Constraints

- Reuse the existing MolyKit Chat widget components; do not rewrite the chat UI
- Bot chat and BotFather chat use the same Chat component, distinguished by message routing
- Connection status is detected via getUpdates request frequency: mark as offline after 2 minutes without a request
- Inline keyboard button clicks send a callback_query type update
- Media messages (images, documents, etc.) must be rendered correctly in chat

## Decisions

- Bot list is displayed in the chat sidebar; BotFather is pinned first, the rest sorted by last active time
- Connection status indicator: green dot = online, gray dot = offline, shown next to the Bot avatar
- Inline keyboard rendered as button rows; clicking sends callback_data
- Messages sent by the user are constructed as Telegram Update objects containing a Message structure

## Boundaries

### Allowed Changes
- src/bot_manager/mod.rs
- src/bot_manager/telegram_bot_client.rs
- src/bot_manager/message_adapter.rs
- src/chat/chats_deck.rs
- src/chat/chat_history.rs
- src/chat/chat_history_card.rs
- src/chat/chat_screen.rs
- src/chat/entity_button.rs
- src/data/store.rs
- src/data/providers.rs
- src/data/bot_fetcher.rs
- src/data/chats/mod.rs
- src/shared/actions.rs
- src/settings/add_provider_modal.rs
- specs/stage2-task5-bot-chat-integration.spec

### Forbidden
- Do not rewrite the MolyKit Chat widget
- Do not modify AITK library code (only call its interfaces)
- Do not modify existing Provider chat behavior

## Out of Scope

- Message search functionality
- Typing indicator animation
- Bot avatar custom rendering

## Acceptance Criteria

Scenario: Bot appears in the chat list
  Test: test_bot_appears_in_chat_list
  Given a "Weather Assistant" Bot was created via BotFather
  When viewing the chat sidebar
  Then "Weather Assistant" appears in the list, below BotFather

Scenario: Send a text message to a Bot
  Test: test_send_text_to_bot
  Given the user enters the "Weather Assistant" chat view
  When the user types "What's the weather like today" and sends
  Then the message appears on the right side of the chat view (user message)
  And the AITK update queue contains an Update object for that message

Scenario: Receive a Bot text reply
  Test: test_receive_bot_text_reply
  Given crew-rs sent "Sunny today, 25C" via sendMessage
  When Moly receives that message via recv_outbound
  Then the message appears on the left side of the chat view (Bot message)

Scenario: Render inline keyboard buttons
  Test: test_render_inline_keyboard
  Given crew-rs sent a message with inline_keyboard:
    | row | text            | callback_data  |
    | 0   | Detailed Weather | weather_detail |
    | 0   | Next 3 Days      | weather_3day   |
  When Moly displays that message
  Then "2" buttons are rendered below the message

Scenario: Click inline keyboard button sends callback
  Test: test_inline_keyboard_click_sends_callback
  Given there is a button with callback_data "weather_detail" below the message
  When the user clicks that button
  Then the AITK update queue contains a callback_query type Update
  And callback_query.data is "weather_detail"

Scenario: Bot connection status — online
  Test: test_bot_online_status
  Given crew-rs called getUpdates within the past "30" seconds
  When viewing the status of "Weather Assistant" in the chat list
  Then a green online indicator is displayed

Scenario: Bot connection status — offline
  Test: test_bot_offline_status
  Given crew-rs has not called getUpdates for more than "2" minutes
  When viewing the status of "Weather Assistant" in the chat list
  Then a gray offline indicator is displayed

Scenario: Receive and render a media message
  Test: test_receive_media_message
  Given crew-rs sent an image via sendPhoto with caption "Weather Chart"
  When Moly receives and displays that message
  Then the chat view shows an image preview
  And the caption "Weather Chart" is displayed below the image

Scenario: Bot message edit updates in real time
  Test: test_message_edit_updates_ui
  Given the Bot has sent message_id "5" with content "Processing..."
  When crew-rs calls editMessageText to change the content to "Processing complete!"
  Then the text of message_id "5" in the chat view is updated to "Processing complete!"

Scenario: Bot message deletion removes it from the UI
  Test: test_message_delete_removes_from_ui
  Given the Bot has sent a message with message_id "5"
  When crew-rs calls deleteMessage to delete that message
  Then that message is no longer displayed in the chat view

Scenario: Chat history is restored after app restart
  Test: test_restore_chat_history_on_restart
  Given Bot "Weather Assistant" has "10" historical messages stored in SQLite
  When the Moly app restarts and opens the "Weather Assistant" chat
  Then "10" historical messages are displayed

Scenario: Sending message to deleted bot shows error
  Test: test_send_message_to_deleted_bot
  Given a Bot "Weather Assistant" existed but was deleted via BotFather
  When the user tries to send a message in the "Weather Assistant" chat
  Then the message is not sent
  And the chat view shows an error indicating the bot no longer exists
