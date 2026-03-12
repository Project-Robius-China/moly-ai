//! BotFather client implementing the `BotClient` trait.
//!
//! This allows BotFather to integrate seamlessly with the existing
//! chat system — users interact with BotFather through the normal
//! chat interface, and responses are generated locally.

use crate::bot_manager::dialog::{self, DialogState};
use async_stream::stream;
use moly_kit::aitk::telegram_server::ServerState;
use moly_kit::prelude::*;
use std::sync::{Arc, Mutex};

/// A purely local `BotClient` that processes BotFather commands.
///
/// No network requests are made — all operations happen in-process
/// against the shared [`ServerState`]'s bot store.
pub struct BotFatherClient {
    server_state: Arc<ServerState>,
    state: Arc<Mutex<DialogState>>,
    server_port: u16,
}

impl Clone for BotFatherClient {
    fn clone(&self) -> Self {
        Self {
            server_state: self.server_state.clone(),
            state: self.state.clone(),
            server_port: self.server_port,
        }
    }
}

impl BotFatherClient {
    /// Creates a new BotFather client backed by the given server state.
    pub fn new(
        server_state: Arc<ServerState>,
        server_port: u16,
    ) -> Self {
        Self {
            server_state,
            state: Arc::new(Mutex::new(DialogState::default())),
            server_port,
        }
    }
}

impl BotFatherClient {
    /// Returns the welcome message content with quick reply buttons.
    ///
    /// Delegates to [`super::dialog::welcome_message`] — the single source
    /// of truth for BotFather's greeting content and buttons.
    pub fn welcome_message() -> MessageContent {
        super::dialog::welcome_message()
    }
}

impl BotClient for BotFatherClient {
    fn bots(
        &mut self,
    ) -> BoxPlatformSendFuture<'static, ClientResult<Vec<Bot>>> {
        let bot = Bot {
            id: BotId::new("botfather"),
            name: "BotFather".to_string(),
            avatar: EntityAvatar::Text("B".into()),
            capabilities: BotCapabilities::new()
                .with_capabilities([BotCapability::TextInput]),
        };

        Box::pin(async move { ClientResult::new_ok(vec![bot]) })
    }

    fn clone_box(&self) -> Box<dyn BotClient> {
        Box::new(self.clone())
    }

    fn send(
        &mut self,
        _bot_id: &BotId,
        messages: &[Message],
        _tools: &[Tool],
    ) -> BoxPlatformSendStream<'static, ClientResult<MessageContent>> {
        let server_state = self.server_state.clone();
        let state = self.state.clone();
        let server_port = self.server_port;

        // Extract the last user message
        let user_message = messages
            .iter()
            .rev()
            .find(|m| m.from == EntityId::User)
            .map(|m| m.content.text.clone())
            .unwrap_or_default();

        let stream = stream! {
            let response = {
                let mut dialog_state =
                    state.lock().expect("BotFather state lock poisoned");

                // If we're in Idle state and the input looks like a bot
                // selection (number or @username), route to the handler.
                let is_selection =
                    matches!(*dialog_state, DialogState::Idle)
                        && !user_message.starts_with('/')
                        && is_bot_selection(&user_message);

                if is_selection {
                    dialog::resolve_bot_selection(
                        &mut dialog_state,
                        user_message.trim(),
                        &server_state.store,
                    )
                } else {
                    dialog::process_input(
                        &mut dialog_state,
                        &user_message,
                        &server_state.store,
                        server_port,
                    )
                }
            };

            yield ClientResult::new_ok(response);
        };

        Box::pin(stream)
    }
}

/// Heuristic to detect if input is a bot selection (number or @username).
fn is_bot_selection(input: &str) -> bool {
    let trimmed = input.trim();
    if trimmed.parse::<usize>().is_ok() {
        return true;
    }
    if trimmed.starts_with('@') {
        return true;
    }
    if !trimmed.contains(' ')
        && trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_botfather_welcome_message() {
        let content = BotFatherClient::welcome_message();
        assert!(content.text.contains("Welcome to BotFather"));
        assert!(content.text.contains("/newbot"));
        assert!(content.text.contains("/mybots"));
        assert!(content.quick_replies.is_empty());
    }
}
