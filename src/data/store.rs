use crate::app::app_runner;
use crate::data::providers::ProviderId;
use crate::shared::actions::ChatAction;
use crate::shared::bot_context::BotContext;

use super::chats::chat::ChatId;
use super::downloads::download::DownloadFileAction;
use super::mcp_servers::McpServersConfig;
use super::moly_client::MolyClient;
use super::preferences::Preferences;
use super::providers::{ProviderFetchModelsResult, ProviderType};
use super::search::SortCriteria;
use super::supported_providers;
use super::{chats::Chats, downloads::Downloads, search::Search};
use chrono::{DateTime, Utc};
use makepad_widgets::{Action, ActionDefaultRef, DefaultNone};
use moly_kit::aitk::utils::asynchronous::spawn;
use moly_kit::prelude::*;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;

use super::providers::{Provider, ProviderConnectionStatus};
use moly_protocol::data::{Author, File, FileId, Model, ModelId, PendingDownload};

use makepad_widgets::*;

#[allow(dead_code)]
const DEFAULT_MOFA_ADDRESS: &str = "http://localhost:8000";

#[derive(Clone, DefaultNone, Debug)]
pub enum StoreAction {
    Search(String),
    ResetSearch,
    Sort(SortCriteria),
    None,
}

#[derive(Clone, DefaultNone, Debug)]
pub enum BotServerAction {
    Restarted(u16),
    RestartFailed { port: u16, message: String },
    None,
}

#[derive(Clone, Debug)]
pub struct FileWithDownloadInfo {
    pub file: File,
    pub download: Option<PendingDownload>,
}

#[derive(Clone, Debug)]
pub struct ModelWithDownloadInfo {
    pub model_id: ModelId,
    pub name: String,
    pub summary: String,
    pub size: String,
    pub requires: String,
    pub architecture: String,
    pub released_at: DateTime<Utc>,
    pub author: Author,
    pub like_count: u32,
    pub download_count: u32,
    pub files: Vec<FileWithDownloadInfo>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ProviderSyncingStatus {
    NotSyncing,
    Syncing(ProviderSyncing),
    Synced,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProviderSyncing {
    pub current: u32,
    pub total: u32,
}

pub struct Store {
    pub search: Search,
    pub downloads: Downloads,
    pub chats: Chats,
    pub preferences: Preferences,
    pub bot_context: Option<BotContext>,
    moly_client: MolyClient,
    pub provider_syncing_status: ProviderSyncingStatus,

    pub provider_icons: Vec<LiveDependency>,

    /// Shared state of the Telegram Bot API server (native only).
    #[cfg(not(target_arch = "wasm32"))]
    pub bot_server_state: Option<Arc<moly_kit::aitk::telegram_server::ServerState>>,
    /// Handle to the running server; Drop triggers shutdown.
    #[cfg(not(target_arch = "wasm32"))]
    _bot_server_handle: Option<moly_kit::aitk::telegram_server::ServerHandle>,

    /// Cached bot token → name mapping to avoid SQLite queries in draw paths.
    #[cfg(not(target_arch = "wasm32"))]
    bot_name_cache: std::collections::HashMap<String, String>,
}

const MOLY_SERVER_VERSION_EXTENSION: &str = "/api/v1";

impl Store {
    pub fn load_into_app() {
        spawn(async move {
            let preferences = Preferences::load().await;

            let server_port = std::env::var("MOLY_SERVER_PORT")
                .ok()
                .and_then(|p| p.parse::<u16>().ok())
                .unwrap_or(8765);

            let moly_client = MolyClient::new(format!("http://localhost:{}", server_port));

            let chats = Chats::load(moly_client.clone()).await;

            // Start Telegram Bot API server (native only).
            #[cfg(not(target_arch = "wasm32"))]
            let (bot_server_state, bot_server_handle) = {
                use moly_kit::aitk::telegram_server::{
                    OutboundEvent, ServerConfig, TelegramBotApiServer,
                };

                let data_dir = {
                    use directories::ProjectDirs;
                    let dirs = ProjectDirs::from("com", "moly-ai", "moly")
                        .expect("Failed to determine app data directory");
                    dirs.data_dir().join("bot_server")
                };
                let _ = std::fs::create_dir_all(&data_dir);

                let config = ServerConfig {
                    port: preferences.bot_server_port,
                    db_path: data_dir
                        .join("moly_bots.db")
                        .to_string_lossy()
                        .into_owned(),
                    media_dir: data_dir
                        .join("media")
                        .to_string_lossy()
                        .into_owned(),
                };

                match TelegramBotApiServer::start(config).await {
                    Ok((handle, state, mut outbound_rx)) => {
                        ::log::info!(
                            "Telegram Bot API server listening on http://{}",
                            handle.addr,
                        );

                        spawn(async move {
                            use crate::shared::actions::BotOutboundAction;
                            use futures::StreamExt as _;
                            while let Some(event) = outbound_rx.next().await {
                                match event {
                                    OutboundEvent::SendMessage {
                                        bot_token,
                                        chat_id: _chat_id,
                                        message,
                                    } => {
                                        ::log::info!("[bot-outbound] SendMessage");
                                        Cx::post_action(
                                            BotOutboundAction::MessageReceived {
                                                bot_token,
                                                message: Box::new(message),
                                            },
                                        );
                                    }
                                    OutboundEvent::EditMessage {
                                        bot_token,
                                        chat_id: _chat_id,
                                        message_id,
                                        new_text,
                                        reply_markup,
                                    } => {
                                        ::log::info!(
                                            "[bot-outbound] EditMessage msg={message_id}",
                                        );
                                        Cx::post_action(
                                            BotOutboundAction::MessageEdited {
                                                bot_token,
                                                message_id,
                                                new_text,
                                                reply_markup,
                                            },
                                        );
                                    }
                                    OutboundEvent::DeleteMessage {
                                        bot_token,
                                        chat_id: _chat_id,
                                        message_id,
                                    } => {
                                        ::log::info!(
                                            "[bot-outbound] DeleteMessage msg={message_id}",
                                        );
                                        Cx::post_action(
                                            BotOutboundAction::MessageDeleted {
                                                bot_token,
                                                message_id,
                                            },
                                        );
                                    }
                                }
                            }
                        });

                        (Some(state), Some(handle))
                    }
                    Err(e) => {
                        ::log::error!("Failed to start Telegram Bot API server: {e}");
                        (None, None)
                    }
                }
            };

            let mut store = Self {
                search: Search::new(moly_client.clone()),
                downloads: Downloads::new(moly_client.clone()),
                chats,
                moly_client,
                preferences,
                bot_context: None,
                provider_syncing_status: ProviderSyncingStatus::NotSyncing,
                provider_icons: vec![],
                #[cfg(not(target_arch = "wasm32"))]
                bot_server_state,
                #[cfg(not(target_arch = "wasm32"))]
                _bot_server_handle: bot_server_handle,
                #[cfg(not(target_arch = "wasm32"))]
                bot_name_cache: std::collections::HashMap::new(),
            };

            #[cfg(not(target_arch = "wasm32"))]
            store.refresh_bot_name_cache();

            store.init_current_chat();
            store.sync_with_moly_server();
            store.load_preference_connections();

            app_runner().defer(move |app, cx, _| {
                app.store = Some(store);
                app.ui.view(ids!(body)).set_visible(cx, true);
                cx.redraw_all(); // app.ui.redraw(cx) doesn't work as expected on web.
            });
        })
    }

    /// Check if the main moly server provider is enabled in settings.
    pub fn is_moly_server_enabled(&self) -> bool {
        self.preferences.providers_preferences.iter().any(|p| {
            p.provider_type == ProviderType::MolyServer
                && p.enabled
                && p.url.starts_with(&self.moly_client.address())
        })
    }

    /// Check if the connection to moly server was successful.
    pub fn is_moly_server_connected(&self) -> bool {
        self.moly_client.is_connected() && self.is_moly_server_enabled()
    }

    /// Pull the latest data from moly server.
    pub fn sync_with_moly_server(&mut self) {
        if !self.is_moly_server_enabled() {
            return;
        }

        let moly_client = self.moly_client.clone();
        spawn(async move {
            let Ok(()) = moly_client.test_connection().await else {
                return;
            };

            app_runner().defer(|app, _, _| {
                let store = app.store.as_mut().unwrap();
                store.downloads.load_downloaded_files();
                store.downloads.load_pending_downloads();
                store.search.load_featured_models();
            });
        });
    }

    pub fn get_chat_associated_bot(&self, chat_id: ChatId) -> Option<BotId> {
        self.chats
            .get_chat_by_id(chat_id)
            .and_then(|chat| chat.borrow().associated_bot.clone())
    }

    pub fn get_bot_display_name(&self, bot_id: &BotId) -> String {
        if let Some(bot) = self.chats.get_bot(bot_id) {
            return bot.human_readable_name().to_string();
        }

        #[cfg(not(target_arch = "wasm32"))]
        if let Some((provider_id, raw_bot_id)) = RouterClient::unprefix(bot_id)
        {
            if provider_id == "botfather" {
                return "BotFather".to_string();
            }

            if provider_id == "telegram_bot"
                && let Some(name) = self.bot_name_cache.get(raw_bot_id.as_str())
            {
                return name.clone();
            }
        }

        "Unknown".to_string()
    }

    /// Refreshes the in-memory bot name cache from SQLite.
    /// Call after bot creation, deletion, or rename operations.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn refresh_bot_name_cache(&mut self) {
        self.bot_name_cache.clear();

        let Some(state) = &self.bot_server_state else {
            return;
        };
        let Ok(bots) = state.store.list_bots() else {
            return;
        };

        for bot in bots {
            self.bot_name_cache
                .insert(bot.token, bot.name);
        }
    }

    /// Checks whether a raw bot token exists in the bot name cache.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn is_bot_token_known(&self, token: &str) -> bool {
        self.bot_name_cache.contains_key(token)
    }

    /// Restarts the Telegram Bot API server on a new port.
    ///
    /// Performs a graceful shutdown of the existing server and starts a new
    /// one with the updated port. Database and media paths remain unchanged,
    /// preserving all bot data.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn restart_bot_server(&mut self, new_port: u16) {
        use moly_kit::aitk::telegram_server::{
            OutboundEvent, ServerConfig, TelegramBotApiServer,
        };

        if self.preferences.bot_server_port == new_port {
            return;
        }

        let data_dir = {
            use directories::ProjectDirs;
            let dirs = ProjectDirs::from("com", "moly-ai", "moly")
                .expect("Failed to determine app data directory");
            dirs.data_dir().join("bot_server")
        };

        let config = ServerConfig {
            port: new_port,
            db_path: data_dir
                .join("moly_bots.db")
                .to_string_lossy()
                .into_owned(),
            media_dir: data_dir
                .join("media")
                .to_string_lossy()
                .into_owned(),
        };

        spawn(async move {
            match TelegramBotApiServer::start(config).await {
                Ok((handle, state, mut outbound_rx)) => {
                    ::log::info!(
                        "Bot server restarted on http://{}",
                        handle.addr,
                    );

                    spawn(async move {
                        use crate::shared::actions::BotOutboundAction;
                        use futures::StreamExt as _;
                        while let Some(event) = outbound_rx.next().await {
                            match event {
                                OutboundEvent::SendMessage {
                                    bot_token,
                                    chat_id: _,
                                    message,
                                } => {
                                    Cx::post_action(
                                        BotOutboundAction::MessageReceived {
                                            bot_token,
                                            message: Box::new(message),
                                        },
                                    );
                                }
                                OutboundEvent::EditMessage {
                                    bot_token,
                                    chat_id: _,
                                    message_id,
                                    new_text,
                                    reply_markup,
                                } => {
                                    Cx::post_action(
                                        BotOutboundAction::MessageEdited {
                                            bot_token,
                                            message_id,
                                            new_text,
                                            reply_markup,
                                        },
                                    );
                                }
                                OutboundEvent::DeleteMessage {
                                    bot_token,
                                    chat_id: _,
                                    message_id,
                                } => {
                                    Cx::post_action(
                                        BotOutboundAction::MessageDeleted {
                                            bot_token,
                                            message_id,
                                        },
                                    );
                                }
                            }
                        }
                    });

                    app_runner().defer(move |app, _, _| {
                        let store = app.store.as_mut().unwrap();
                        store.preferences.bot_server_port = new_port;
                        store.preferences.save();
                        store.bot_server_state = Some(state);
                        store._bot_server_handle = Some(handle);
                        store.refresh_bot_name_cache();
                        store.reload_bot_context();
                        Cx::post_action(BotServerAction::Restarted(new_port));
                    });
                }
                Err(e) => {
                    ::log::error!("Failed to restart bot server: {e}");
                    Cx::post_action(BotServerAction::RestartFailed {
                        port: new_port,
                        message: e.to_string(),
                    });
                }
            }
        });
    }

    /// This function combines the search results information for a given model
    /// with the download information for the files of that model.
    pub fn add_download_info_to_model(&self, model: &Model) -> ModelWithDownloadInfo {
        let files = model
            .files
            .iter()
            .map(|file| {
                let download = self
                    .downloads
                    .pending_downloads
                    .iter()
                    .find(|d| d.file.id == file.id)
                    .cloned();

                FileWithDownloadInfo {
                    file: file.clone(),
                    download,
                }
            })
            .collect();

        ModelWithDownloadInfo {
            model_id: model.id.clone(),
            name: model.name.clone(),
            summary: model.summary.clone(),
            size: model.size.clone(),
            requires: model.requires.clone(),
            architecture: model.architecture.clone(),
            like_count: model.like_count,
            download_count: model.download_count,
            released_at: model.released_at,
            author: model.author.clone(),
            files,
        }
    }

    pub fn get_model_and_file_download(&self, file_id: &str) -> (Model, File) {
        if let Some(result) = self
            .downloads
            .get_model_and_file_for_pending_download(file_id)
        {
            result
        } else {
            self.search
                .get_model_and_file_from_search_results(file_id)
                .unwrap()
        }
    }

    pub fn delete_file(&mut self, file_id: FileId) {
        let moly_client = self.moly_client.clone();
        spawn(async move {
            let Ok(()) = moly_client.eject_model().await else {
                eprintln!("Eject model operation failed");
                return;
            };

            let Ok(()) = moly_client.delete_file(file_id.clone()).await else {
                eprintln!("Delete file operation failed");
                return;
            };

            app_runner().defer(move |app, _, _| {
                let store = app.store.as_mut().unwrap();
                store.downloads.load_downloaded_files();
                store.downloads.load_pending_downloads();
                store
                    .search
                    .update_downloaded_file_in_search_results(&file_id, false);
            });
        });
    }

    pub fn handle_action(&mut self, action: &Action) {
        self.search.handle_action(action);
        self.downloads.handle_action(action);

        if action.downcast_ref::<DownloadFileAction>().is_some() {
            self.update_downloads();
        }
    }

    fn update_downloads(&mut self) {
        let completed_download_ids = self.downloads.refresh_downloads_data();

        let mut address = self.moly_client.address().clone();
        address.push_str(MOLY_SERVER_VERSION_EXTENSION);

        if !completed_download_ids.is_empty() {
            // Find MolyServer provider
            let provider = self
                .chats
                .providers
                .values()
                .find(|p| {
                    p.url == address && p.provider_type == ProviderType::MolyServer && p.enabled
                })
                .cloned();

            if let Some(provider) = provider {
                self.chats.test_provider_and_fetch_models(
                    &provider.id,
                    &mut self.provider_syncing_status,
                );
            }
        }

        // For search results let's trust on our local cache, but updating
        // the downloaded state of the files
        for file_id in completed_download_ids {
            self.search
                .update_downloaded_file_in_search_results(&file_id, true);
        }
    }

    fn init_current_chat(&mut self) {
        if let Some(chat_id) = self.chats.get_last_selected_chat_id() {
            self.chats.set_current_chat(Some(chat_id));
            Cx::post_action(ChatAction::ChatSelected(chat_id));
        } else {
            self.chats.create_empty_chat(None);
            if let Some(chat_id) = self.chats.get_last_selected_chat_id() {
                Cx::post_action(ChatAction::ChatSelected(chat_id));
            }
        }
    }

    pub fn delete_chat(&mut self, chat_id: ChatId) {
        self.chats.remove_chat(chat_id);

        // TODO Decide proper behavior when deleting the current chat
        // For now, we just create a new empty chat because we don't fully
        // support having no chat selected
        self.init_current_chat();
    }

    pub fn handle_provider_connection_action(&mut self, result: ProviderFetchModelsResult) {
        if let ProviderFetchModelsResult::None = result {
            return;
        }
        let fetched_from_moly_server = self.chats.handle_provider_connection_result(
            result,
            &mut self.preferences,
            &mut self.provider_syncing_status,
        );
        if fetched_from_moly_server && !self.moly_client.is_connected() {
            self.sync_with_moly_server();
        }
    }

    /// Loads the preference connections from the preferences and registers them in the chats.
    pub fn load_preference_connections(&mut self) {
        let supported = supported_providers::load_supported_providers();
        let mut final_list = Vec::new();

        for s in &supported {
            let maybe_prefs = self
                .preferences
                .providers_preferences
                .iter()
                .find(|pp| pp.id == s.id || (pp.id.is_empty() && pp.url == s.url));

            if let Some(prefs) = maybe_prefs {
                final_list.push(Provider {
                    id: if !prefs.id.is_empty() {
                        prefs.id.clone()
                    } else {
                        s.id.clone()
                    },
                    name: s.name.clone(),
                    url: prefs.url.clone(),
                    api_key: prefs.api_key.clone(),
                    provider_type: s.provider_type.clone(),
                    connection_status: ProviderConnectionStatus::Disconnected,
                    enabled: prefs.enabled,
                    models: vec![],
                    was_customly_added: prefs.was_customly_added,
                    system_prompt: prefs.system_prompt.clone(),
                    tools_enabled: prefs.tools_enabled,
                });
            } else {
                // Known from supported_providers.json but user has no preferences
                final_list.push(Provider {
                    id: s.id.clone(),
                    name: s.name.clone(),
                    url: s.url.clone(),
                    api_key: None,
                    provider_type: s.provider_type.clone(),
                    connection_status: ProviderConnectionStatus::Disconnected,
                    enabled: false,
                    models: vec![],
                    was_customly_added: false,
                    system_prompt: None,
                    tools_enabled: true,
                });
            }
        }

        // Custom providers from preferences (not in the supported_providers.json)
        for pp in &self.preferences.providers_preferences {
            let is_custom = !supported
                .iter()
                .any(|sp| sp.id == pp.id || (pp.id.is_empty() && sp.url == pp.url));
            if is_custom {
                // Ensure provider has an ID
                let mut pp_clone = pp.clone();
                pp_clone.ensure_id();

                final_list.push(Provider {
                    id: pp_clone.id.clone(),
                    name: pp_clone.name.clone(),
                    url: pp_clone.url.clone(),
                    api_key: pp_clone.api_key.clone(),
                    provider_type: pp_clone.provider_type.clone(),
                    connection_status: ProviderConnectionStatus::Disconnected,
                    enabled: pp_clone.enabled,
                    models: vec![],
                    was_customly_added: pp_clone.was_customly_added,
                    system_prompt: pp_clone.system_prompt.clone(),
                    tools_enabled: pp_clone.tools_enabled,
                });
            }
        }

        for provider in final_list {
            self.chats.providers.insert(provider.id.clone(), provider);
        }

        // Auto-register bot providers (native only, requires server)
        #[cfg(not(target_arch = "wasm32"))]
        if self.bot_server_state.is_some() {
            let botfather_id = "botfather".to_string();
            if !self.chats.providers.contains_key(&botfather_id) {
                let provider = Provider {
                    id: botfather_id,
                    name: "BotFather".to_string(),
                    url: String::new(),
                    api_key: None,
                    provider_type: ProviderType::BotFather,
                    connection_status: ProviderConnectionStatus::Connected,
                    enabled: true,
                    models: vec![],
                    was_customly_added: false,
                    system_prompt: None,
                    tools_enabled: false,
                };
                self.chats.providers.insert(
                    provider.id.clone(),
                    provider.clone(),
                );
                self.chats.register_provider(
                    provider,
                    &mut self.provider_syncing_status,
                );
            }

            let telegram_bot_id = "telegram_bot".to_string();
            if !self.chats.providers.contains_key(&telegram_bot_id) {
                let provider = Provider {
                    id: telegram_bot_id,
                    name: "Telegram Bot".to_string(),
                    url: String::new(),
                    api_key: None,
                    provider_type: ProviderType::TelegramBot,
                    connection_status: ProviderConnectionStatus::Connected,
                    enabled: true,
                    models: vec![],
                    was_customly_added: false,
                    system_prompt: None,
                    tools_enabled: false,
                };
                self.chats.providers.insert(
                    provider.id.clone(),
                    provider.clone(),
                );
                self.chats.register_provider(
                    provider,
                    &mut self.provider_syncing_status,
                );
            }
        }

        self.auto_fetch_for_enabled_providers();
    }

    fn auto_fetch_for_enabled_providers(&mut self) {
        // Automatically fetch providers that are enabled and have an API key or are MoFa servers
        let ids_to_fetch: Vec<String> = self
            .preferences
            .providers_preferences
            .iter()
            // TODO: If the provider requires an API key, we should fetch only if the API key is set
            .filter(|pp| {
                pp.enabled
                    && (pp.api_key.is_some()
                        || pp.provider_type == ProviderType::MoFa
                        || pp.provider_type == ProviderType::DeepInquire
                        || pp.provider_type == ProviderType::OpenAiRealtime
                        || pp.url.starts_with("http://localhost"))
            })
            .map(|pp| {
                // Ensure we have an ID to use
                if !pp.id.is_empty() {
                    pp.id.clone()
                } else {
                    // Generate ID for backward compatibility
                    super::preferences::ProviderPreferences::generate_id_from_url_and_name(
                        &pp.url,
                        &pp.name,
                        &pp.provider_type,
                    )
                }
            })
            .collect();

        // Collect providers first to avoid borrow issues
        let providers_to_register: Vec<Provider> = ids_to_fetch
            .iter()
            .filter_map(|id| self.chats.providers.get(id).cloned())
            .collect();

        for provider in providers_to_register {
            self.chats
                .register_provider(provider, &mut self.provider_syncing_status);
        }
    }

    pub fn insert_or_update_provider(&mut self, provider: &Provider) {
        // Update in memory
        self.chats
            .insert_or_update_provider(provider, &mut self.provider_syncing_status);
        // Update in preferences (persist in disk)
        self.preferences.insert_or_update_provider(provider);
        // Update in MolyKit (to update the API key used by the client, if needed)
        // Because MolyKit does not currently expose an API to update the clients, we'll remove and recreate the entire bot context
        // TODO(MolyKit): Find a better way to do this
        self.reload_bot_context();
    }

    pub fn remove_provider(&mut self, provider_id: &ProviderId) {
        self.chats.remove_provider(provider_id);
        self.preferences.remove_provider(provider_id);
    }

    pub fn get_provider_icon(&self, provider_name: &str) -> Option<LiveDependency> {
        let base_name = normalize_provider_name(provider_name);

        self.provider_icons
            .iter()
            .find(|icon| {
                icon.as_str()
                    .to_lowercase()
                    .contains(&base_name.to_lowercase())
            })
            .cloned()
    }

    pub fn get_mcp_servers_config(&self) -> &McpServersConfig {
        &self.preferences.mcp_servers_config
    }

    pub fn get_mcp_servers_config_json(&self) -> String {
        self.preferences.get_mcp_servers_config_json()
    }

    /// Creates a new MCP tool manager and loads servers asynchronously
    /// Returns the manager immediately, loading happens in the background
    pub fn create_and_load_mcp_tool_manager(&self) -> McpManagerClient {
        let tool_manager = McpManagerClient::new();

        // Check if MCP servers are globally enabled
        if !self.preferences.get_mcp_servers_enabled() {
            // Return empty tool manager if globally disabled
            return tool_manager;
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let mcp_config = self.get_mcp_servers_config().clone();
            tool_manager.set_dangerous_mode_enabled(mcp_config.dangerous_mode_enabled);
            let tool_manager_clone = tool_manager.clone();

            spawn(async move {
                // Load MCP servers from configuration
                for (server_id, server_config) in mcp_config.list_enabled_servers() {
                    if let Some(transport) = server_config.to_transport() {
                        match tool_manager_clone.add_server(server_id, transport).await {
                            Ok(()) => {
                                ::log::debug!("Successfully added MCP server: {}", server_id);
                            }
                            Err(e) => {
                                ::log::error!("Failed to add MCP server '{}': {}", server_id, e);
                            }
                        }
                    }
                }
            });
        }

        tool_manager
    }

    pub fn update_mcp_servers_from_json(&mut self, json: &str) -> Result<(), serde_json::Error> {
        self.preferences.update_mcp_servers_from_json(json)?;
        self.update_mcp_tool_manager();

        Ok(())
    }

    pub fn update_mcp_tool_manager(&mut self) {
        let new_tool_manager = self.create_and_load_mcp_tool_manager();
        if let Some(ref mut bot_context_mut) = self.bot_context {
            bot_context_mut.set_tool_manager(new_tool_manager);
        }
    }

    pub fn set_mcp_servers_enabled(&mut self, enabled: bool) {
        self.preferences.set_mcp_servers_enabled(enabled);
        // Recreate bot context to apply the new MCP setting
        self.reload_bot_context();
    }

    pub fn set_mcp_servers_dangerous_mode_enabled(&mut self, enabled: bool) {
        self.preferences
            .set_mcp_servers_dangerous_mode_enabled(enabled);
        self.update_mcp_tool_manager();
    }

    /// Triggers a bot context reload by clearing it.
    /// The ChatScreen will automatically recreate it on the next event,
    /// applying updated filters (like enabled status changes).
    pub fn reload_bot_context(&mut self) {
        if self.bot_context.is_some() {
            self.bot_context = None;
        }
    }
}

/// Extracts the base provider name from provider variants for icon matching.
///
/// This allows provider variants like "OpenAI Realtime" and "OpenAI Image"
/// to share the same icon as the base "OpenAI" provider.
///
/// # Examples
/// - "OpenAI Realtime" -> "openai"
/// - "OpenAI Image" -> "openai"
/// - "Anthropic" -> "anthropic"
pub fn normalize_provider_name(name: &str) -> String {
    name.split_whitespace()
        .next()
        .unwrap_or(name)
        .to_lowercase()
}
