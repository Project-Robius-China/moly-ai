use makepad_widgets::{ActionDefaultRef, DefaultNone};
use moly_kit::prelude::*;
use moly_protocol::data::FileId;

use crate::data::chats::chat::ChatId;

#[derive(Clone, DefaultNone, Debug)]
pub enum ChatAction {
    // Start a new chat, no entity specified
    StartWithoutEntity,
    // Start a new chat with a given entity
    Start(BotId),
    // Select the existing chat for a given entity, or create one if it does not exist yet
    #[allow(dead_code)]
    StartOrSelect(BotId),
    // Select a chat from the chat history
    ChatSelected(ChatId),
    None,
}

#[derive(Clone, DefaultNone, Debug)]
pub enum DownloadAction {
    Play(FileId),
    Pause(FileId),
    Cancel(FileId),
    None,
}

/// Bridges async Telegram Bot API outbound events into Makepad's UI actions.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, DefaultNone, Debug)]
pub enum BotOutboundAction {
    MessageReceived {
        bot_token: String,
        message: Box<moly_kit::aitk::telegram_server::Message>,
    },
    MessageEdited {
        bot_token: String,
        message_id: i64,
        new_text: String,
        reply_markup:
            Option<moly_kit::aitk::telegram_server::InlineKeyboardMarkup>,
    },
    MessageDeleted {
        bot_token: String,
        message_id: i64,
    },
    None,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_action_separates_new_chat_from_sidebar_selection() {
        let start = ChatAction::Start(BotId::new("model"));
        let select = ChatAction::StartOrSelect(BotId::new("telegram_bot/token"));

        assert!(matches!(start, ChatAction::Start(_)));
        assert!(matches!(select, ChatAction::StartOrSelect(_)));
    }
}
