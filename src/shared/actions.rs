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
