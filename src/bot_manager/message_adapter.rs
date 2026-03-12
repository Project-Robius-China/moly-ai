//! Converts between Telegram and AITK message types.

use moly_kit::aitk::telegram_server::{
    InlineKeyboardMarkup,
    Message as TelegramMessage,
};
use moly_kit::prelude::*;

/// Converts a Telegram `Message` into an AITK `Message`.
/// Stores `message_id` in `MessageContent.data` for edit/delete lookups.
pub fn telegram_to_aitk_message(
    msg: &TelegramMessage,
    bot_id: &BotId,
) -> Message {
    let text = msg
        .text
        .clone()
        .or_else(|| msg.caption.clone())
        .unwrap_or_default();

    let quick_replies = msg
        .reply_markup
        .as_ref()
        .map(inline_keyboard_to_quick_replies)
        .unwrap_or_default();

    Message {
        from: EntityId::Bot(bot_id.clone()),
        content: MessageContent {
            text,
            quick_replies,
            // Store the Telegram message_id for edit/delete lookups
            data: Some(msg.message_id.to_string()),
            ..Default::default()
        },
        ..Default::default()
    }
}

/// Flattens an `InlineKeyboardMarkup` into `QuickReplyButton`s.
/// Callback data is prefixed with `cb:` to distinguish from regular text.
pub fn inline_keyboard_to_quick_replies(
    kb: &InlineKeyboardMarkup,
) -> Vec<QuickReplyButton> {
    kb.inline_keyboard
        .iter()
        .flatten()
        .map(|btn| {
            let action = if let Some(cb) = &btn.callback_data {
                format!("cb:{cb}")
            } else if let Some(url) = &btn.url {
                url.clone()
            } else {
                btn.text.clone()
            };

            QuickReplyButton {
                label: btn.text.clone(),
                action,
                style: ButtonStyle::Primary,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use moly_kit::aitk::telegram_server::{
        Chat as TgChat, InlineKeyboardButton,
    };

    fn make_telegram_message(text: &str) -> TelegramMessage {
        TelegramMessage {
            message_id: 42,
            from: None,
            chat: TgChat {
                id: 1,
                chat_type: "private".to_string(),
                title: None,
                first_name: None,
                username: None,
            },
            date: 1000,
            text: Some(text.to_string()),
            caption: None,
            photo: None,
            voice: None,
            audio: None,
            document: None,
            reply_markup: None,
        }
    }

    #[test]
    fn test_telegram_to_aitk_basic() {
        let tg_msg = make_telegram_message("Hello from bot");
        let bot_id = BotId::new("test_bot");
        let msg = telegram_to_aitk_message(&tg_msg, &bot_id);

        assert_eq!(msg.content.text, "Hello from bot");
        assert_eq!(msg.from, EntityId::Bot(bot_id));
        assert_eq!(msg.content.data, Some("42".to_string()));
        assert!(msg.content.quick_replies.is_empty());
    }

    #[test]
    fn test_inline_keyboard_to_quick_replies() {
        let kb = InlineKeyboardMarkup {
            inline_keyboard: vec![vec![
                InlineKeyboardButton {
                    text: "Option A".to_string(),
                    callback_data: Some("a".to_string()),
                    url: None,
                },
                InlineKeyboardButton {
                    text: "Visit".to_string(),
                    callback_data: None,
                    url: Some("https://example.com".to_string()),
                },
            ]],
        };

        let replies = inline_keyboard_to_quick_replies(&kb);
        assert_eq!(replies.len(), 2);
        assert_eq!(replies[0].label, "Option A");
        assert_eq!(replies[0].action, "cb:a");
        assert_eq!(replies[1].label, "Visit");
        assert_eq!(replies[1].action, "https://example.com");
    }
}
