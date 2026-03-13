use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::widgets::*;

    use crate::shared::styles::*;
    use crate::shared::widgets::*;

    BOTFATHER_GREEN = #4CAF50
    BOTFATHER_CARD_BG = #f5f5f5

    StatCard = <RoundedView> {
        width: Fill, height: Fit
        padding: 16
        draw_bg: {
            color: (BOTFATHER_CARD_BG)
            border_radius: 10.0
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
            border_radius: 10.0
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
                    border_radius: 4.0
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

        // Server Configuration
        config_section = <View> {
            width: Fill, height: Fit
            flow: Down
            margin: {top: 16}

            config_title = <Label> {
                draw_text: {
                    text_style: <BOLD_FONT>{font_size: 12}
                    color: #222222
                }
                text: "Server Configuration"
            }

            <View> {
                width: Fill, height: Fit
                flow: Right
                align: {y: 0.5}
                spacing: 8
                margin: {top: 8}

                <Label> {
                    width: 100
                    draw_text: {
                        text_style: {font_size: 11}
                        color: #555555
                    }
                    text: "Server Port"
                }

                port_input = <MolyTextInput> {
                    width: Fill, height: 30
                    draw_text: {
                        text_style: {font_size: 11}
                        color: #333333
                    }
                    text: "8488"
                }
            }

            port_hint = <Label> {
                margin: {top: 4}
                draw_text: {
                    text_style: {font_size: 9}
                    color: #999999
                }
                text: "Server will be restarted with the new port"
            }

            save_port_button = <MolyButton> {
                margin: {top: 8}
                width: Fit
                height: 30
                padding: {left: 20, right: 20, top: 0, bottom: 0}
                text: "Save"
                draw_bg: {
                    color: #4a90d9
                    border_size: 0
                }
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

        // Command reference (text set at runtime from COMMAND_LIST)
        commands_panel = <InfoPanel> {
            panel_title = { text: "Available Commands" }
            commands_content = <Label> {
                margin: {top: 8}
                width: Fill
                draw_text: {
                    text_style: {font_size: 10}
                    color: #555555
                    wrap: Word
                }
            }
        }

        // Footer
        <Label> {
            margin: {top: 16}
            draw_text: {
                text_style: {font_size: 9}
                color: #999999
            }
            text: "BotFather runs locally — created bots are accessible from the chat model selector."
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
        self.deref.handle_event(cx, event, scope);
        self.widget_match_event(cx, event, scope);
    }
}

impl BotFatherView {
    /// Update the displayed bot count, server port, and command list.
    pub fn set_data(
        &mut self,
        cx: &mut Cx,
        bot_count: usize,
        server_port: u16,
        command_list: &str,
    ) {
        self.label(ids!(bot_count_value))
            .set_text(cx, &bot_count.to_string());
        self.label(ids!(api_address_value))
            .set_text(cx, &format!("localhost:{server_port}"));
        self.text_input(ids!(port_input))
            .set_text(cx, &server_port.to_string());
        self.label(ids!(commands_content))
            .set_text(cx, command_list);
    }
}

impl WidgetMatchEvent for BotFatherView {
    fn handle_actions(
        &mut self,
        cx: &mut Cx,
        actions: &Actions,
        _scope: &mut Scope,
    ) {
        if self.button(ids!(save_port_button)).clicked(actions) {
            let port_text = self.text_input(ids!(port_input)).text();
            if let Ok(port) = port_text.parse::<u16>() {
                if port > 0 {
                    cx.action(BotFatherAction::SavePort(port));
                }
            }
        }
    }
}

impl BotFatherViewRef {
    /// See [`BotFatherView::set_data`].
    pub fn set_data(
        &mut self,
        cx: &mut Cx,
        bot_count: usize,
        server_port: u16,
        command_list: &str,
    ) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_data(cx, bot_count, server_port, command_list);
        }
    }
}

/// Actions emitted by the BotFather settings panel.
#[derive(Clone, Debug, DefaultNone)]
pub enum BotFatherAction {
    /// User saved a new server port value.
    SavePort(u16),
    None,
}
