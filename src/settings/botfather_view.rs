use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::widgets::*;

    use crate::shared::styles::*;

    BOTFATHER_GREEN = #4CAF50
    BOTFATHER_CARD_BG = #f5f5f5
    BOTFATHER_CARD_BORDER = #e0e0e0

    StatCard = <RoundedView> {
        width: Fill, height: Fit
        padding: 16
        draw_bg: {
            color: (BOTFATHER_CARD_BG)
            border_width: 1.0
            border_color: (BOTFATHER_CARD_BORDER)
            radius: 10.0
        }
        flow: Down
        align: {x: 0.5}
        stat_value = <Label> {
            draw_text: {
                text_style: <BOLD_FONT>{font_size: 18}
                color: (BOTFATHER_GREEN)
            }
        }
        stat_label = <Label> {
            margin: {top: 4}
            draw_text: {
                text_style: {font_size: 9}
                color: #888888
            }
        }
    }

    InfoPanel = <RoundedView> {
        width: Fill, height: Fit
        padding: 16
        margin: {top: 12}
        draw_bg: {
            color: (BOTFATHER_CARD_BG)
            border_width: 1.0
            border_color: (BOTFATHER_CARD_BORDER)
            radius: 10.0
        }
        flow: Down
        panel_title = <Label> {
            draw_text: {
                text_style: <BOLD_FONT>{font_size: 12}
                color: #222222
            }
        }
        panel_content = <Label> {
            margin: {top: 8}
            width: Fill
            draw_text: {
                text_style: {font_size: 10}
                color: #555555
                wrap: Word
            }
        }
    }

    pub BotFatherView = {{BotFatherView}} {
        width: Fill, height: Fit
        flow: Down
        padding: {top: 16, left: 0, right: 0}
        visible: false

        // Status indicator
        status_row = <View> {
            width: Fit, height: Fit
            flow: Right
            align: {y: 0.5}
            spacing: 6
            status_dot = <RoundedView> {
                width: 8, height: 8
                draw_bg: {
                    color: (BOTFATHER_GREEN)
                    radius: 4.0
                }
            }
            status_label = <Label> {
                draw_text: {
                    text_style: {font_size: 10}
                    color: (BOTFATHER_GREEN)
                }
                text: "Local service running"
            }
        }

        // Stats row
        stats_row = <View> {
            width: Fill, height: Fit
            flow: Right
            spacing: 10
            margin: {top: 16}
            <StatCard> {
                bot_count_value = <Label> {
                    draw_text: {
                        text_style: <BOLD_FONT>{font_size: 18}
                        color: (BOTFATHER_GREEN)
                    }
                    text: "0"
                }
                stat_label = { text: "Bots Created" }
            }
            <StatCard> {
                api_address_value = <Label> {
                    draw_text: {
                        text_style: <REGULAR_FONT>{font_size: 11}
                        color: #666666
                    }
                    text: "localhost:8488"
                }
                stat_label = { text: "API Address" }
            }
        }

        // Quick start guide
        guide_panel = <InfoPanel> {
            panel_title = { text: "Quick Start" }
            panel_content = {
                text: "1. Go to Chat → Select BotFather model\n\
                       2. Type /newbot to create your first bot\n\
                       3. Get the token → Configure in crew-rs"
            }
        }

        // Command reference
        commands_panel = <InfoPanel> {
            panel_title = { text: "Available Commands" }
            panel_content = {
                text: "/newbot  — Create a new bot\n\
                       /mybots  — Manage your bots\n\
                       /start   — Show welcome message\n\
                       /help    — Show help\n\
                       /cancel  — Cancel current operation"
            }
        }

        // Footer
        <Label> {
            margin: {top: 16}
            draw_text: {
                text_style: {font_size: 9}
                color: #999999
            }
            text: "No configuration needed — BotFather runs locally"
        }
    }
}

#[derive(Live, Widget, LiveHook)]
pub struct BotFatherView {
    #[deref]
    deref: View,
}

impl Widget for BotFatherView {
    fn draw_walk(
        &mut self,
        cx: &mut Cx2d,
        scope: &mut Scope,
        walk: Walk,
    ) -> DrawStep {
        self.deref.draw_walk(cx, scope, walk)
    }

    fn handle_event(
        &mut self,
        cx: &mut Cx,
        event: &Event,
        scope: &mut Scope,
    ) {
        self.deref.handle_event(cx, event, scope)
    }
}

impl BotFatherView {
    /// Update the displayed bot count and server port.
    pub fn set_data(
        &mut self,
        cx: &mut Cx,
        bot_count: usize,
        server_port: u16,
    ) {
        self.label(ids!(bot_count_value))
            .set_text(cx, &bot_count.to_string());
        self.label(ids!(api_address_value))
            .set_text(cx, &format!("localhost:{server_port}"));
    }
}

impl BotFatherViewRef {
    /// See [`BotFatherView::set_data`].
    pub fn set_data(
        &mut self,
        cx: &mut Cx,
        bot_count: usize,
        server_port: u16,
    ) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_data(cx, bot_count, server_port);
        }
    }
}
