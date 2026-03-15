use crate::{
    aitk::{protocol::*, utils::tool::display_name_from_namespaced},
    widgets::{
        attachment_list::AttachmentListWidgetExt,
        attachment_viewer_modal::AttachmentViewerModalWidgetExt,
    },
};

use makepad_widgets::*;

use super::{
    citation_list::CitationListWidgetExt,
    message_thinking_block::MessageThinkingBlockWidgetExt,
    quick_reply_group::QuickReplyGroupWidgetExt,
};

live_design! {
    use link::theme::*;
    use link::widgets::*;
    use link::moly_kit_theme::*;

    use crate::widgets::message_thinking_block::*;
    use crate::widgets::message_markdown::*;
    use crate::widgets::citation_list::*;
    use crate::widgets::quick_reply_group::*;
    use crate::widgets::attachment_list::*;
    use crate::widgets::attachment_viewer_modal::*;

    pub StandardMessageContent = {{StandardMessageContent}} {
        width: Fill,
        flow: Down
        height: Fit,
        spacing: 5
        thinking_block = <MessageThinkingBlock> {}
        markdown = <MessageMarkdown> {}
        quick_replies = <QuickReplyGroup> {}
        citations = <CitationList> { visible: false }
        attachments = <AttachmentList> {}
        attachment_viewer_modal = <AttachmentViewerModal> {}
    }
}

#[derive(Live, Widget, LiveHook)]
pub struct StandardMessageContent {
    #[deref]
    deref: View,
}

impl Widget for StandardMessageContent {
    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.deref.draw_walk(cx, scope, walk)
    }

    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.ui_runner().handle(cx, event, scope, self);
        self.deref.handle_event(cx, event, scope)
    }
}

/// Converts LaTeX bracket math delimiters to dollar-sign delimiters.
/// - `\(...\)` → `$...$` (inline math)
/// - `\[...\]` → `$$...$$` (display math)
fn convert_math_delimiters(text: &str) -> String {
    text.replace(r"\(", "$")
        .replace(r"\)", "$")
        .replace(r"\[", "$$")
        .replace(r"\]", "$$")
}

fn normalize_message_body(text: &str) -> String {
    // Telegram channel sends a limited HTML subset. Converting it into the
    // existing Markdown widget is more reliable than Makepad's Html widget
    // for mixed CJK + emoji content.
    let mut body = String::with_capacity(text.len());
    let mut i = 0;
    let mut bold_depth = 0usize;
    let mut italic_depth = 0usize;
    let mut strike_depth = 0usize;
    let mut code_depth = 0usize;
    let mut spoiler_depth = 0usize;
    let mut blockquote_depth = 0usize;
    let mut in_pre = false;
    let mut link_stack: Vec<String> = Vec::new();

    while i < text.len() {
        let rest = &text[i..];

        if let Some(tag_start) = rest.find('<') {
            let text_part = &rest[..tag_start];
            append_text_fragment(
                &mut body,
                text_part,
                blockquote_depth,
                in_pre,
            );
            i += tag_start;

            let rest = &text[i..];
            let Some(tag_end) = rest.find('>') else {
                append_text_fragment(
                    &mut body,
                    rest,
                    blockquote_depth,
                    in_pre,
                );
                break;
            };

            let tag = &rest[1..tag_end];
            handle_html_tag(
                &mut body,
                tag,
                &mut bold_depth,
                &mut italic_depth,
                &mut strike_depth,
                &mut code_depth,
                &mut spoiler_depth,
                &mut blockquote_depth,
                &mut in_pre,
                &mut link_stack,
            );
            i += tag_end + 1;
        } else {
            append_text_fragment(
                &mut body,
                rest,
                blockquote_depth,
                in_pre,
            );
            break;
        }
    }

    body = body
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&nbsp;", " ");

    let lines: Vec<&str> = body.lines().map(str::trim_end).collect();
    let mut normalized = lines.join("\n");
    while normalized.contains("\n\n\n") {
        normalized = normalized.replace("\n\n\n", "\n\n");
    }
    normalized.trim().to_string()
}

fn append_text_fragment(
    body: &mut String,
    text: &str,
    blockquote_depth: usize,
    in_pre: bool,
) {
    if blockquote_depth == 0 || in_pre {
        body.push_str(text);
        return;
    }

    for ch in text.chars() {
        body.push(ch);
        if ch == '\n' {
            body.push_str("> ");
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_html_tag(
    body: &mut String,
    tag: &str,
    bold_depth: &mut usize,
    italic_depth: &mut usize,
    strike_depth: &mut usize,
    code_depth: &mut usize,
    spoiler_depth: &mut usize,
    blockquote_depth: &mut usize,
    in_pre: &mut bool,
    link_stack: &mut Vec<String>,
) {
    let lower = tag.trim().to_ascii_lowercase();

    match lower.as_str() {
        "br" | "br/" | "br /" => body.push('\n'),
        "p" => {}
        "/p" => body.push_str("\n\n"),
        "blockquote" => {
            ensure_block_spacing(body);
            body.push_str("> ");
            *blockquote_depth += 1;
        }
        "/blockquote" => {
            *blockquote_depth = blockquote_depth.saturating_sub(1);
            body.push_str("\n\n");
        }
        "pre" => {
            ensure_block_spacing(body);
            body.push_str("```\n");
            *in_pre = true;
        }
        "/pre" => {
            if !body.ends_with('\n') {
                body.push('\n');
            }
            body.push_str("```\n\n");
            *in_pre = false;
        }
        "b" | "strong" => toggle_marker(body, bold_depth, "**", true),
        "/b" | "/strong" => toggle_marker(body, bold_depth, "**", false),
        "i" | "em" => toggle_marker(body, italic_depth, "_", true),
        "/i" | "/em" => toggle_marker(body, italic_depth, "_", false),
        "s" | "del" => toggle_marker(body, strike_depth, "~~", true),
        "/s" | "/del" => toggle_marker(body, strike_depth, "~~", false),
        "u" => {}
        "/u" => {}
        "tg-spoiler" => toggle_marker(body, spoiler_depth, "||", true),
        "/tg-spoiler" => toggle_marker(body, spoiler_depth, "||", false),
        "code" if !*in_pre => toggle_marker(body, code_depth, "`", true),
        "/code" if !*in_pre => toggle_marker(body, code_depth, "`", false),
        "/a" => {
            if let Some(url) = link_stack.pop() {
                body.push_str(&format!("]({url})"));
            }
        }
        _ if lower.starts_with("a ") => {
            if let Some(url) = extract_href(tag) {
                body.push('[');
                link_stack.push(url);
            }
        }
        _ if lower.starts_with("code ") => {}
        _ => {}
    }
}

fn toggle_marker(body: &mut String, depth: &mut usize, marker: &str, opening: bool) {
    if opening {
        if *depth == 0 {
            body.push_str(marker);
        }
        *depth += 1;
        return;
    }

    if *depth == 0 {
        return;
    }

    *depth -= 1;
    if *depth == 0 {
        body.push_str(marker);
    }
}

fn ensure_block_spacing(body: &mut String) {
    if body.is_empty() || body.ends_with("\n\n") {
        return;
    }

    if body.ends_with('\n') {
        body.push('\n');
    } else {
        body.push_str("\n\n");
    }
}

fn extract_href(tag: &str) -> Option<String> {
    let href_start = tag.find("href=\"")?;
    let href = &tag[href_start + 6..];
    let href_end = href.find('"')?;
    Some(href[..href_end].to_string())
}

impl StandardMessageContent {
    fn set_content_impl(
        &mut self,
        cx: &mut Cx,
        content: &MessageContent,
        metadata: &MessageMetadata,
    ) {
        /// String to add as suffix to the message text when its being typed.
        const TYPING_INDICATOR: &str = "●";

        let citation_list = self.citation_list(ids!(citations));
        citation_list.borrow_mut().unwrap().urls = content.citations.clone();
        citation_list.borrow_mut().unwrap().visible = !content.citations.is_empty();

        self.quick_reply_group(ids!(quick_replies))
            .set_buttons(cx, &content.quick_replies);

        let mut attachments = self.attachment_list(ids!(attachments));
        attachments.write().attachments = content.attachments.clone();

        let ui = self.ui_runner();
        attachments.write().on_tap(move |list, index| {
            if let Some(attachment) = list.attachments.get(index).cloned() {
                if crate::widgets::attachment_view::can_preview(&attachment) {
                    ui.defer(move |me, cx, _| {
                        let modal = me.attachment_viewer_modal(ids!(attachment_viewer_modal));
                        modal.borrow_mut().unwrap().open(cx, attachment);
                    });
                } else {
                    attachment.save();
                }
            }
        });

        self.message_thinking_block(ids!(thinking_block))
            .borrow_mut()
            .unwrap()
            .set_content(cx, content, metadata);

        let rendered_text = if metadata.is_writing() {
            let text_with_typing = format!("{} {}", content.text, TYPING_INDICATOR);
            normalize_message_body(&convert_math_delimiters(&text_with_typing))
        } else if !content.tool_calls.is_empty() {
            let tool_calls_text = Self::generate_tool_calls_text(content);
            normalize_message_body(&convert_math_delimiters(&tool_calls_text))
        } else {
            normalize_message_body(&convert_math_delimiters(&content.text))
        };
        self.label(ids!(markdown)).set_text(cx, &rendered_text);
    }

    fn generate_tool_calls_text(content: &MessageContent) -> String {
        // Create enhanced text that includes tool calls
        if !content.tool_calls.is_empty() {
            let mut text = content.text.clone();

            if content.tool_calls.len() == 1 {
                let tool_call = &content.tool_calls[0];
                text.push_str(&format!(
                    "🔧 **Requesting permission to call:** `{}`",
                    display_name_from_namespaced(&tool_call.name)
                ));

                if !tool_call.arguments.is_empty() {
                    let args_str = tool_call
                        .arguments
                        .iter()
                        .map(|(k, v)| format!("{}: {}", k, v))
                        .collect::<Vec<_>>()
                        .join(", ");

                    text.push_str(&format!(" with args {}", args_str));
                };
            } else {
                text.push_str(&format!(
                    "🔧 **Requesting permission to call {} tools:**\n",
                    content.tool_calls.len()
                ));
                for tool_call in &content.tool_calls {
                    if !tool_call.arguments.is_empty() {
                        let args_str = format!(
                            "args: `{}`",
                            tool_call
                                .arguments
                                .iter()
                                .map(|(k, v)| format!("{}: {}", k, v))
                                .collect::<Vec<_>>()
                                .join(", ")
                        );
                        text.push_str(&format!(
                            "- `{}` with {}\n",
                            display_name_from_namespaced(&tool_call.name),
                            args_str
                        ));
                    }
                }
            }
            text
        } else {
            content.text.clone()
        }
    }

    /// Set a message content to display it.
    pub fn set_content(&mut self, cx: &mut Cx, content: &MessageContent) {
        self.set_content_impl(cx, content, &MessageMetadata::new());
    }

    /// Same as [`set_content`], but also passes down metadata which is required
    /// by certain features.
    pub fn set_content_with_metadata(
        &mut self,
        cx: &mut Cx,
        content: &MessageContent,
        metadata: &MessageMetadata,
    ) {
        self.set_content_impl(cx, content, metadata);
    }
}

impl StandardMessageContentRef {
    /// See [`StandardMessageContent::set_content`].
    pub fn set_content(&mut self, cx: &mut Cx, content: &MessageContent) {
        let Some(mut inner) = self.borrow_mut() else {
            return;
        };

        inner.set_content(cx, content);
    }

    /// See [`StandardMessageContent::set_content_with_typing`].
    pub fn set_content_with_metadata(
        &mut self,
        cx: &mut Cx,
        content: &MessageContent,
        metadata: &MessageMetadata,
    ) {
        let Some(mut inner) = self.borrow_mut() else {
            return;
        };

        inner.set_content_with_metadata(cx, content, metadata);
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_message_body;

    #[test]
    fn test_normalize_html() {
        let text = "Intro\n\n1. <b>Bold item</b>\n<blockquote>quote</blockquote>";
        assert_eq!(
            normalize_message_body(text),
            "Intro\n\n1. **Bold item**\n\n> quote"
        );
    }

    #[test]
    fn test_normalize_link_and_entities() {
        let text = "<a href=\"https://example.com\">link</a> &amp; &lt;tag&gt;";
        assert_eq!(
            normalize_message_body(text),
            "[link](https://example.com) & <tag>"
        );
    }

    #[test]
    fn test_nested_bold() {
        let text = "<b>🧠 <b>核心功能</b></b>";
        assert_eq!(normalize_message_body(text), "**🧠 核心功能**");
    }
}
