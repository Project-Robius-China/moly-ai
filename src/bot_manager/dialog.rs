//! Dialog state machine and command handling for BotFather.

use moly_kit::aitk::telegram_server::{BotInfo, BotStore, BotUpdate};

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
) -> String {
    let trimmed = input.trim();

    // Commands always take priority, resetting any active wizard.
    if let Some(response) = try_command(trimmed, state, store, server_port) {
        return response;
    }

    // Otherwise, handle input based on current state.
    match state.clone() {
        DialogState::Idle => handle_unknown(trimmed),
        DialogState::AwaitingBotName => {
            handle_awaiting_bot_name(state, trimmed)
        }
        DialogState::AwaitingUsername { name } => {
            handle_awaiting_username(state, &name, trimmed, store, server_port)
        }
        DialogState::ManagingBot { token } => {
            handle_managing_bot_input(state, &token, trimmed, store, server_port)
        }
        DialogState::AwaitingNewName { token } => {
            handle_awaiting_new_name(state, &token, trimmed, store)
        }
        DialogState::ConfirmingDelete { token } => {
            handle_confirming_delete(state, &token, trimmed, store)
        }
    }
}

/// Attempts to match a `/command` and returns `Some(response)` if matched.
fn try_command(
    input: &str,
    state: &mut DialogState,
    store: &BotStore,
    _server_port: u16,
) -> Option<String> {
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
            Some(
                "Alright, a new bot. Please choose a name for your bot:"
                    .to_string(),
            )
        }
        "/mybots" => {
            *state = DialogState::Idle;
            Some(cmd_mybots(store))
        }
        "/cancel" => {
            *state = DialogState::Idle;
            Some(
                "Operation cancelled. Send /start to see available commands."
                    .to_string(),
            )
        }
        _ => {
            *state = DialogState::Idle;
            Some(format!(
                "Unknown command: {cmd}\n\n\
                 Send /start to see the list of available commands."
            ))
        }
    }
}

fn cmd_start() -> String {
    "\
Welcome to BotFather! I can help you create and manage bots.

Available commands:
/newbot — Create a new bot
/mybots — Manage your bots
/cancel — Cancel current operation
/help — Show help"
        .to_string()
}

fn cmd_mybots(store: &BotStore) -> String {
    match store.list_bots() {
        Ok(bots) if bots.is_empty() => {
            "You haven't created any bots yet.\n\n\
             Use /newbot to create your first bot!"
                .to_string()
        }
        Ok(bots) => {
            let mut msg = format!(
                "Your bots ({} total):\n\n",
                bots.len()
            );
            for (i, bot) in bots.iter().enumerate() {
                msg.push_str(&format!(
                    "{}. {} (@{})\n",
                    i + 1,
                    bot.name,
                    bot.username
                ));
            }
            msg.push_str(
                "\nEnter a number or username to manage a bot.",
            );
            msg
        }
        Err(e) => format!("Failed to list bots: {e}"),
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
    "Good. Now please choose a username for your bot.\n\n\
     It must end with 'bot' or '_bot' \
     (only lowercase letters, digits, and underscores, 3-32 chars):"
        .to_string()
}

fn handle_awaiting_username(
    state: &mut DialogState,
    name: &str,
    username: &str,
    store: &BotStore,
    server_port: u16,
) -> String {
    if let Err(reason) = validate_username(username) {
        return format!("{reason}\n\nPlease enter a different username:");
    }

    match store.create_bot(name, username) {
        Ok(bot) => {
            *state = DialogState::Idle;
            format_bot_created(&bot, server_port)
        }
        Err(e) => {
            let msg = e.to_string();
            if msg.contains("already taken") {
                format!(
                    "The username @{username} is already taken. \
                     Please choose another:"
                )
            } else {
                *state = DialogState::Idle;
                format!("Failed to create bot: {e}")
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

fn format_bot_created(bot: &BotInfo, server_port: u16) -> String {
    format!(
        "Done! Your new bot \"{name}\" (@{username}) is ready.\n\n\
         Token: `{token}`\n\n\
         To connect with crew-rs:\n\
         1. Set API URL to: http://localhost:{server_port}\n\
         2. Paste the token above into your crew-rs config\n\n\
         Use /mybots to manage your bots.",
        name = bot.name,
        username = bot.username,
        token = bot.token,
    )
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
            "Are you sure you want to delete this bot? \
             This cannot be undone.\n\n\
             Type \"confirm delete\" to confirm, or /cancel to cancel."
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
                "Bot \"{name}\" (@{username}) token:\n\n\
                 `{token}`\n\n\
                 To use with crew-rs:\n\
                 1. Set API URL to: http://localhost:{server_port}\n\
                 2. Paste the token above into your crew-rs config",
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
                "Token has been revoked. New token:\n\n\
                 `{new_token}`\n\n\
                 Please update the token in your crew-rs config.\n\
                 API URL: http://localhost:{server_port}"
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
                "Name updated to \"{}\" (@{}).",
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
                "Bot has been deleted.".to_string()
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
) -> String {
    let bots = match store.list_bots() {
        Ok(b) => b,
        Err(e) => return format!("Failed to list bots: {e}"),
    };

    if bots.is_empty() {
        *state = DialogState::Idle;
        return "No bots to manage. Use /newbot to create one.".to_string();
    }

    // Try to match by index (1-based)
    if let Ok(idx) = input.parse::<usize>()
        && idx >= 1
        && idx <= bots.len()
    {
        let bot = &bots[idx - 1];
        return enter_management_menu(state, bot);
    }

    // Try to match by @username
    let username = input.trim_start_matches('@');
    if let Some(bot) = bots.iter().find(|b| b.username == username) {
        return enter_management_menu(state, bot);
    }

    format!(
        "No matching bot found. \
         Please enter a valid number (1-{}) or username:",
        bots.len()
    )
}

fn enter_management_menu(state: &mut DialogState, bot: &BotInfo) -> String {
    *state = DialogState::ManagingBot {
        token: bot.token.clone(),
    };
    format!(
        "Managing bot \"{}\" (@{}):\n\n\
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

    fn test_store() -> BotStore {
        BotStore::open(&BotStoreConfig {
            db_path: ":memory:".into(),
            media_dir: String::new(),
        })
        .expect("open in-memory db")
    }

    #[test]
    fn test_botfather_start_command() {
        let store = test_store();
        let mut state = DialogState::default();
        let response = process_input(&mut state, "/start", &store, 8488);
        assert!(response.contains("Welcome"));
        assert!(response.contains("/newbot"));
        assert!(response.contains("/mybots"));
    }

    #[test]
    fn test_botfather_newbot_flow() {
        let store = test_store();
        let mut state = DialogState::default();

        let r = process_input(&mut state, "/newbot", &store, 8488);
        assert!(r.contains("name"));

        let r = process_input(&mut state, "Weather Bot", &store, 8488);
        assert!(r.contains("username"));

        let r = process_input(&mut state, "weather_bot", &store, 8488);
        assert!(r.contains("Done"));
        assert!(r.contains("Token"));

        // Verify bot exists in store
        let bots = store.list_bots().unwrap();
        assert_eq!(bots.len(), 1);
        assert_eq!(bots[0].name, "Weather Bot");
        assert_eq!(bots[0].username, "weather_bot");
    }

    #[test]
    fn test_botfather_newbot_invalid_username() {
        let store = test_store();
        let mut state = DialogState::AwaitingUsername {
            name: "Test".into(),
        };

        let r = process_input(
            &mut state,
            "invalid name with spaces",
            &store,
            8488,
        );
        assert!(r.contains("lowercase"));
        assert!(matches!(
            state,
            DialogState::AwaitingUsername { .. }
        ));
    }

    #[test]
    fn test_botfather_newbot_duplicate_username() {
        let store = test_store();
        store.create_bot("First", "weather_bot").unwrap();

        let mut state = DialogState::AwaitingUsername {
            name: "Second".into(),
        };
        let r = process_input(&mut state, "weather_bot", &store, 8488);
        assert!(r.contains("already taken"));
        assert!(matches!(
            state,
            DialogState::AwaitingUsername { .. }
        ));
    }

    #[test]
    fn test_botfather_mybots() {
        let store = test_store();
        store.create_bot("Weather Bot", "weather_bot").unwrap();
        store.create_bot("Code Bot", "code_bot").unwrap();

        let mut state = DialogState::default();
        let r = process_input(&mut state, "/mybots", &store, 8488);
        assert!(r.contains("2"));
        assert!(r.contains("Weather Bot"));
        assert!(r.contains("Code Bot"));
    }

    #[test]
    fn test_botfather_mybots_empty() {
        let store = test_store();
        let mut state = DialogState::default();
        let r = process_input(&mut state, "/mybots", &store, 8488);
        assert!(r.contains("haven't created any bots"));
        assert!(r.contains("/newbot"));
    }

    #[test]
    fn test_botfather_token_command() {
        let store = test_store();
        let bot = store.create_bot("Weather Bot", "weather_bot").unwrap();

        let mut state = DialogState::ManagingBot {
            token: bot.token.clone(),
        };
        let r = process_input(&mut state, "1", &store, 8488);
        assert!(r.contains(&bot.token));
        assert!(r.contains("crew-rs"));
    }

    #[test]
    fn test_botfather_revoke_token() {
        let store = test_store();
        let bot = store.create_bot("Weather Bot", "weather_bot").unwrap();
        let old_token = bot.token.clone();

        let mut state = DialogState::ManagingBot {
            token: old_token.clone(),
        };
        let r = process_input(&mut state, "3", &store, 8488);
        assert!(r.contains("revoked"));
        assert!(!r.contains(&old_token));

        // Old token should no longer work
        assert!(store.get_bot_by_token(&old_token).unwrap().is_none());
    }

    #[test]
    fn test_botfather_setname() {
        let store = test_store();
        let bot = store.create_bot("Weather Bot", "weather_bot").unwrap();

        let mut state = DialogState::ManagingBot {
            token: bot.token.clone(),
        };
        // Select "Edit Name"
        let r = process_input(&mut state, "2", &store, 8488);
        assert!(r.contains("new name"));

        let r = process_input(
            &mut state,
            "Weather Master",
            &store,
            8488,
        );
        assert!(r.contains("updated"));
        assert!(r.contains("Weather Master"));
    }

    #[test]
    fn test_botfather_deletebot_confirms() {
        let store = test_store();
        let bot = store.create_bot("Weather Bot", "weather_bot").unwrap();

        let mut state = DialogState::ManagingBot {
            token: bot.token.clone(),
        };
        // Select "Delete Bot"
        let r = process_input(&mut state, "4", &store, 8488);
        assert!(r.contains("Are you sure"));

        let r = process_input(&mut state, "confirm delete", &store, 8488);
        assert!(r.contains("deleted"));

        // Verify bot is gone
        assert!(store.list_bots().unwrap().is_empty());
    }

    #[test]
    fn test_botfather_unknown_command() {
        let store = test_store();
        let mut state = DialogState::default();
        let r = process_input(&mut state, "some random text", &store, 8488);
        assert!(r.contains("/start"));
    }

    #[test]
    fn test_username_validation() {
        assert!(validate_username("ab").is_err()); // too short
        assert!(validate_username("a".repeat(33).as_str()).is_err()); // too long
        assert!(validate_username("UPPER_bot").is_err()); // uppercase
        assert!(validate_username("no spaces bot").is_err()); // spaces
        assert!(validate_username("noending").is_err()); // no bot suffix
        assert!(validate_username("good_bot").is_ok());
        assert!(validate_username("mybot").is_ok());
        assert!(validate_username("test123_bot").is_ok());
    }
}
