use super::chat_history_card::ChatHistoryCardWidgetRefExt;
use crate::chat::entity_button::EntityButtonWidgetRefExt;
use crate::data::chats::chat::ChatId;
use crate::data::store::Store;
use crate::shared::actions::ChatAction;
use makepad_widgets::*;
#[cfg(target_arch = "wasm32")]
use moly_kit::prelude::BotId;

live_design! {
    use link::theme::*;
    use link::shaders::*;
    use link::widgets::*;

    use crate::shared::styles::*;
    use crate::shared::widgets::*;
    use crate::chat::chat_history_card::ChatHistoryCard;
    use crate::chat::entity_button::*;

    HeadingLabel = <Label> {
        margin: {left: 4, bottom: 4},
        draw_text:{
            text_style: <BOLD_FONT>{font_size: 10.5},
            color: #3
        }
    }

    NoAgentsWarning = <Label> {
        margin: {left: 4, bottom: 4},
        width: Fill
        draw_text:{
            text_style: {font_size: 8.5},
            color: #3
        }
    }

    pub ChatHistory = {{ChatHistory}} {
        width: Fill, height: Fill
        show_bg: true
        draw_bg: {
            color: (MAIN_BG_COLOR)
        }
        padding: { left: 10, right: 10 }

        list = <PortalList> {
            drag_scrolling: false,
            BotsHeading = <HeadingLabel> { text: "BOTS", margin: {top: 10}, }
            BotButton = <EntityButton> {
                server_url_visible: false,
            }
            ChatsHeading = <HeadingLabel> { text: "CHATS", margin: {top: 10}, }
            ChatHistoryCard = <ChatHistoryCard> {
                cursor: Default
            }
        }
    }
}

#[derive(Live, LiveHook, Widget)]
pub struct ChatHistory {
    #[deref]
    deref: View,
}

impl Widget for ChatHistory {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.deref.handle_event(cx, event, scope);
        self.widget_match_event(cx, event, scope);
    }

    fn draw_walk(
        &mut self,
        cx: &mut Cx2d,
        scope: &mut Scope,
        walk: Walk,
    ) -> DrawStep {
        let store = scope.data.get_mut::<Store>().unwrap();

        // Use cached bot entries to avoid SQLite queries in draw path
        #[cfg(not(target_arch = "wasm32"))]
        let bot_entries = store.bot_sidebar_cache.clone();

        #[cfg(target_arch = "wasm32")]
        let bot_entries: Vec<(BotId, String)> = Vec::new();

        enum Item<'a> {
            BotsHeader,
            BotButton(usize),
            ChatsHeader,
            ChatButton(&'a ChatId),
        }

        let mut items: Vec<Item> = Vec::new();

        if !bot_entries.is_empty() {
            items.push(Item::BotsHeader);
            for i in 0..bot_entries.len() {
                items.push(Item::BotButton(i));
            }
        }

        items.push(Item::ChatsHeader);

        let mut chat_ids = store
            .chats
            .saved_chats
            .iter()
            .map(|c| c.borrow().id)
            .collect::<Vec<_>>();

        // Reverse sort chat ids (most recent first).
        chat_ids.sort_by(|a, b| b.cmp(a));

        items.extend(chat_ids.iter().map(Item::ChatButton));

        while let Some(view_item) =
            self.deref.draw_walk(cx, scope, walk).step()
        {
            if let Some(mut list) =
                view_item.as_portal_list().borrow_mut()
            {
                list.set_item_range(cx, 0, items.len() - 1);
                while let Some(item_id) = list.next_visible_item(cx) {
                    if item_id >= items.len() {
                        continue;
                    }

                    match &items[item_id] {
                        Item::BotsHeader => {
                            let item = list.item(
                                cx,
                                item_id,
                                live_id!(BotsHeading),
                            );
                            item.draw_all(cx, scope);
                        }
                        Item::BotButton(idx) => {
                            let (bot_id, _name) = &bot_entries[*idx];
                            let item = list.item(
                                cx,
                                item_id,
                                live_id!(BotButton),
                            );
                            item.as_entity_button()
                                .set_bot_id(cx, bot_id);
                            item.draw_all(cx, scope);
                        }
                        Item::ChatsHeader => {
                            let item = list.item(
                                cx,
                                item_id,
                                live_id!(ChatsHeading),
                            );
                            item.draw_all(cx, scope);
                        }
                        Item::ChatButton(chat_id) => {
                            let mut item = list
                                .item(
                                    cx,
                                    item_id,
                                    live_id!(ChatHistoryCard),
                                )
                                .as_chat_history_card();
                            let _ = item.set_chat_id(**chat_id);
                            item.draw_all(cx, scope);
                        }
                    }
                }
            }
        }

        DrawStep::done()
    }
}

impl WidgetMatchEvent for ChatHistory {
    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions, _scope: &mut Scope) {
        let clicked_entity_button = self
            .portal_list(ids!(list))
            .items_with_actions(actions)
            .iter()
            .map(|(_, item)| item.as_entity_button())
            .find(|eb| eb.clicked(actions));

        if let Some(entity_button) = clicked_entity_button {
            let bot_id = entity_button.get_bot_id();
            if let Some(bot_id) = bot_id {
                cx.action(ChatAction::StartOrSelect(bot_id));
            }
        }
    }
}
