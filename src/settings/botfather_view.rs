use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::widgets::*;

    use crate::shared::styles::*;
    use crate::shared::widgets::*;

    BOTFATHER_GREEN = #4CAF50
    BOTFATHER_CARD_BG = #FCFCFC
    BOTFATHER_STAT_CARD_BG = #FFFFFF

    SectionPanel = <RoundedShadowView> {
        width: Fill, height: Fit
        padding: 16
        margin: {top: 12}
        draw_bg: {
            color: (BOTFATHER_CARD_BG)
            border_radius: 4.5,
            uniform shadow_color: #0002
            shadow_radius: 8.0,
            shadow_offset: vec2(0.0,-1.5)
        }
        flow: Down

        panel_title = <Label> {
            margin: {bottom: 4}
            draw_text: {
                text_style: <BOLD_FONT>{font_size: 12}
                color: #222222
            }
        }
    }

    StatCard = <RoundedShadowView> {
        width: Fill, height: 92
        padding: {top: 14, right: 14, bottom: 14, left: 14}
        draw_bg: {
            color: (BOTFATHER_STAT_CARD_BG)
            border_radius: 4.5,
            uniform shadow_color: #0002
            shadow_radius: 8.0,
            shadow_offset: vec2(0.0,-1.5)
        }
        flow: Down
        spacing: 4
        align: {x: 0.5, y: 0.5}
        stat_label = <Label> {
            draw_text: {
                text_style: {font_size: 9}
                color: #888888
            }
        }
        stat_value = <Label> {
            draw_text: {
                text_style: <BOLD_FONT>{font_size: 18}
                color: (BOTFATHER_GREEN)
            }
        }
    }

    InfoPanel = <SectionPanel> {
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

    GuideStepRow = <View> {
        width: Fill, height: Fit
        flow: Right
        spacing: 8
        align: {y: 0.0}

        step_index = <Label> {
            width: 18, height: Fit
            draw_text: {
                text_style: <BOLD_FONT>{font_size: 10}
                color: #555555
            }
        }

        step_text = <Label> {
            width: Fill, height: Fit
            draw_text: {
                text_style: {font_size: 10}
                color: #555555
                wrap: Word
            }
        }
    }

    CommandRow = <View> {
        visible: false
        width: Fill, height: Fit
        flow: Right
        spacing: 10
        align: {y: 0.0}

        command_name = <Label> {
            width: 88, height: Fit
            draw_text: {
                text_style: <BOLD_FONT>{font_size: 9.5}
                color: #444444
            }
        }

        command_description = <Label> {
            width: Fill, height: Fit
            draw_text: {
                text_style: {
                    font_size: 9.5
                    line_spacing: 1.25
                }
                color: #555555
                wrap: Word
            }
        }
    }

    pub BotFatherView = {{BotFatherView}} {
        width: Fill, height: Fit
        flow: Down
        padding: {top: 16, left: 0, right: 0}

        // Status indicator
        status_row = <View> {
            width: Fit, height: Fit
            flow: Right
            align: {y: 0.5}
            spacing: 6
            margin: {left: 2}
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
            bot_count_card = <StatCard> {
                stat_value = { text: "0" }
                stat_label = { text: "Bots Created" }
            }
            api_address_card = <StatCard> {
                stat_value = <Label> {
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
        config_section = <SectionPanel> {
            panel_title = { text: "Server Configuration" }

            port_row = <View> {
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

                save_port_button = <MolyButton> {
                    width: Fit
                    height: 30
                    padding: {left: 18, right: 18, top: 0, bottom: 0}
                    text: "Save"
                    draw_bg: {
                        color: #4a90d9
                        border_size: 0
                    }
                }
            }

            port_hint = <Label> {
                margin: {top: 6}
                width: Fill
                draw_text: {
                    text_style: {font_size: 9}
                    color: #999999
                    wrap: Word
                }
                text: "Server will be restarted with the new port"
            }

            port_status_wrap = <View> {
                visible: false
                width: Fill
                flow: Down
                margin: {top: 4}

                port_status = <Label> {
                    width: Fill
                    draw_text: {
                        text_style: {font_size: 9}
                        color: #999999
                        wrap: Word
                    }
                    text: ""
                }
            }

        }

        // Quick start guide
        guide_panel = <SectionPanel> {
            panel_title = { text: "Quick Start" }
            steps = <View> {
                width: Fill, height: Fit
                flow: Down
                spacing: 6
                margin: {top: 8}

                <GuideStepRow> {
                    step_index = { text: "1." }
                    step_text = { text: "Go to Chat and select the BotFather model" }
                }
                <GuideStepRow> {
                    step_index = { text: "2." }
                    step_text = { text: "Type /newbot to create your first bot" }
                }
                <GuideStepRow> {
                    step_index = { text: "3." }
                    step_text = { text: "Copy the token and configure it in crew-rs" }
                }
            }
        }

        // Command reference (text set at runtime from COMMAND_LIST)
        commands_panel = <SectionPanel> {
            panel_title = { text: "Available Commands" }
            commands_list = <View> {
                width: Fill, height: Fit
                flow: Down
                spacing: 6
                margin: {top: 8}

                command_row_1 = <CommandRow> {}
                command_row_2 = <CommandRow> {}
                command_row_3 = <CommandRow> {}
                command_row_4 = <CommandRow> {}
                command_row_5 = <CommandRow> {}
                command_row_6 = <CommandRow> {}
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
    #[rust]
    port_input_dirty: bool,
    #[rust]
    last_synced_port: Option<u16>,
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
        self.label(ids!(bot_count_card.stat_value))
            .set_text(cx, &bot_count.to_string());
        self.label(ids!(api_address_card.stat_value))
            .set_text(cx, &format!("localhost:{server_port}"));
        let port_input = self.text_input(ids!(port_input));
        if should_sync_port_input(
            self.port_input_dirty,
            self.last_synced_port,
            &port_input.text(),
            server_port,
        ) {
            port_input.set_text(cx, &server_port.to_string());
            self.last_synced_port = Some(server_port);
        } else if port_input.text() == server_port.to_string() {
            self.last_synced_port = Some(server_port);
        }
        self.set_command_rows(cx, parse_command_list(command_list));
    }

    fn set_command_rows(
        &mut self,
        cx: &mut Cx,
        commands: Vec<(&str, &str)>,
    ) {
        self.set_command_row(
            cx,
            ids!(commands_panel.commands_list.command_row_1),
            commands.first().copied(),
        );
        self.set_command_row(
            cx,
            ids!(commands_panel.commands_list.command_row_2),
            commands.get(1).copied(),
        );
        self.set_command_row(
            cx,
            ids!(commands_panel.commands_list.command_row_3),
            commands.get(2).copied(),
        );
        self.set_command_row(
            cx,
            ids!(commands_panel.commands_list.command_row_4),
            commands.get(3).copied(),
        );
        self.set_command_row(
            cx,
            ids!(commands_panel.commands_list.command_row_5),
            commands.get(4).copied(),
        );
        self.set_command_row(
            cx,
            ids!(commands_panel.commands_list.command_row_6),
            commands.get(5).copied(),
        );
    }

    fn set_command_row(
        &mut self,
        cx: &mut Cx,
        row: &[LiveId],
        command: Option<(&str, &str)>,
    ) {
        let row_view = self.view(row);
        if let Some((name, description)) = command {
            row_view.set_visible(cx, true);
            row_view.label(ids!(command_name)).set_text(cx, name);
            row_view
                .label(ids!(command_description))
                .set_text(cx, description);
        } else {
            row_view.set_visible(cx, false);
        }
    }

    fn set_port_status(
        &mut self,
        cx: &mut Cx,
        message: &str,
        color: Vec4,
    ) {
        self.view(ids!(port_status_wrap)).set_visible(cx, true);
        let status = self.label(ids!(port_status));
        status.set_text(cx, message);
        status.apply_over(
            cx,
            live! {
                draw_text: { color: (color) }
            },
        );
    }

    fn clear_port_status(&mut self, cx: &mut Cx) {
        self.view(ids!(port_status_wrap)).set_visible(cx, false);
        let status = self.label(ids!(port_status));
        status.set_text(cx, "");
    }
}

fn parse_command_list(command_list: &str) -> Vec<(&str, &str)> {
    command_list
        .lines()
        .filter_map(|line| {
            let line = line.trim().strip_prefix("- ")?;
            let (name, description) = line.split_once(" — ")?;
            Some((name.trim_matches('`'), description.trim()))
        })
        .collect()
}

impl WidgetMatchEvent for BotFatherView {
    fn handle_actions(
        &mut self,
        cx: &mut Cx,
        actions: &Actions,
        _scope: &mut Scope,
    ) {
        let port_input = self.text_input(ids!(port_input));
        if port_input.changed(actions).is_some() {
            self.port_input_dirty = true;
            self.clear_port_status(cx);
        }

        if self.button(ids!(save_port_button)).clicked(actions) {
            let port_text = port_input.text();
            if let Ok(port) = port_text.parse::<u16>()
                && port > 0
            {
                self.port_input_dirty = false;
                port_input.set_text(cx, &port.to_string());
                cx.action(BotFatherAction::SavePort(port));
            } else {
                self.set_port_status(
                    cx,
                    "Enter a valid port between 1 and 65535.",
                    vec4(0.77, 0.16, 0.2, 1.0),
                );
            }
        }
    }
}

fn should_sync_port_input(
    port_input_dirty: bool,
    last_synced_port: Option<u16>,
    current_text: &str,
    server_port: u16,
) -> bool {
    !port_input_dirty
        && current_text != server_port.to_string()
        && last_synced_port
            .is_none_or(|port| current_text == port.to_string())
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

    pub fn set_port_status(
        &mut self,
        cx: &mut Cx,
        message: &str,
        color: Vec4,
    ) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_port_status(cx, message, color);
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

#[cfg(test)]
mod tests {
    use super::should_sync_port_input;

    #[test]
    fn clean_port_input_tracks_server_port() {
        assert!(should_sync_port_input(
            false,
            Some(8488),
            "8488",
            9494,
        ));
    }

    #[test]
    fn dirty_port_input_keeps_user_edit() {
        assert!(!should_sync_port_input(
            true,
            Some(8488),
            "9494",
            8488,
        ));
    }

    #[test]
    fn pending_saved_port_is_not_replaced_by_stale_port() {
        assert!(!should_sync_port_input(
            false,
            Some(8488),
            "9494",
            8488,
        ));
    }
}
