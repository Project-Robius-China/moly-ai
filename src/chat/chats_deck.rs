use std::collections::{HashMap, VecDeque};

use makepad_widgets::*;
use moly_kit::prelude::*;

use super::chat_view::ChatViewRef;
use crate::chat::chat_view::ChatViewWidgetRefExt;
use crate::data::capture::CaptureAction;
use crate::data::chats::chat::Chat as ChatData;
use crate::data::chats::chat::ChatId;
use crate::data::store::Store;
use crate::shared::actions::ChatAction;
#[cfg(not(target_arch = "wasm32"))]
use crate::shared::actions::BotOutboundAction;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    use crate::shared::styles::*;
    use crate::shared::widgets::*;
    use crate::chat::chat_view::ChatView;

    pub ChatsDeck = {{ChatsDeck}} {
        width: Fill, height: Fill
        padding: {top: 18, bottom: 0, right: 28, left: 28},

        chat_view_template: <ChatView> {}
    }
}

#[derive(Live, LiveHook, Widget)]
pub struct ChatsDeck {
    #[deref]
    view: View,

    /// All currently active chat instances, keyed by their corresponding ChatID.
    /// Each chat maintains its own instance to keep background streaming alive.
    #[rust]
    chat_view_refs: HashMap<ChatId, ChatViewRef>,

    /// LRU tracking for memory management.
    /// When we exceed MAX_CHAT_VIEWS, we evict the oldest chat (unless it's streaming).
    #[rust]
    chat_view_accessed_order: VecDeque<ChatId>,

    /// The currently visible/focused chat id.
    #[rust]
    currently_visible_chat_id: Option<ChatId>,

    /// The template for creating new chat views.
    #[live]
    chat_view_template: Option<LivePtr>,

    /// HACK(telegram-reveal): Active reveal animations, keyed by
    /// `(ChatId, telegram_message_id)`.
    #[cfg(not(target_arch = "wasm32"))]
    #[rust]
    pending_reveals: HashMap<(ChatId, String), PendingReveal>,

    /// HACK(telegram-reveal): Single interval timer driving all active reveals.
    #[cfg(not(target_arch = "wasm32"))]
    #[rust]
    reveal_timer: Timer,
}

/// The maximum number of chat views that can be kept alive at once.
/// Prevents unbounded memory growth in long-running sessions.
const MAX_CHAT_VIEWS: usize = 10;

/// HACK(telegram-reveal): Characters to reveal per timer tick.
/// At 25ms interval, 3 chars/tick ≈ 120 chars/sec.
/// Remove when crew-rs adopts `sendMessageDraft` (Telegram Bot API 9.5).
#[cfg(not(target_arch = "wasm32"))]
const REVEAL_CHARS_PER_TICK: usize = 3;

/// HACK(telegram-reveal): Interval between reveal ticks in seconds.
#[cfg(not(target_arch = "wasm32"))]
const REVEAL_INTERVAL_SECS: f64 = 0.025;

/// HACK(telegram-reveal): Per-message reveal animation state.
///
/// Crew-rs throttles `editMessageText` to once per second (`EDIT_THROTTLE = 1000ms`),
/// causing bot responses to arrive in jarring ~1-second blocks. This struct drives a
/// client-side character-by-character reveal that smooths out the visual presentation.
///
/// The Store always receives the full text immediately (data integrity),
/// while the ChatController receives progressively revealed text via a Makepad
/// interval timer.
///
/// Remove when crew-rs adopts `sendMessageDraft` (Telegram Bot API 9.5).
#[cfg(not(target_arch = "wasm32"))]
struct PendingReveal {
    chat_id: ChatId,
    target_text: String,
    revealed_len: usize,
    pending_quick_replies: Vec<QuickReplyButton>,
    base_message: Message,
}

/// HACK(telegram-reveal): Snap a byte offset to the nearest valid UTF-8 char
/// boundary at or before `byte_offset`. Returns `s.len()` if offset exceeds
/// the string length.
#[cfg(not(target_arch = "wasm32"))]
fn snap_to_char_boundary(s: &str, byte_offset: usize) -> usize {
    if byte_offset >= s.len() {
        return s.len();
    }
    let mut pos = byte_offset;
    while pos > 0 && !s.is_char_boundary(pos) {
        pos -= 1;
    }
    pos
}

impl Widget for ChatsDeck {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
        self.widget_match_event(cx, event, scope);

        // HACK(telegram-reveal): Drive character reveal animations.
        #[cfg(not(target_arch = "wasm32"))]
        if self.reveal_timer.is_event(event).is_some() {
            self.tick_reveals(cx);
        }

        // Handle events for ALL instances to keep background activity (streaming, etc.) alive
        for (_, chat_view) in self.chat_view_refs.iter_mut() {
            chat_view.handle_event(cx, event, scope);
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        // Because chats_deck is being cached, overriding its properties in the DSL does not take effect.
        // For now we'll override them through apply_over.
        // TODO: Do not use CachedWidget, create a shared structure of chat instances that is shared across layouts.
        if cx.display_context.is_desktop() {
            self.view.apply_over(
                cx,
                live! {padding: {top: 18, bottom: 0, right: 28, left: 28} },
            );
        } else {
            self.view.apply_over(
                cx,
                live! { padding: {top: 55, left: 0, right: 0, bottom: 0} },
            );
        }

        cx.begin_turtle(walk, self.layout);

        // Draw only the currently visible chat
        if let Some(chat_id) = self.currently_visible_chat_id
            && let Some(chat_view) = self.chat_view_refs.get_mut(&chat_id) {
                let _ = chat_view.draw(cx, scope);
            }

        cx.end_turtle();
        DrawStep::done()
    }
}

impl WidgetMatchEvent for ChatsDeck {
    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions, scope: &mut Scope) {
        let store = scope.data.get_mut::<Store>().unwrap();
        for action in actions {
            // Handle chat start
            match action.cast() {
                ChatAction::Start(bot_id) => {
                    self.open_chat_for_bot(cx, store, &bot_id, false);
                }
                ChatAction::StartOrSelect(bot_id) => {
                    self.open_chat_for_bot(cx, store, &bot_id, true);
                }
                ChatAction::StartWithoutEntity => {
                    let chat_id = store.chats.create_empty_chat(None);
                    let chat = store.chats.get_chat_by_id(chat_id);
                    if let Some(chat) = chat {
                        self.create_or_update_chat_view(cx, &chat.borrow());
                    }
                }
                _ => {}
            }

            // Handle chat selection (from chat history)
            if let ChatAction::ChatSelected(chat_id) = action.cast() {
                let selected_chat = store.chats.get_chat_by_id(chat_id);

                if let Some(chat) = selected_chat {
                    store
                        .preferences
                        .set_current_chat_model(chat.borrow().associated_bot.clone());

                    self.create_or_update_chat_view(cx, &chat.borrow());
                }
            }

            // Handle Context Capture
            if let CaptureAction::Capture { event } = action.cast() {
                // Paste the captured text into the currently visible chat
                if let Some(chat_id) = self.currently_visible_chat_id
                    && let Some(chat_view) = self.chat_view_refs.get_mut(&chat_id) {
                        chat_view
                            .prompt_input(ids!(prompt))
                            .write()
                            .set_text(cx, event.contents());
                    }
            }

            // Handle bot outbound events (bot → UI)
            #[cfg(not(target_arch = "wasm32"))]
            self.handle_bot_outbound_action(cx, action, store);
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl ChatsDeck {
    fn handle_bot_outbound_action(
        &mut self,
        cx: &mut Cx,
        action: &Action,
        store: &mut Store,
    ) {
        use crate::bot_manager::message_adapter;

        match action.cast() {
            BotOutboundAction::MessageReceived {
                bot_token, message,
            } => {
                let bot_id = Self::bot_id_from_token(&bot_token);
                let msg = message_adapter::telegram_to_aitk_message(
                    &message, &bot_id,
                );
                let chat_id =
                    self.find_or_create_bot_chat(cx, store, &bot_id);

                if let Some(chat) = store.chats.get_chat_by_id(chat_id)
                {
                    let mut chat = chat.borrow_mut();
                    chat.messages.push(msg.clone());
                    chat.update_title_based_on_first_message();
                    chat.save_and_forget();
                }

                self.dispatch_to_controller(
                    chat_id,
                    VecMutation::Push(msg),
                );
                cx.redraw_all();
            }
            BotOutboundAction::MessageEdited {
                bot_token,
                message_id,
                new_text,
                reply_markup,
            } => {
                let bot_id = Self::bot_id_from_token(&bot_token);
                let Some(chat_id) =
                    store.chats.find_chat_for_bot(&bot_id)
                else {
                    return;
                };
                let Some(chat) = store.chats.get_chat_by_id(chat_id)
                else {
                    return;
                };

                let quick_replies = reply_markup.as_ref().map(|kb| {
                    message_adapter::inline_keyboard_to_quick_replies(
                        kb, message_id,
                    )
                }).unwrap_or_default();

                let mut chat = chat.borrow_mut();
                let msg_id_str = message_id.to_string();
                let Some((idx, _)) = chat.messages.iter().enumerate()
                    .find(|(_, m)| {
                        m.content.data.as_deref()
                            == Some(msg_id_str.as_str())
                    })
                else {
                    return;
                };

                // HACK(telegram-reveal): Capture old text before
                // updating Store, for streaming-append detection.
                let old_text =
                    chat.messages[idx].content.text.clone();

                // Always update Store with full text immediately.
                let mut updated = chat.messages[idx].clone();
                updated.content.text = new_text.clone();
                updated.content.quick_replies =
                    quick_replies.clone();
                chat.messages[idx] = updated.clone();
                chat.save_and_forget();

                // HACK(telegram-reveal): Determine if this is a
                // streaming append (text grows with same prefix).
                let reveal_key =
                    (chat_id, msg_id_str.clone());

                let anchor = self
                    .pending_reveals
                    .get(&reveal_key)
                    .map(|r| &r.target_text[..r.revealed_len])
                    .unwrap_or(&old_text);

                let is_streaming_append =
                    quick_replies.is_empty()
                        && new_text.starts_with(anchor)
                        && new_text.len() > anchor.len();

                if is_streaming_append {
                    let revealed_len = self
                        .pending_reveals
                        .get(&reveal_key)
                        .map(|r| r.revealed_len)
                        .unwrap_or(old_text.len());

                    self.pending_reveals.insert(
                        reveal_key,
                        PendingReveal {
                            chat_id,
                            target_text: new_text,
                            revealed_len,
                            pending_quick_replies: quick_replies,
                            base_message: updated,
                        },
                    );

                    if self.reveal_timer.is_empty() {
                        self.reveal_timer = cx.start_interval(
                            REVEAL_INTERVAL_SECS,
                        );
                    }
                } else {
                    // Non-append edit: flush immediately.
                    self.pending_reveals.remove(&reveal_key);
                    self.dispatch_to_controller(
                        chat_id,
                        VecMutation::Update(idx, updated),
                    );
                }
                cx.redraw_all();
            }
            BotOutboundAction::MessageDeleted {
                bot_token, message_id
            } => {
                let bot_id = Self::bot_id_from_token(&bot_token);
                let Some(chat_id) =
                    store.chats.find_chat_for_bot(&bot_id)
                else {
                    return;
                };
                let Some(chat) = store.chats.get_chat_by_id(chat_id)
                else {
                    return;
                };

                let mut chat = chat.borrow_mut();
                let msg_id_str = message_id.to_string();
                if let Some(idx) = chat.messages.iter().position(|m| {
                    m.content.data.as_deref()
                        == Some(msg_id_str.as_str())
                }) {
                    chat.messages.remove(idx);
                    chat.save_and_forget();

                    // HACK(telegram-reveal): Remove any pending
                    // reveal for the deleted message.
                    let del_key =
                        (chat_id, msg_id_str.clone());
                    self.pending_reveals.remove(&del_key);

                    self.dispatch_to_controller(
                        chat_id,
                        VecMutation::<Message>::RemoveOne(idx),
                    );
                    cx.redraw_all();
                }
            }
            BotOutboundAction::None => {}
        }
    }

    /// Dispatches a mutation to the ChatController for live UI update.
    fn dispatch_to_controller<M>(&mut self, chat_id: ChatId, mutation: M)
    where
        M: Into<ChatStateMutation>,
    {
        if let Some(view) = self.chat_view_refs.get_mut(&chat_id) {
            view.borrow()
                .unwrap()
                .chat_controller()
                .lock()
                .unwrap()
                .dispatch_mutation(mutation);
        }
    }

    fn bot_id_from_token(token: &str) -> BotId {
        BotId::new(format!("telegram_bot/{token}"))
    }

    fn find_or_create_bot_chat(
        &mut self,
        cx: &mut Cx,
        store: &mut Store,
        bot_id: &BotId,
    ) -> ChatId {
        if let Some(id) = store.chats.find_chat_for_bot(bot_id) {
            return id;
        }
        let chat_id =
            store.chats.create_empty_chat(Some(bot_id.clone()));
        if let Some(chat) = store.chats.get_chat_by_id(chat_id) {
            self.create_or_update_chat_view(cx, &chat.borrow());
        }
        chat_id
    }

    /// HACK(telegram-reveal): Advance all active reveal animations by one tick.
    ///
    /// Resolves message index by `content.data` (telegram message ID) each tick,
    /// so the reveal is immune to message insertions/removals between ticks.
    fn tick_reveals(&mut self, cx: &mut Cx) {
        let keys: Vec<_> = self.pending_reveals.keys().cloned().collect();
        let mut completed = Vec::new();

        for key in &keys {
            let Some(reveal) = self.pending_reveals.get_mut(key) else {
                continue;
            };

            let new_len = snap_to_char_boundary(
                &reveal.target_text,
                reveal.revealed_len + REVEAL_CHARS_PER_TICK,
            );
            reveal.revealed_len = new_len;

            let done = new_len >= reveal.target_text.len();

            let mut partial = reveal.base_message.clone();
            if done {
                partial.content.text = reveal.target_text.clone();
                partial.content.quick_replies =
                    reveal.pending_quick_replies.clone();
            } else {
                partial.content.text =
                    reveal.target_text[..new_len].to_string();
            }

            let chat_id = reveal.chat_id;
            let msg_id_str = &key.1;

            // Resolve index dynamically from controller state.
            let idx = self
                .chat_view_refs
                .get_mut(&chat_id)
                .and_then(|view| {
                    let inner = view.borrow()?;
                    let ctrl = inner.chat_controller();
                    let guard = ctrl.lock().ok()?;
                    guard.state().messages.iter().position(|m| {
                        m.content.data.as_deref()
                            == Some(msg_id_str.as_str())
                    })
                });

            if let Some(idx) = idx {
                self.dispatch_to_controller(
                    chat_id,
                    VecMutation::Update(idx, partial),
                );
            }

            if done || idx.is_none() {
                completed.push(key.clone());
            }
        }

        for key in completed {
            self.pending_reveals.remove(&key);
        }

        if self.pending_reveals.is_empty() {
            cx.stop_timer(self.reveal_timer);
            self.reveal_timer = Timer::empty();
        }

        cx.redraw_all();
    }
}

impl ChatsDeck {
    fn open_chat_for_bot(
        &mut self,
        cx: &mut Cx,
        store: &mut Store,
        bot_id: &BotId,
        reuse_existing: bool,
    ) {
        if reuse_existing
            && let Some(chat_id) = store.chats.find_chat_for_bot(bot_id)
        {
            store.chats.set_current_chat(Some(chat_id));

            if let Some(chat) = store.chats.get_chat_by_id(chat_id) {
                store
                    .preferences
                    .set_current_chat_model(chat.borrow().associated_bot.clone());
                self.create_or_update_chat_view(cx, &chat.borrow());
            }
            return;
        }

        let chat_id = store.chats.create_empty_chat(Some(bot_id.clone()));

        #[cfg(not(target_arch = "wasm32"))]
        if bot_id.as_str().ends_with("/botfather")
            && let Some(chat) = store.chats.get_chat_by_id(chat_id)
        {
            let content = if store.bot_server_state.is_some() {
                crate::bot_manager::BotFatherClient::welcome_message()
            } else {
                MessageContent {
                    text: "BotFather server is not running. \
                           Please restart the application."
                        .to_string(),
                    ..Default::default()
                }
            };
            chat.borrow_mut().messages.push(Message {
                from: EntityId::Bot(bot_id.clone()),
                content,
                ..Default::default()
            });
            chat.borrow().save_and_forget();
        }

        if let Some(chat) = store.chats.get_chat_by_id(chat_id) {
            self.create_or_update_chat_view(cx, &chat.borrow());
        }
    }

    pub fn create_or_update_chat_view(&mut self, cx: &mut Cx, chat_data: &ChatData) {
        // Check if an instance already exists for this chat
        if let Some(existing_view) = self.chat_view_refs.get_mut(&chat_data.id) {
            // Instance exists, just make it visible and focused
            self.currently_visible_chat_id = Some(chat_data.id);

            // Update focus states
            existing_view.set_focused(true);
            for (id, chat_view) in self.chat_view_refs.iter_mut() {
                if *id != chat_data.id {
                    chat_view.set_focused(false);
                }
            }

            // Update LRU access order
            self.chat_view_accessed_order
                .retain(|id| *id != chat_data.id);
            self.chat_view_accessed_order.push_back(chat_data.id);

            return; // EARLY RETURN, don't recreate!
        }

        // No existing instance, create a new one
        let chat_view = WidgetRef::new_from_ptr(cx, self.chat_view_template);
        let mut chat_view = chat_view.as_chat_view();

        // Initialize new instance
        chat_view.set_chat_id(chat_data.id);

        // Load messages into the controller
        chat_view
            .borrow()
            .unwrap()
            .chat_controller()
            .lock()
            .unwrap()
            .dispatch_mutation(VecMutation::Set(chat_data.messages.clone()));

        // Sync associated_bot from Store to ChatController
        if let Some(bot_id) = &chat_data.associated_bot {
            chat_view.set_bot_id(Some(bot_id.clone()));
        }

        // Set as focused
        chat_view.set_focused(true);

        // Insert into HashMap
        self.chat_view_refs.insert(chat_data.id, chat_view);
        self.currently_visible_chat_id = Some(chat_data.id);

        // Defocus other chats
        for (id, cv) in self.chat_view_refs.iter_mut() {
            if *id != chat_data.id {
                cv.set_focused(false);
            }
        }

        // Add to LRU tracking
        self.chat_view_accessed_order.push_back(chat_data.id);

        // Evict oldest instance if we exceed max
        if self.chat_view_accessed_order.len() > MAX_CHAT_VIEWS {
            let oldest_id = self.chat_view_accessed_order.pop_front().unwrap();
            if let Some(oldest_view) = self.chat_view_refs.get_mut(&oldest_id) {
                // Don't evict if currently streaming
                if !oldest_view.chat(ids!(chat)).read().is_streaming() {
                    self.chat_view_refs.remove(&oldest_id);
                } else {
                    // Put back in queue if streaming
                    self.chat_view_accessed_order.push_front(oldest_id);
                }
            }
        }

        // TODO: Focus on prompt input
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_botfather_seed_message_is_persisted() {
        let source = include_str!("chats_deck.rs");
        assert!(
            source.contains("chat.borrow().save_and_forget();"),
            "BotFather seed message should be saved after insertion",
        );
    }

    #[test]
    fn test_message_received_does_not_refresh_bot_cache() {
        let source = include_str!("chats_deck.rs");
        let start = source
            .find("BotOutboundAction::MessageReceived")
            .expect("MessageReceived branch should exist");
        let end = source[start..]
            .find("BotOutboundAction::MessageEdited")
            .map(|offset| start + offset)
            .expect("MessageEdited branch should follow MessageReceived");
        let message_received_block = &source[start..end];

        assert!(
            !message_received_block.contains("refresh_bot_name_cache"),
            "MessageReceived should not rebuild the bot cache",
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn test_snap_to_char_boundary() {
        use super::snap_to_char_boundary;

        // ASCII: every byte is a char boundary.
        assert_eq!(snap_to_char_boundary("hello", 3), 3);
        assert_eq!(snap_to_char_boundary("hello", 0), 0);
        assert_eq!(snap_to_char_boundary("hello", 10), 5);

        // Multi-byte: '你' is 3 bytes (E4 BD A0).
        let s = "a你b"; // bytes: [97, E4, BD, A0, 98]
        assert_eq!(snap_to_char_boundary(s, 1), 1); // before '你'
        assert_eq!(snap_to_char_boundary(s, 2), 1); // mid '你' → snaps back
        assert_eq!(snap_to_char_boundary(s, 3), 1); // mid '你' → snaps back
        assert_eq!(snap_to_char_boundary(s, 4), 4); // at 'b'

        // Empty string.
        assert_eq!(snap_to_char_boundary("", 0), 0);
        assert_eq!(snap_to_char_boundary("", 5), 0);
    }

    #[test]
    fn test_reveal_hack_is_documented() {
        let source = include_str!("chats_deck.rs");
        assert!(
            source.contains("HACK(telegram-reveal)"),
            "Reveal hack code must include HACK(telegram-reveal) tag",
        );
    }
}
