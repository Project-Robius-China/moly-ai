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
                "好的，让我们来创建一个新 Bot。\n\n请给它起个名字：".to_string(),
            )
        }
        "/mybots" => {
            *state = DialogState::Idle;
            Some(cmd_mybots(store))
        }
        "/cancel" => {
            *state = DialogState::Idle;
            Some("操作已取消。发送 /start 查看可用命令。".to_string())
        }
        _ => {
            *state = DialogState::Idle;
            Some(format!(
                "未知命令: {cmd}\n\n发送 /start 查看可用命令列表。"
            ))
        }
    }
}

fn cmd_start() -> String {
    "\
欢迎使用 BotFather！我可以帮你创建和管理 Bot。

可用命令：
/newbot — 创建一个新 Bot
/mybots — 管理已有的 Bot
/cancel — 取消当前操作
/help — 显示帮助信息"
        .to_string()
}

fn cmd_mybots(store: &BotStore) -> String {
    match store.list_bots() {
        Ok(bots) if bots.is_empty() => {
            "你还没有创建任何 Bot。\n\n使用 /newbot 创建你的第一个 Bot！"
                .to_string()
        }
        Ok(bots) => {
            let mut msg = format!("你的 Bot（共 {} 个）：\n\n", bots.len());
            for (i, bot) in bots.iter().enumerate() {
                msg.push_str(&format!(
                    "{}. {} (@{})\n",
                    i + 1,
                    bot.name,
                    bot.username
                ));
            }
            msg.push_str(
                "\n输入序号或用户名来管理对应的 Bot：",
            );
            msg
        }
        Err(e) => format!("获取 Bot 列表失败: {e}"),
    }
}

fn handle_unknown(input: &str) -> String {
    format!(
        "不理解 \"{input}\"。\n\n发送 /start 查看可用命令列表。"
    )
}

fn handle_awaiting_bot_name(state: &mut DialogState, name: &str) -> String {
    if name.is_empty() {
        return "名称不能为空，请重新输入：".to_string();
    }
    *state = DialogState::AwaitingUsername {
        name: name.to_string(),
    };
    "好的。现在请给它起一个用户名（必须以 bot 结尾，\
     仅允许小写字母、数字和下划线，3-32 个字符）："
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
        return format!("{reason}\n\n请重新输入用户名：");
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
                    "用户名 @{username} 已被使用，请换一个用户名："
                )
            } else {
                *state = DialogState::Idle;
                format!("创建 Bot 失败: {e}")
            }
        }
    }
}

/// Validates a bot username according to the spec rules:
/// 3-32 chars, lowercase letters/digits/underscores, must end with `bot` or `_bot`.
fn validate_username(username: &str) -> Result<(), String> {
    let len = username.len();
    if !(3..=32).contains(&len) {
        return Err("用户名长度必须在 3 到 32 个字符之间。".to_string());
    }
    if !username
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        return Err(
            "用户名仅允许小写字母、数字和下划线。".to_string(),
        );
    }
    if !username.ends_with("bot") && !username.ends_with("_bot") {
        return Err(
            "用户名必须以 \"bot\" 或 \"_bot\" 结尾。".to_string(),
        );
    }
    Ok(())
}

fn format_bot_created(bot: &BotInfo, server_port: u16) -> String {
    format!(
        "已创建！你的新 Bot「{}」(@{}) 已就绪。\n\n\
         Token: `{}`\n\n\
         在 crew-rs 中使用此 Bot：\n\
         1. 设置 API URL 为: http://localhost:{server_port}\n\
         2. 将上面的 Token 填入 crew-rs 配置\n\n\
         使用 /mybots 管理你的 Bot。",
        bot.name, bot.username, bot.token
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
            "请输入新名称：".to_string()
        }
        "3" => revoke_token(state, token, store, server_port),
        "4" => {
            *state = DialogState::ConfirmingDelete {
                token: token.to_string(),
            };
            "确定要删除这个 Bot 吗？此操作不可恢复。\n\n\
             输入 \"确认删除\" 来确认，或 /cancel 取消。"
                .to_string()
        }
        "5" => {
            *state = DialogState::Idle;
            "已退出管理菜单。".to_string()
        }
        _ => "请输入选项序号（1-5）：".to_string(),
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
                "Bot「{}」(@{}) 的 Token：\n\n\
                 `{}`\n\n\
                 在 crew-rs 中使用：\n\
                 1. 设置 API URL 为: http://localhost:{server_port}\n\
                 2. 将上面的 Token 填入 crew-rs 配置",
                bot.name, bot.username, bot.token
            )
        }
        _ => {
            *state = DialogState::Idle;
            "Bot 不存在或已被删除。".to_string()
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
            // Update state to reference the new token
            *state = DialogState::Idle;
            format!(
                "Token 已重置。新 Token：\n\n\
                 `{new_token}`\n\n\
                 请更新 crew-rs 配置中的 Token。\n\
                 API URL: http://localhost:{server_port}"
            )
        }
        Err(e) => {
            *state = DialogState::Idle;
            format!("重置 Token 失败: {e}")
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
        return "名称不能为空，请重新输入：".to_string();
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
                "名称已更新为「{}」(@{})。",
                bot.name, bot.username
            )
        }
        Err(e) => {
            *state = DialogState::Idle;
            format!("更新名称失败: {e}")
        }
    }
}

fn handle_confirming_delete(
    state: &mut DialogState,
    token: &str,
    input: &str,
    store: &BotStore,
) -> String {
    if input == "确认删除" {
        match store.delete_bot(token) {
            Ok(()) => {
                *state = DialogState::Idle;
                "Bot 已删除。".to_string()
            }
            Err(e) => {
                *state = DialogState::Idle;
                format!("删除失败: {e}")
            }
        }
    } else {
        *state = DialogState::Idle;
        "已取消删除。".to_string()
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
        Err(e) => return format!("获取 Bot 列表失败: {e}"),
    };

    if bots.is_empty() {
        *state = DialogState::Idle;
        return "没有可管理的 Bot。使用 /newbot 创建一个。".to_string();
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
        "未找到匹配的 Bot。请输入正确的序号（1-{}）或用户名：",
        bots.len()
    )
}

fn enter_management_menu(state: &mut DialogState, bot: &BotInfo) -> String {
    *state = DialogState::ManagingBot {
        token: bot.token.clone(),
    };
    format!(
        "管理 Bot「{}」(@{})：\n\n\
         1. 查看 Token\n\
         2. 编辑名称\n\
         3. 重置 Token\n\
         4. 删除 Bot\n\
         5. 返回\n\n\
         请输入选项序号：",
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
        assert!(response.contains("欢迎"));
        assert!(response.contains("/newbot"));
        assert!(response.contains("/mybots"));
    }

    #[test]
    fn test_botfather_newbot_flow() {
        let store = test_store();
        let mut state = DialogState::default();

        let r = process_input(&mut state, "/newbot", &store, 8488);
        assert!(r.contains("名字"));

        let r = process_input(&mut state, "天气助手", &store, 8488);
        assert!(r.contains("用户名"));

        let r = process_input(&mut state, "weather_bot", &store, 8488);
        assert!(r.contains("已创建"));
        assert!(r.contains("Token"));

        // Verify bot exists in store
        let bots = store.list_bots().unwrap();
        assert_eq!(bots.len(), 1);
        assert_eq!(bots[0].name, "天气助手");
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
        assert!(r.contains("小写字母"));
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
        assert!(r.contains("已被使用"));
        assert!(matches!(
            state,
            DialogState::AwaitingUsername { .. }
        ));
    }

    #[test]
    fn test_botfather_mybots() {
        let store = test_store();
        store.create_bot("天气助手", "weather_bot").unwrap();
        store.create_bot("代码助手", "code_bot").unwrap();

        let mut state = DialogState::default();
        let r = process_input(&mut state, "/mybots", &store, 8488);
        assert!(r.contains("2"));
        assert!(r.contains("天气助手"));
        assert!(r.contains("代码助手"));
    }

    #[test]
    fn test_botfather_mybots_empty() {
        let store = test_store();
        let mut state = DialogState::default();
        let r = process_input(&mut state, "/mybots", &store, 8488);
        assert!(r.contains("没有创建任何 Bot"));
        assert!(r.contains("/newbot"));
    }

    #[test]
    fn test_botfather_token_command() {
        let store = test_store();
        let bot = store.create_bot("天气助手", "weather_bot").unwrap();

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
        let bot = store.create_bot("天气助手", "weather_bot").unwrap();
        let old_token = bot.token.clone();

        let mut state = DialogState::ManagingBot {
            token: old_token.clone(),
        };
        let r = process_input(&mut state, "3", &store, 8488);
        assert!(r.contains("已重置"));
        assert!(!r.contains(&old_token));

        // Old token should no longer work
        assert!(store.get_bot_by_token(&old_token).unwrap().is_none());
    }

    #[test]
    fn test_botfather_setname() {
        let store = test_store();
        let bot = store.create_bot("天气助手", "weather_bot").unwrap();

        let mut state = DialogState::ManagingBot {
            token: bot.token.clone(),
        };
        // Select "编辑名称"
        let r = process_input(&mut state, "2", &store, 8488);
        assert!(r.contains("新名称"));

        let r = process_input(&mut state, "天气预报大师", &store, 8488);
        assert!(r.contains("已更新"));
        assert!(r.contains("天气预报大师"));
    }

    #[test]
    fn test_botfather_deletebot_confirms() {
        let store = test_store();
        let bot = store.create_bot("天气助手", "weather_bot").unwrap();

        let mut state = DialogState::ManagingBot {
            token: bot.token.clone(),
        };
        // Select "删除 Bot"
        let r = process_input(&mut state, "4", &store, 8488);
        assert!(r.contains("确定要删除"));

        let r = process_input(&mut state, "确认删除", &store, 8488);
        assert!(r.contains("已删除"));

        // Verify bot is gone
        assert!(store.list_bots().unwrap().is_empty());
    }

    #[test]
    fn test_botfather_unknown_command() {
        let store = test_store();
        let mut state = DialogState::default();
        let r = process_input(&mut state, "随便说点什么", &store, 8488);
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
