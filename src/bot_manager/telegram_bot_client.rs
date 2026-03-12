//! `BotClient` that bridges user messages to Telegram bots created via
//! BotFather. Pushes user input as Telegram Updates; bot responses arrive
//! asynchronously via `OutboundEvent` (handled in `chats_deck.rs`).

use moly_kit::aitk::telegram_server::ServerState;
use moly_kit::prelude::*;
use std::sync::Arc;

/// Routes user messages to Telegram bots via the server's update queue.
#[derive(Clone)]
pub struct TelegramBotClient {
    server_state: Arc<ServerState>,
}

impl TelegramBotClient {
    /// Creates a new client backed by the given server state.
    pub fn new(server_state: Arc<ServerState>, _server_port: u16) -> Self {
        Self { server_state }
    }

    /// Strips the RouterClient provider prefix from a `BotId`
    /// (e.g. `"telegram_bot/123:ABC"` → `"123:ABC"`).
    fn extract_token(bot_id: &BotId) -> &str {
        bot_id
            .as_str()
            .rsplit_once('/')
            .map(|(_, token)| token)
            .unwrap_or(bot_id.as_str())
    }

    fn build_update_json(messages: &[Message]) -> serde_json::Value {
        let now = chrono::Utc::now().timestamp();
        let user_text = messages
            .iter()
            .rev()
            .find(|m| m.from == EntityId::User)
            .map(|m| m.content.text.clone())
            .unwrap_or_default();

        // Callback format: "cb:{source_message_id}:{callback_data}"
        if let Some(cb_payload) = user_text.strip_prefix("cb:") {
            let (source_msg_id, callback_data) = cb_payload
                .split_once(':')
                .unwrap_or(("0", cb_payload));

            let mut update = serde_json::json!({
                "update_id": 0,
                "callback_query": {
                    "id": format!("cb_{now}"),
                    "from": {
                        "id": 1,
                        "is_bot": false,
                        "first_name": "User"
                    },
                    "chat_instance": "1",
                    "data": callback_data
                }
            });

            if let Some(source_message) =
                Self::find_message_by_id(messages, source_msg_id, now)
            {
                update["callback_query"]["message"] = source_message;
            }

            update
        } else {
            serde_json::json!({
                "update_id": 0,
                "message": {
                    "message_id": 0,
                    "from": {
                        "id": 1,
                        "is_bot": false,
                        "first_name": "User"
                    },
                    "chat": {
                        "id": 1,
                        "type": "private"
                    },
                    "date": now,
                    "text": user_text
                }
            })
        }
    }

    /// Finds a bot message by its Telegram message_id (stored in `content.data`).
    fn find_message_by_id(
        messages: &[Message],
        message_id: &str,
        now: i64,
    ) -> Option<serde_json::Value> {
        messages.iter().rev().find_map(|message| {
            if message.content.data.as_deref() != Some(message_id) {
                return None;
            }
            let msg_id = message_id.parse::<i64>().ok()?;
            Some(serde_json::json!({
                "message_id": msg_id,
                "from": {
                    "id": 0,
                    "is_bot": true,
                    "first_name": "Bot"
                },
                "chat": {
                    "id": 1,
                    "type": "private"
                },
                "date": now,
                "text": message.content.text
            }))
        })
    }
}

impl BotClient for TelegramBotClient {
    fn bots(
        &mut self,
    ) -> BoxPlatformSendFuture<'static, ClientResult<Vec<Bot>>> {
        let bots = match self.server_state.store.list_bots() {
            Ok(bot_infos) => {
                let bots: Vec<Bot> = bot_infos
                    .into_iter()
                    // BotFather is handled by BotFatherClient
                    .filter(|info| info.username != "BotFather")
                    .map(|info| Bot {
                        id: BotId::new(&info.token),
                        name: info.name.clone(),
                        avatar: EntityAvatar::Text(
                            info.name
                                .chars()
                                .next()
                                .unwrap_or('B')
                                .to_string(),
                        ),
                        capabilities: BotCapabilities::new()
                            .with_capabilities([
                                BotCapability::TextInput,
                            ]),
                    })
                    .collect();
                ClientResult::new_ok(bots)
            }
            Err(e) => ClientResult::new_err(vec![ClientError::new(
                ClientErrorKind::Unknown,
                format!("Failed to list bots: {e}"),
            )]),
        };

        Box::pin(async move { bots })
    }

    fn clone_box(&self) -> Box<dyn BotClient> {
        Box::new(self.clone())
    }

    fn send(
        &mut self,
        bot_id: &BotId,
        messages: &[Message],
        _tools: &[Tool],
    ) -> BoxPlatformSendStream<'static, ClientResult<MessageContent>> {
        let server_state = self.server_state.clone();
        let bot_token = Self::extract_token(bot_id).to_string();
        let update_json = Self::build_update_json(messages).to_string();

        let stream = async_stream::stream! {
            // Verify the bot still exists
            match server_state.store.get_bot_by_token(&bot_token) {
                Ok(Some(_)) => {}
                Ok(None) => {
                    yield ClientResult::new_ok(MessageContent {
                        text: "This bot no longer exists.".to_string(),
                        ..Default::default()
                    });
                    return;
                }
                Err(e) => {
                    yield ClientResult::new_ok(MessageContent {
                        text: format!("Failed to reach bot: {e}"),
                        ..Default::default()
                    });
                    return;
                }
            }

            if let Err(e) = server_state.push_update(
                &bot_token,
                &update_json,
            ) {
                yield ClientResult::new_ok(MessageContent {
                    text: format!("Failed to send message to bot: {e}"),
                    ..Default::default()
                });
            }
        };

        Box::pin(stream)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn user_message(text: &str) -> Message {
        Message {
            from: EntityId::User,
            content: MessageContent {
                text: text.to_string(),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn bot_message(message_id: &str, text: &str) -> Message {
        Message {
            from: EntityId::Bot(BotId::new("telegram_bot/123:ABC")),
            content: MessageContent {
                text: text.to_string(),
                data: Some(message_id.to_string()),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    #[test]
    fn test_extract_token() {
        assert_eq!(
            TelegramBotClient::extract_token(&BotId::new(
                "telegram_bot/123:ABC"
            )),
            "123:ABC"
        );
        assert_eq!(
            TelegramBotClient::extract_token(&BotId::new("raw_token")),
            "raw_token"
        );
    }

    #[test]
    fn test_build_update_json_for_callback_includes_source_message() {
        // Format: cb:{source_message_id}:{callback_data}
        let update = TelegramBotClient::build_update_json(&[
            bot_message("42", "Pick one"),
            user_message("cb:42:option-a"),
        ]);

        assert_eq!(update["callback_query"]["data"], "option-a");
        assert_eq!(update["callback_query"]["message"]["message_id"], 42);
        assert_eq!(update["callback_query"]["message"]["text"], "Pick one");
        assert_eq!(update["callback_query"]["message"]["chat"]["id"], 1);
    }

    #[test]
    fn test_callback_binds_to_correct_message_not_latest() {
        // Bot sends message A (id=10, with buttons), then message B (id=11).
        // User clicks button on A → should bind to A, not B.
        let update = TelegramBotClient::build_update_json(&[
            bot_message("10", "Pick one"),
            bot_message("11", "Just a follow-up"),
            user_message("cb:10:option-a"),
        ]);

        assert_eq!(update["callback_query"]["data"], "option-a");
        assert_eq!(update["callback_query"]["message"]["message_id"], 10);
        assert_eq!(update["callback_query"]["message"]["text"], "Pick one");
    }

    #[test]
    fn test_build_update_json_for_plain_message_keeps_text_message_shape() {
        let update =
            TelegramBotClient::build_update_json(&[user_message("hello")]);

        assert_eq!(
            update,
            json!({
                "update_id": 0,
                "message": {
                    "message_id": 0,
                    "from": {
                        "id": 1,
                        "is_bot": false,
                        "first_name": "User"
                    },
                    "chat": {
                        "id": 1,
                        "type": "private"
                    },
                    "date": update["message"]["date"],
                    "text": "hello"
                }
            })
        );
    }
}
