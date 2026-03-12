//! Bot manager — local bots for the Telegram Bot API integration.
//!
//! BotFather replicates the Telegram @BotFather interaction pattern:
//! users send `/command` messages and BotFather responds with guided
//! dialog steps to create, edit, and manage bots.
//!
//! `TelegramBotClient` bridges user messages to bots created via BotFather,
//! pushing them as Telegram Updates for connected frameworks to poll.

mod client;
pub(crate) mod dialog;
pub(crate) mod message_adapter;
mod telegram_bot_client;

pub use client::BotFatherClient;
pub use telegram_bot_client::TelegramBotClient;
