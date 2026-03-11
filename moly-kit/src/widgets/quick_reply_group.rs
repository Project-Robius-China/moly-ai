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
        padding: {top: 6, bottom: 6, left: 14, right: 14}
        margin: {right: 6, bottom: 6}
        draw_bg: {
            radius: 15.0
            border_width: 1.0
            border_color: (QUICK_REPLY_PRIMARY_COLOR)
            color: #00000000
            color_hover: #ffffff10
            color_pressed: #ffffff20
        }
        draw_text: {
            text_style: { font_size: 9.0 }
            color: (QUICK_REPLY_PRIMARY_COLOR)
            fn get_color(self) -> vec4 {
                return self.color;
            }
        }
        text: ""
    }

    pub QuickReplyGroup = {{QuickReplyGroup}} {
        width: Fill, height: Fit
        flow: Right
        padding: {top: 6}
        visible: false

        list = <PortalList> {
            flow: Right,
            width: Fill,
            height: Fit,
            QuickReplyBtn = <QuickReplyBtn> {}
        }
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
    #[deref]
    deref: View,

    #[rust]
    buttons: Vec<QuickReplyButton>,

    #[rust]
    disabled: bool,
}

impl Widget for QuickReplyGroup {
    fn draw_walk(
        &mut self,
        cx: &mut Cx2d,
        scope: &mut Scope,
        walk: Walk,
    ) -> DrawStep {
        let list_uid = self.portal_list(ids!(list)).widget_uid();
        while let Some(widget) =
            self.deref.draw_walk(cx, scope, walk).step()
        {
            if widget.widget_uid() == list_uid {
                self.draw_buttons(
                    cx,
                    &mut widget
                        .as_portal_list()
                        .borrow_mut()
                        .unwrap(),
                );
            }
        }

        DrawStep::done()
    }

    fn handle_event(
        &mut self,
        cx: &mut Cx,
        event: &Event,
        scope: &mut Scope,
    ) {
        let actions = cx.capture_actions(|cx| {
            self.deref.handle_event(cx, event, scope);
        });

        if self.disabled || self.buttons.is_empty() {
            return;
        }

        let clicked = {
            let list = self.portal_list(ids!(list));
            self.buttons.iter().enumerate().find_map(|(index, btn)| {
                list.get_item(index).and_then(|(_, item)| {
                    item.as_button()
                        .clicked(&actions)
                        .then(|| btn.action.clone())
                })
            })
        };

        if let Some(action_text) = clicked {
            self.disabled = true;
            self.redraw(cx);
            cx.action(QuickReplyAction::Clicked(action_text));
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
        self.set_visible(cx, !buttons.is_empty());

        if buttons.is_empty() {
            return;
        }

        self.redraw(cx);
    }

    fn draw_buttons(&self, cx: &mut Cx2d, list: &mut PortalList) {
        list.set_item_range(cx, 0, self.buttons.len());
        while let Some(index) = list.next_visible_item(cx) {
            if index >= self.buttons.len() {
                continue;
            }

            let button = &self.buttons[index];
            let item = list.item(cx, index, live_id!(QuickReplyBtn));

            let (border_color, text_color) = if self.disabled {
                (
                    vec4(0.27, 0.27, 0.27, 1.0),
                    vec4(0.27, 0.27, 0.27, 1.0),
                )
            } else {
                style_colors(button.style)
            };

            item.apply_over(
                cx,
                live! {
                    draw_bg: {
                        border_color: (border_color)
                    }
                    draw_text: {
                        color: (text_color)
                    }
                },
            );

            if self.disabled {
                item.apply_over(cx, live! { cursor: Default });
            }

            item.as_button().set_text(cx, &button.label);
            item.draw_all(cx, &mut Scope::empty());
        }
    }
}

/// Returns `(border_color, text_color)` for a button style.
fn style_colors(style: ButtonStyle) -> (Vec4, Vec4) {
    match style {
        ButtonStyle::Primary => (
            vec4(0.298, 0.686, 0.314, 1.0), // #4CAF50
            vec4(0.298, 0.686, 0.314, 1.0),
        ),
        ButtonStyle::Secondary => (
            vec4(0.129, 0.588, 0.953, 1.0), // #2196F3
            vec4(0.129, 0.588, 0.953, 1.0),
        ),
        ButtonStyle::Subtle => (
            vec4(0.533, 0.533, 0.533, 1.0), // #888888
            vec4(0.533, 0.533, 0.533, 1.0),
        ),
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
