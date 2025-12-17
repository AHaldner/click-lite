use crate::app::ClickLiteApp;
use crate::ui::stable_u64_hash;
use gpui::{Context, IntoElement, Window, div, prelude::*, px};
use gpui_component::ActiveTheme as _;
use gpui_component::Disableable;
use gpui_component::Sizable;
use gpui_component::avatar::Avatar;
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::Input;
use gpui_component::skeleton::Skeleton;
use gpui_component::text::{TextView, TextViewStyle};
use regex::Regex;
use std::iter::repeat_n;
use std::sync::LazyLock;

static LINK_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\[\s*([^\]]*?)\s*\]\(([^)]+)\)").expect("Invalid regex"));

pub fn render_chat_area(
    app: &mut ClickLiteApp,
    window: &mut Window,
    cx: &mut Context<ClickLiteApp>,
) -> impl IntoElement {
    div()
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .child(render_messages(app, window, cx))
        .child(render_input_area(app, window, cx))
}

fn render_messages(
    app: &ClickLiteApp,
    window: &mut Window,
    cx: &mut Context<ClickLiteApp>,
) -> impl IntoElement {
    let scroll_handle = app.scroll_handle.clone();

    div()
        .id("chat_messages")
        .flex_1()
        .min_h_0()
        .overflow_y_scroll()
        .track_scroll(&scroll_handle)
        .p_4()
        .child(if app.selected_channel.is_some() {
            render_message_list(app, window, cx)
        } else {
            render_welcome_message(cx)
        })
}

fn render_message_list(
    app: &ClickLiteApp,
    window: &mut Window,
    cx: &mut Context<ClickLiteApp>,
) -> gpui::AnyElement {
    let current_user_id = app.user.as_ref().map(|u| u.id.to_string());
    let messages: Vec<_> = app.messages().collect();
    let mut rendered_messages = Vec::with_capacity(messages.len());
    for msg in messages {
        let is_own_message = current_user_id
            .as_ref()
            .map(|id| *id == msg.creator_id())
            .unwrap_or(false);
        rendered_messages
            .push(render_message_bubble(msg, is_own_message, window, cx).into_any_element());
    }

    let has_messages = !rendered_messages.is_empty();

    div()
        .flex()
        .flex_col()
        .w_full()
        .gap_3()
        .when(app.messages_loading, |this| {
            this.child(render_messages_loading_placeholder(cx))
        })
        .when(!app.messages_loading && !has_messages, |this| {
            this.child(
                div().p_4().rounded_lg().bg(cx.theme().secondary).child(
                    div()
                        .text_sm()
                        .text_color(cx.theme().muted_foreground)
                        .child("This is the beginning of the conversation."),
                ),
            )
        })
        .children(rendered_messages)
        .into_any_element()
}

fn render_messages_loading_placeholder(cx: &Context<ClickLiteApp>) -> gpui::AnyElement {
    div()
        .flex()
        .flex_col()
        .gap_3()
        .children((0..6).map(|ix| {
            let is_own_message = ix % 3 == 2;
            render_message_skeleton(ix, is_own_message, cx)
        }))
        .into_any_element()
}

fn render_message_skeleton(
    ix: usize,
    is_own_message: bool,
    cx: &Context<ClickLiteApp>,
) -> gpui::AnyElement {
    let name_width = match ix % 3 {
        0 => px(96.),
        1 => px(72.),
        _ => px(110.),
    };

    let (line_1, line_2, line_3) = match ix % 3 {
        0 => (px(280.), Some(px(210.)), None),
        1 => (px(360.), Some(px(140.)), Some(px(220.))),
        _ => (px(220.), None, None),
    };

    let bubble_bg = if is_own_message {
        cx.theme().primary.opacity(0.22)
    } else {
        cx.theme().secondary.opacity(0.55)
    };

    let mut bubble_lines = div()
        .flex()
        .flex_col()
        .gap_1()
        .child(Skeleton::new().h(px(12.)).w(line_1).rounded_sm());

    if let Some(width) = line_2 {
        bubble_lines =
            bubble_lines.child(Skeleton::new().h(px(12.)).w(width).rounded_sm().secondary());
    }

    if let Some(width) = line_3 {
        bubble_lines = bubble_lines.child(Skeleton::new().h(px(12.)).w(width).rounded_sm());
    }

    div()
        .id(("msg_skeleton", ix))
        .flex()
        .gap_3()
        .w_full()
        .when(is_own_message, |this| this.flex_row_reverse())
        .child(
            Skeleton::new()
                .w(px(24.))
                .h(px(24.))
                .rounded_full()
                .secondary(),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .max_w(px(500.0))
                .items_start()
                .when(is_own_message, |this| this.items_end())
                .child(
                    div()
                        .flex()
                        .items_center()
                        .when(is_own_message, |this| this.flex_row_reverse())
                        .child(Skeleton::new().h(px(12.)).w(name_width).rounded_sm()),
                )
                .child(
                    div()
                        .px_3()
                        .py_2()
                        .rounded_lg()
                        .bg(bubble_bg)
                        .max_w(px(500.0))
                        .child(bubble_lines),
                ),
        )
        .into_any_element()
}

fn render_message_bubble(
    msg: &crate::api::ChatMessage,
    is_own_message: bool,
    window: &mut Window,
    cx: &mut Context<ClickLiteApp>,
) -> impl IntoElement {
    let username = msg.creator_name();
    let msg_id = stable_u64_hash(&msg.id);
    let message_content = msg.display_content();
    let is_pending = msg.pending;

    // Note: Profile pictures for message creators are not available without ClickUp Enterprise plan
    let avatar = Avatar::new()
        .name(username.clone())
        .with_size(gpui_component::Size::Small);

    div()
        .id(("msg", msg_id))
        .flex()
        .gap_3()
        .w_full()
        .when(is_own_message, |this| this.flex_row_reverse())
        .when(is_pending, |this| this.opacity(0.6))
        .child(avatar)
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .max_w(px(500.0))
                .items_start()
                .when(is_own_message, |this| this.items_end())
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap_2()
                        .when(is_own_message, |this| this.flex_row_reverse())
                        .child(
                            div()
                                .text_sm()
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(cx.theme().foreground)
                                .child(username),
                        )
                        .when(is_pending, |this| {
                            this.child(
                                div()
                                    .text_xs()
                                    .text_color(cx.theme().muted_foreground)
                                    .child("Sending..."),
                            )
                        }),
                )
                .child(
                    div()
                        .px_3()
                        .py_2()
                        .rounded_lg()
                        .bg(if is_own_message {
                            cx.theme().primary
                        } else {
                            cx.theme().secondary
                        })
                        .text_sm()
                        .text_color(if is_own_message {
                            cx.theme().primary_foreground
                        } else {
                            cx.theme().secondary_foreground
                        })
                        .max_w(px(500.0))
                        .child(render_message_content(
                            msg_id,
                            &message_content,
                            is_own_message,
                            window,
                            cx,
                        )),
                ),
        )
}

fn render_message_content(
    msg_id: u64,
    content: &str,
    is_own_message: bool,
    window: &mut Window,
    cx: &mut Context<ClickLiteApp>,
) -> gpui::AnyElement {
    let base_text_color = if is_own_message {
        cx.theme().primary_foreground
    } else {
        cx.theme().secondary_foreground
    };

    let markdown = normalize_chat_markdown(content);
    TextView::markdown(("msg_content", msg_id), markdown, window, cx)
        .style(TextViewStyle::default().paragraph_gap(gpui::rems(0.25)))
        .text_sm()
        .text_color(base_text_color)
        .selectable(true)
        .into_any_element()
}

fn normalize_chat_markdown(content: &str) -> String {
    let content = fix_clickup_links(content);

    let mut output = String::with_capacity(content.len() * 2);
    let mut in_fence = false;
    let mut fence_char: char = '`';
    let mut fence_len: usize = 0;

    let mut lines = content.lines().peekable();
    while let Some(line) = lines.next() {
        let trimmed = line.trim_start();
        let mut close_fence_after_line = false;

        if let Some((ch, len)) = fence_marker(trimmed) {
            if !in_fence {
                in_fence = true;
                fence_char = ch;
                fence_len = len;
            } else if fence_char == ch && len >= fence_len {
                close_fence_after_line = true;
            }
        }

        if in_fence {
            output.push_str(line);
        } else {
            output.push_str(&normalize_leading_whitespace(line));
        }

        if lines.peek().is_some() {
            if in_fence {
                output.push('\n');
            } else {
                output.push_str("\n\n");
            }
        }

        if close_fence_after_line {
            in_fence = false;
        }
    }

    output
}

fn fix_clickup_links(content: &str) -> String {
    let result = LINK_REGEX.replace_all(content, |caps: &regex::Captures| {
        let display_text = &caps[1];
        let url = &caps[2];

        let clean_display: String = display_text
            .replace("\\_", "_")
            .replace("\\", "")
            .split_whitespace()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(" ");

        let final_display = if clean_display.contains("http") {
            clean_display
                .split_whitespace()
                .next()
                .unwrap_or(&clean_display)
                .to_string()
        } else {
            clean_display
        };

        let clean_url = url.replace("\\_", "_").replace("\\", "");

        format!("[{}]({})", final_display, clean_url)
    });

    result.into_owned()
}

fn fence_marker(line: &str) -> Option<(char, usize)> {
    let mut chars = line.chars();
    let first = chars.next()?;
    if first != '`' && first != '~' {
        return None;
    }

    let mut count = 1;
    for ch in chars {
        if ch == first {
            count += 1;
        } else {
            break;
        }
    }

    (count >= 3).then_some((first, count))
}

fn normalize_leading_whitespace(line: &str) -> String {
    let mut normalized = String::with_capacity(line.len());
    let mut in_prefix = true;

    for ch in line.chars() {
        if in_prefix {
            match ch {
                ' ' => normalized.push('\u{00A0}'),
                '\t' => normalized.extend(repeat_n('\u{00A0}', 4)),
                _ => {
                    in_prefix = false;
                    normalized.push(ch);
                }
            }
        } else {
            normalized.push(ch);
        }
    }

    normalized
}

fn render_welcome_message(cx: &Context<ClickLiteApp>) -> gpui::AnyElement {
    div()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .h_full()
        .gap_2()
        .child(
            div()
                .text_xl()
                .text_color(cx.theme().muted_foreground)
                .child("👋 Welcome to ClickLite"),
        )
        .child(
            div()
                .text_sm()
                .text_color(cx.theme().muted_foreground.opacity(0.8))
                .child("Select a chat from the sidebar to get started."),
        )
        .into_any_element()
}

fn render_input_area(
    app: &ClickLiteApp,
    _window: &Window,
    cx: &mut Context<ClickLiteApp>,
) -> impl IntoElement {
    let has_channel = app.selected_channel.is_some();
    let can_send = has_channel
        && !app
            .message_input
            .read(cx)
            .unmask_value()
            .as_ref()
            .trim()
            .is_empty();
    let app_entity = cx.entity();

    div()
        .id("chat_input")
        .px_4()
        .py_3()
        .border_t_1()
        .border_color(cx.theme().border)
        .flex()
        .gap_2()
        .child(render_text_input(app))
        .when(has_channel, |this| {
            this.child(
                Button::new("send_button")
                    .primary()
                    .label("Send")
                    .h(px(38.0))
                    .disabled(!can_send)
                    .on_click(move |_ev, _window, cx| {
                        app_entity.update(cx, |this, cx| this.send_message(cx));
                    }),
            )
        })
}

fn render_text_input(app: &ClickLiteApp) -> impl IntoElement {
    Input::new(&app.message_input)
        .cleanable(true)
        .disabled(app.selected_channel.is_none())
        .w_full()
        .flex_1()
}
