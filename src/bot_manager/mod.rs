//! BotFather dialog manager — a local bot for creating and managing other bots.
//!
//! BotFather replicates the Telegram @BotFather interaction pattern:
//! users send `/command` messages and BotFather responds with guided
//! dialog steps to create, edit, and manage bots.

mod client;
mod dialog;

pub use client::BotFatherClient;
