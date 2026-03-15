//! Dialog state machine and command handling for BotFather.

use moly_kit::aitk::telegram_server::{BotInfo, BotStore, BotUpdate};
use moly_kit::prelude::MessageContent;

/// Canonical command reference — the **single source of truth** for every
/// place that lists BotFather commands (welcome message, `/start`, `/help`,
/// and the settings panel).
pub const COMMAND_LIST: &str = "\
- `/newbot` — Create a new bot\n\
- `/mybots` — Manage your bots\n\
- `/start` — Show welcome message\n\
- `/help` — Show help\n\
- `/cancel` — Cancel current operation";

/// Build the canonical welcome text from [`COMMAND_LIST`].
pub fn welcome_text() -> String {
    format!(
        "Welcome to BotFather! I can help you create and manage bots.\n\n\
         Available commands:\n{COMMAND_LIST}"
    )
}

/// BotFather dialog states for multi-step wizards.
#[derive(Debug, Clone, Default)]
pub enum DialogState {
    /// No active wizard; ready for a new command.
    #[default]
    Idle,
    /// `/newbot` step 1: waiting for the bot's display name.
    AwaitingBotName,
    /// `/newbot` step 2: waiting for the bot's username.
    AwaitingUsername { name: String },
    /// Viewing a specific bot's management menu.
    ManagingBot { token: String },
    /// Waiting for a new display name for a bot.
    AwaitingNewName { token: String },
    /// Waiting for deletion confirmation.
    ConfirmingDelete { token: String },
}


/// Processes user input against the current dialog state and returns a response.
pub fn process_input(
    state: &mut DialogState,
    input: &str,
    store: &BotStore,
    server_port: u16,
) -> MessageContent {
    let trimmed = input.trim();

    // Commands always take priority, resetting any active wizard.
    if let Some(response) = try_command(trimmed, state, store, server_port) {
        return response;
    }

    // Otherwise, handle input based on current state.
    match state.clone() {
        DialogState::Idle => plain(handle_unknown(trimmed)),
        DialogState::AwaitingBotName => {
            plain(handle_awaiting_bot_name(state, trimmed))
        }
        DialogState::AwaitingUsername { name } => {
            handle_awaiting_username(state, &name, trimmed, store, server_port)
        }
        DialogState::ManagingBot { token } => {
            plain(handle_managing_bot_input(
                state,
                &token,
                trimmed,
                store,
                server_port,
            ))
        }
        DialogState::AwaitingNewName { token } => {
            plain(handle_awaiting_new_name(state, &token, trimmed, store))
        }
        DialogState::ConfirmingDelete { token } => {
            plain(handle_confirming_delete(state, &token, trimmed, store))
        }
    }
}

/// Wrap a plain text string into a [`MessageContent`] with no quick replies.
fn plain(text: String) -> MessageContent {
    MessageContent {
        text,
        ..Default::default()
    }
}

/// Attempts to match a `/command` and returns `Some(response)` if matched.
fn try_command(
    input: &str,
    state: &mut DialogState,
    store: &BotStore,
    _server_port: u16,
) -> Option<MessageContent> {
    if !input.starts_with('/') {
        return None;
    }

    let cmd = input.split_whitespace().next().unwrap_or("");
    match cmd {
        "/start" | "/help" => {
            *state = DialogState::Idle;
            Some(cmd_start())
        }
        "/newbot" => {
            *state = DialogState::AwaitingBotName;
            Some(plain(
                "Alright, a new bot. Please choose a name for your bot:"
                    .to_string(),
            ))
        }
        "/mybots" => {
            *state = DialogState::Idle;
            Some(cmd_mybots(store))
        }
        "/cancel" => {
            let was_idle =
                matches!(state, DialogState::Idle);
            *state = DialogState::Idle;
            let text = if was_idle {
                "No active operation to cancel. \
                 Send /start to see available commands."
            } else {
                "Operation cancelled. \
                 Send /start to see available commands."
            };
            Some(plain(text.to_string()))
        }
        _ => {
            *state = DialogState::Idle;
            Some(plain(format!(
                "Unknown command: {cmd}\n\n\
                 Send /start to see the list of available commands."
            )))
        }
    }
}

/// Returns the welcome message.
///
/// Used both as the `/start` response and the initial chat greeting.
pub(crate) fn welcome_message() -> MessageContent {
    cmd_start()
}

fn cmd_start() -> MessageContent {
    plain(welcome_text())
}

fn cmd_mybots(store: &BotStore) -> MessageContent {
    match store.list_bots() {
        Ok(bots) if bots.is_empty() => plain(
            "You haven't created any bots yet.\n\n\
             Use /newbot to create your first bot!"
                .to_string(),
        ),
        Ok(bots) => {
            let mut msg = format!(
                "**Your bots** ({} total)\n\n",
                bots.len()
            );
            for (i, bot) in bots.iter().enumerate() {
                msg.push_str(&format!(
                    "{}. **{}** — @{}\n",
                    i + 1,
                    bot.name,
                    bot.username
                ));
            }
            msg.push_str(
                "\nEnter a number or @username to manage a bot.",
            );
            plain(msg)
        }
        Err(e) => plain(format!("Failed to list bots: {e}")),
    }
}

fn handle_unknown(input: &str) -> String {
    format!(
        "I don't understand \"{input}\".\n\n\
         Send /start to see available commands."
    )
}

fn handle_awaiting_bot_name(state: &mut DialogState, name: &str) -> String {
    if name.is_empty() {
        return "Name cannot be empty. Please try again:".to_string();
    }
    *state = DialogState::AwaitingUsername {
        name: name.to_string(),
    };
    "Good. Now please choose a **username** for your bot.\n\n\
     Requirements:\n\
     - 3–32 characters\n\
     - Lowercase letters, digits, and underscores only\n\
     - Must end with `bot` or `_bot`"
        .to_string()
}

fn handle_awaiting_username(
    state: &mut DialogState,
    name: &str,
    username: &str,
    store: &BotStore,
    server_port: u16,
) -> MessageContent {
    if let Err(reason) = validate_username(username) {
        return plain(format!(
            "{reason}\n\nPlease enter a different username:"
        ));
    }

    match store.create_bot(name, username) {
        Ok(bot) => {
            *state = DialogState::Idle;
            format_bot_created(&bot, server_port)
        }
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("already taken") {
                plain(format!(
                    "The username @{username} is already taken. \
                     Please choose another:"
                ))
            } else {
                *state = DialogState::Idle;
                plain(format!("Failed to create bot: {e}"))
            }
        }
    }
}

/// Validates a bot username according to the spec rules:
/// 3-32 chars, lowercase letters/digits/underscores, must end with `bot` or `_bot`.
fn validate_username(username: &str) -> Result<(), String> {
    let len = username.len();
    if !(3..=32).contains(&len) {
        return Err(
            "Username must be between 3 and 32 characters.".to_string(),
        );
    }
    if !username
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        return Err(
            "Username can only contain lowercase letters, digits, \
             and underscores."
                .to_string(),
        );
    }
    if !username.ends_with("bot") && !username.ends_with("_bot") {
        return Err(
            "Username must end with 'bot' or '_bot'.".to_string(),
        );
    }
    Ok(())
}

fn format_bot_created(bot: &BotInfo, server_port: u16) -> MessageContent {
    plain(format!(
        "**Done!** Your new bot **{name}** (@{username}) is ready.\n\n\
         **Token:**\n```\n{token}\n```\n\n\
         **Connect with Octos:**\n\
         1. Set API URL to `http://localhost:{server_port}`\n\
         2. Paste the token above into your Octos config",
        name = bot.name,
        username = bot.username,
        token = bot.token,
    ))
}

fn handle_managing_bot_input(
    state: &mut DialogState,
    token: &str,
    input: &str,
    store: &BotStore,
    server_port: u16,
) -> String {
    match input {
        "1" => show_token(state, token, store, server_port),
        "2" => {
            *state = DialogState::AwaitingNewName {
                token: token.to_string(),
            };
            "Please enter a new name:".to_string()
        }
        "3" => revoke_token(state, token, store, server_port),
        "4" => {
            *state = DialogState::ConfirmingDelete {
                token: token.to_string(),
            };
            "**Are you sure?** This cannot be undone.\n\n\
             Type `confirm delete` to proceed, or /cancel to cancel."
                .to_string()
        }
        "5" => {
            *state = DialogState::Idle;
            "Exited management menu.".to_string()
        }
        _ => "Please enter an option number (1-5):".to_string(),
    }
}

fn show_token(
    state: &mut DialogState,
    token: &str,
    store: &BotStore,
    server_port: u16,
) -> String {
    match store.get_bot_by_token(token) {
        Ok(Some(bot)) => {
            *state = DialogState::Idle;
            format!(
                "**{name}** (@{username})\n\n\
                 **Token:**\n```\n{token}\n```\n\n\
                 **Connect with Octos:**\n\
                 1. Set API URL to `http://localhost:{server_port}`\n\
                 2. Paste the token above into your Octos config",
                name = bot.name,
                username = bot.username,
                token = bot.token,
            )
        }
        _ => {
            *state = DialogState::Idle;
            "Bot not found or has been deleted.".to_string()
        }
    }
}

fn revoke_token(
    state: &mut DialogState,
    old_token: &str,
    store: &BotStore,
    server_port: u16,
) -> String {
    match store.revoke_token(old_token) {
        Ok(new_token) => {
            *state = DialogState::Idle;
            format!(
                "**Token revoked.** New token:\n\n\
                 `{new_token}`\n\n\
                 **Update your Octos config:**\n\
                 - API URL: `http://localhost:{server_port}`\n\
                 - Replace the old token with the one above"
            )
        }
        Err(e) => {
            *state = DialogState::Idle;
            format!("Failed to revoke token: {e}")
        }
    }
}

fn handle_awaiting_new_name(
    state: &mut DialogState,
    token: &str,
    new_name: &str,
    store: &BotStore,
) -> String {
    if new_name.is_empty() {
        return "Name cannot be empty. Please try again:".to_string();
    }
    match store.update_bot(
        token,
        &BotUpdate {
            name: Some(new_name.to_string()),
            ..Default::default()
        },
    ) {
        Ok(bot) => {
            *state = DialogState::Idle;
            format!(
                "**Name updated** to **{}** (@{}).",
                bot.name, bot.username
            )
        }
        Err(e) => {
            *state = DialogState::Idle;
            format!("Failed to update name: {e}")
        }
    }
}

fn handle_confirming_delete(
    state: &mut DialogState,
    token: &str,
    input: &str,
    store: &BotStore,
) -> String {
    if input == "confirm delete" {
        match store.delete_bot(token) {
            Ok(()) => {
                *state = DialogState::Idle;
                "**Bot deleted.**".to_string()
            }
            Err(e) => {
                *state = DialogState::Idle;
                format!("Failed to delete bot: {e}")
            }
        }
    } else {
        *state = DialogState::Idle;
        "Deletion cancelled.".to_string()
    }
}

/// Resolves a user selection from `/mybots` list into a management menu.
///
/// `input` can be a 1-based index number or a `@username`.
pub fn resolve_bot_selection(
    state: &mut DialogState,
    input: &str,
    store: &BotStore,
) -> MessageContent {
    let bots = match store.list_bots() {
        Ok(b) => b,
        Err(e) => return plain(format!("Failed to list bots: {e}")),
    };

    if bots.is_empty() {
        *state = DialogState::Idle;
        return plain(
            "No bots to manage. Use /newbot to create one.".to_string(),
        );
    }

    // Try to match by index (1-based)
    if let Ok(idx) = input.parse::<usize>()
        && idx >= 1
        && idx <= bots.len()
    {
        let bot = &bots[idx - 1];
        return plain(enter_management_menu(state, bot));
    }

    // Try to match by @username
    let username = input.trim_start_matches('@');
    if let Some(bot) = bots.iter().find(|b| b.username == username) {
        return plain(enter_management_menu(state, bot));
    }

    plain(format!(
        "No matching bot found. \
         Please enter a valid number (1-{}) or username:",
        bots.len()
    ))
}

fn enter_management_menu(state: &mut DialogState, bot: &BotInfo) -> String {
    *state = DialogState::ManagingBot {
        token: bot.token.clone(),
    };
    format!(
        "**Managing** **{}** (@{})\n\n\
         1. View Token\n\
         2. Edit Name\n\
         3. Revoke Token\n\
         4. Delete Bot\n\
         5. Return\n\n\
         Enter an option number:",
        bot.name, bot.username
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use moly_kit::aitk::telegram_server::BotStoreConfig;

    fn store() -> BotStore {
        BotStore::open(&BotStoreConfig {
            db_path: ":memory:".into(),
            media_dir: String::new(),
        })
        .expect("open in-memory db")
    }

    #[test]
    fn test_start() {
        let store = store();
        let mut state = DialogState::default();
        let r = process_input(&mut state, "/start", &store, 8488);
        assert!(r.text.contains("Welcome"));
        assert!(r.text.contains("/newbot"));
        assert!(r.text.contains("/mybots"));
    }

    #[test]
    fn test_start_no_replies() {
        let store = store();
        let mut state = DialogState::default();
        let r = process_input(&mut state, "/start", &store, 8488);
        assert!(r.quick_replies.is_empty());
    }

    #[test]
    fn test_newbot_flow() {
        let store = store();
        let mut state = DialogState::default();

        let r = process_input(&mut state, "/newbot", &store, 8488);
        assert!(r.text.contains("name"));

        let r = process_input(&mut state, "Weather Bot", &store, 8488);
        assert!(r.text.contains("username"));

        let r = process_input(&mut state, "weather_bot", &store, 8488);
        assert!(r.text.contains("Done"));
        assert!(r.text.contains("Token"));

        // Verify bot exists in store
        let bots = store.list_bots().unwrap();
        assert_eq!(bots.len(), 1);
        assert_eq!(bots[0].name, "Weather Bot");
        assert_eq!(bots[0].username, "weather_bot");
    }

    #[test]
    fn test_token_block() {
        let store = store();
        let mut state = DialogState::default();
        let _ = process_input(&mut state, "/newbot", &store, 8488);
        let _ = process_input(&mut state, "Demo", &store, 8488);
        let r = process_input(&mut state, "demo_bot", &store, 8488);

        let bot = store.list_bots().unwrap().pop().unwrap();
        assert!(r.text.contains(&format!("```\n{}\n```", bot.token)));
        assert!(r.quick_replies.is_empty());
    }

    #[test]
    fn test_invalid_username() {
        let store = store();
        let mut state = DialogState::AwaitingUsername {
            name: "Test".into(),
        };

        let r = process_input(
            &mut state,
            "invalid name with spaces",
            &store,
            8488,
        );
        assert!(r.text.contains("lowercase"));
        assert!(matches!(
            state,
            DialogState::AwaitingUsername { .. }
        ));
    }

    #[test]
    fn test_duplicate_username() {
        let store = store();
        store.create_bot("First", "weather_bot").unwrap();

        let mut state = DialogState::AwaitingUsername {
            name: "Second".into(),
        };
        let r = process_input(&mut state, "weather_bot", &store, 8488);
        assert!(r.text.contains("already taken"));
        assert!(matches!(
            state,
            DialogState::AwaitingUsername { .. }
        ));
    }

    #[test]
    fn test_mybots() {
        let store = store();
        store.create_bot("Alpha", "alpha_bot").unwrap();
        store.create_bot("Beta", "beta_bot").unwrap();

        let mut state = DialogState::default();
        let r = process_input(&mut state, "/mybots", &store, 8488);
        assert!(r.text.contains("Your bots** (2 total)"));
        assert!(r.text.contains("1. **Alpha**"));
        assert!(r.text.contains("2. **Beta**"));
        assert!(r.text.contains("Enter a number"));
        assert!(r.quick_replies.is_empty());
    }

    #[test]
    fn test_empty_mybots() {
        let store = store();
        let mut state = DialogState::default();
        let r = process_input(&mut state, "/mybots", &store, 8488);
        assert!(r.text.contains("haven't created any bots"));
        assert!(r.text.contains("/newbot"));
        assert!(r.quick_replies.is_empty());
    }

    #[test]
    fn test_token() {
        let store = store();
        let bot = store.create_bot("Weather Bot", "weather_bot").unwrap();

        let mut state = DialogState::ManagingBot {
            token: bot.token.clone(),
        };
        let r = process_input(&mut state, "1", &store, 8488);
        assert!(r.text.contains(&format!("```\n{}\n```", bot.token)));
        assert!(r.text.contains("Octos"));
    }

    #[test]
    fn test_revoke_token() {
        let store = store();
        let bot = store.create_bot("Weather Bot", "weather_bot").unwrap();
        let old_token = bot.token.clone();

        let mut state = DialogState::ManagingBot {
            token: old_token.clone(),
        };
        let r = process_input(&mut state, "3", &store, 8488);
        assert!(r.text.contains("revoked"));
        assert!(!r.text.contains(&old_token));

        // Old token should no longer work
        assert!(store.get_bot_by_token(&old_token).unwrap().is_none());
    }

    #[test]
    fn test_setname() {
        let store = store();
        let bot = store.create_bot("Weather Bot", "weather_bot").unwrap();

        let mut state = DialogState::ManagingBot {
            token: bot.token.clone(),
        };
        let r = process_input(&mut state, "2", &store, 8488);
        assert!(r.text.contains("new name"));

        let r = process_input(
            &mut state,
            "Weather Master",
            &store,
            8488,
        );
        assert!(r.text.contains("updated"));
        assert!(r.text.contains("Weather Master"));
    }

    #[test]
    fn test_deletebot() {
        let store = store();
        let bot = store.create_bot("Weather Bot", "weather_bot").unwrap();

        let mut state = DialogState::ManagingBot {
            token: bot.token.clone(),
        };
        let r = process_input(&mut state, "4", &store, 8488);
        assert!(r.text.contains("Are you sure"));

        let r = process_input(&mut state, "confirm delete", &store, 8488);
        assert!(r.text.contains("deleted"));

        assert!(store.list_bots().unwrap().is_empty());
    }

    #[test]
    fn test_unknown_input() {
        let store = store();
        let mut state = DialogState::default();
        let r = process_input(&mut state, "some random text", &store, 8488);
        assert!(r.text.contains("/start"));
    }

    #[test]
    fn test_cancel_idle() {
        let store = store();
        let mut state = DialogState::default();
        let r = process_input(&mut state, "/cancel", &store, 8488);
        assert!(r.text.contains("No active operation"));
    }

    #[test]
    fn test_resolve() {
        let store = store();
        store.create_bot("Test", "test_bot").unwrap();
        let mut state = DialogState::default();
        let r = resolve_bot_selection(&mut state, "1", &store);
        assert!(r.text.contains("test_bot"));
        assert!(r.text.contains("Managing"));
    }

    #[test]
    fn test_username() {
        assert!(validate_username("ab").is_err());
        assert!(validate_username("a".repeat(33).as_str()).is_err());
        assert!(validate_username("UPPER_bot").is_err());
        assert!(validate_username("no spaces bot").is_err());
        assert!(validate_username("noending").is_err());
        assert!(validate_username("good_bot").is_ok());
        assert!(validate_username("mybot").is_ok());
        assert!(validate_username("test123_bot").is_ok());
    }
}
