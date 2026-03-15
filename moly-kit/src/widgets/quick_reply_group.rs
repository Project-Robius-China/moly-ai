use crate::aitk::protocol::{ButtonStyle, QuickReplyButton};
use makepad_widgets::*;

live_design! {
    use link::theme::*;
    use link::widgets::*;
    use link::moly_kit_theme::*;

    QUICK_REPLY_PRIMARY_COLOR = #4CAF50
    QUICK_REPLY_SECONDARY_COLOR = #2196F3
    QUICK_REPLY_SUBTLE_COLOR = #888888
    QuickReplyBtn = <Button> {
        width: Fit, height: Fit
        padding: {top: 7, bottom: 7, left: 14, right: 14}
        margin: {right: 6, bottom: 6}
        draw_bg: {
            instance color: #EEF7F0
            instance color_hover: #E2F2E6
            instance color_down: #D4EBCB
            instance border_color: #CFE4D3
            instance border_color_hover: #B9D8C0
            instance border_color_down: #A3CAA9
            instance border_size: 1.0
            instance border_radius: 11.0

            fn pixel(self) -> vec4 {
                let sdf = Sdf2d::viewport(self.pos * self.rect_size);
                let body = mix(
                    mix(self.color, self.color_hover, self.hover),
                    self.color_down,
                    self.down,
                );
                let border = mix(
                    mix(self.border_color, self.border_color_hover, self.hover),
                    self.border_color_down,
                    self.down,
                );

                sdf.box(
                    self.border_size,
                    self.border_size,
                    self.rect_size.x - (self.border_size * 2.0),
                    self.rect_size.y - (self.border_size * 2.0),
                    self.border_radius
                );
                sdf.fill_keep(body);
                sdf.stroke(border, self.border_size);
                return sdf.result;
            }
        }
        draw_text: {
            text_style: <THEME_FONT_BOLD>{ font_size: 9.5 }
            color: #2D6A3D
            color_hover: #245631
            color_down: #1C4528
            fn get_color(self) -> vec4 {
                return mix(
                    mix(self.color, self.color_hover, self.hover),
                    self.color_down,
                    self.down,
                );
            }
        }
        text: ""
    }

    pub QuickReplyGroup = {{QuickReplyGroup}} {
        width: Fill, height: Fit
        flow: Right
        spacing: 0.0
        padding: {top: 6}
        visible: false
        button_template = <QuickReplyBtn> {}
    }
}

/// Action dispatched when a quick reply button is clicked.
#[derive(Debug, Clone, DefaultNone)]
pub enum QuickReplyAction {
    /// A quick reply button was clicked. Contains the action text to inject.
    Clicked(String),
    None,
}

#[derive(Live, Widget, LiveHook)]
pub struct QuickReplyGroup {
    #[redraw]
    #[rust]
    area: Area,

    #[walk]
    walk: Walk,

    #[layout]
    layout: Layout,

    #[live]
    visible: bool,

    #[live]
    button_template: Option<LivePtr>,

    #[rust]
    buttons: Vec<QuickReplyButton>,

    #[rust]
    disabled: bool,

    #[rust]
    items: ComponentMap<LiveId, WidgetRef>,
}

impl Widget for QuickReplyGroup {
    fn draw_walk(
        &mut self,
        cx: &mut Cx2d,
        _scope: &mut Scope,
        walk: Walk,
    ) -> DrawStep {
        if !self.visible {
            return DrawStep::done();
        }

        cx.begin_turtle(walk, self.layout);
        self.draw_buttons(cx);
        cx.end_turtle_with_area(&mut self.area);
        DrawStep::done()
    }

    fn handle_event(
        &mut self,
        cx: &mut Cx,
        event: &Event,
        scope: &mut Scope,
    ) {
        let actions = cx.capture_actions(|cx| {
            for (_, item) in self.items.iter_mut() {
                item.handle_event(cx, event, scope);
            }
        });

        if self.disabled || self.buttons.is_empty() {
            return;
        }

        let clicked = self.buttons.iter().enumerate().find_map(|(index, btn)| {
            self.items
                .get(&LiveId(index as u64))
                .is_some_and(|item| item.as_button().clicked(&actions))
                .then(|| btn.action.clone())
        });

        if let Some(action_text) = clicked {
            self.disabled = true;
            self.redraw(cx);
            cx.widget_action(
                self.widget_uid(),
                &scope.path,
                QuickReplyAction::Clicked(action_text),
            );
        }
    }
}

impl QuickReplyGroup {
    /// Set the quick reply buttons to display.
    pub fn set_buttons(
        &mut self,
        cx: &mut Cx,
        buttons: &[QuickReplyButton],
    ) {
        self.buttons = buttons.to_vec();
        self.disabled = false;
        self.set_visible(cx, !buttons.is_empty());
        self.sync_items(cx);

        if buttons.is_empty() {
            return;
        }

        self.redraw(cx);
    }

    fn sync_items(&mut self, cx: &mut Cx) {
        self.items.clear();
        for index in 0..self.buttons.len() {
            let item_id = LiveId(index as u64);
            let item = WidgetRef::new_from_ptr(cx, self.button_template);
            self.items.insert(item_id, item);
        }
    }

    fn draw_buttons(&mut self, cx: &mut Cx2d) {
        for (index, button) in self.buttons.iter().enumerate() {
            let Some(item) = self.items.get(&LiveId(index as u64)) else {
                continue;
            };

            let colors = if self.disabled {
                disabled_colors()
            } else {
                style_colors(button.style)
            };

            item.apply_over(
                cx,
                live! {
                    draw_bg: {
                        color: (colors.fill)
                        color_hover: (colors.fill)
                        color_down: (colors.fill)
                        border_color: (colors.border)
                        border_color_hover: (colors.border)
                        border_color_down: (colors.border)
                    }
                    draw_text: {
                        color: (colors.text)
                        color_hover: (colors.text)
                        color_down: (colors.text)
                    }
                },
            );

            if self.disabled {
                item.apply_over(cx, live! { cursor: Default });
            } else {
                item.apply_over(cx, live! { cursor: Hand });
            }

            item.as_button().set_text(cx, &button.label);
            item.draw_all(cx, &mut Scope::empty());
        }
    }
}

#[derive(Clone, Copy)]
struct QuickReplyColors {
    fill: Vec4,
    border: Vec4,
    text: Vec4,
}

/// Returns visual colors for a button style.
fn style_colors(style: ButtonStyle) -> QuickReplyColors {
    match style {
        ButtonStyle::Primary => QuickReplyColors {
            fill: vec4(0.933, 0.969, 0.941, 1.0),
            border: vec4(0.776, 0.882, 0.804, 1.0),
            text: vec4(0.176, 0.416, 0.239, 1.0),
        },
        ButtonStyle::Secondary => QuickReplyColors {
            fill: vec4(0.922, 0.953, 0.992, 1.0),
            border: vec4(0.749, 0.847, 0.976, 1.0),
            text: vec4(0.102, 0.345, 0.612, 1.0),
        },
        ButtonStyle::Subtle => QuickReplyColors {
            fill: vec4(0.965, 0.965, 0.969, 1.0),
            border: vec4(0.835, 0.843, 0.863, 1.0),
            text: vec4(0.365, 0.392, 0.443, 1.0),
        },
    }
}

fn disabled_colors() -> QuickReplyColors {
    QuickReplyColors {
        fill: vec4(0.949, 0.953, 0.961, 1.0),
        border: vec4(0.851, 0.863, 0.886, 1.0),
        text: vec4(0.612, 0.643, 0.698, 1.0),
    }
}

impl QuickReplyGroupRef {
    /// See [`QuickReplyGroup::set_buttons`].
    pub fn set_buttons(
        &mut self,
        cx: &mut Cx,
        buttons: &[QuickReplyButton],
    ) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_buttons(cx, buttons);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_widget_actions() {
        let source = include_str!("quick_reply_group.rs");
        let implementation = source.split("#[cfg(test)]").next().unwrap_or(source);
        assert!(implementation.contains("cx.widget_action("));
        assert!(!implementation.contains("cx.action(QuickReplyAction::Clicked"));
    }

    #[test]
    fn test_no_portal_list() {
        let source = include_str!("quick_reply_group.rs");
        let implementation = source.split("#[cfg(test)]").next().unwrap_or(source);
        assert!(!implementation.contains("<PortalList>"));
        assert!(implementation.contains("button_template = <QuickReplyBtn> {}"));
    }

    #[test]
    fn test_button_shader() {
        let source = include_str!("quick_reply_group.rs");
        let implementation = source.split("#[cfg(test)]").next().unwrap_or(source);
        assert!(implementation.contains("fn pixel(self) -> vec4"));
    }
}
