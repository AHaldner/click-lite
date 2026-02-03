use crate::app::ClickLiteApp;
use crate::ui::rich_text::RichText;
use crate::ui::{AVATAR_SIZE, MESSAGE_BUBBLE_MAX_WIDTH, MESSAGE_COLUMN_MAX_WIDTH, stable_u64_hash};
use gpui::{Context, IntoElement, Window, div, prelude::*, px};
use gpui_component::ActiveTheme as _;
use gpui_component::Disableable;
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::input::Input;
use gpui_component::skeleton::Skeleton;

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
    let content = if app.selected_channel.is_some() {
        div()
            .flex()
            .justify_center()
            .child(
                div()
                    .w_full()
                    .max_w(px(MESSAGE_COLUMN_MAX_WIDTH))
                    .child(render_message_list(app, window, cx)),
            )
            .into_any_element()
    } else {
        render_welcome_message(cx)
    };

    div()
        .id("chat_messages")
        .flex_1()
        .min_h_0()
        .overflow_y_scroll()
        .track_scroll(&scroll_handle)
        .p_4()
        .child(content)
}

fn render_message_list(
    app: &ClickLiteApp,
    window: &mut Window,
    cx: &mut Context<ClickLiteApp>,
) -> gpui::AnyElement {
    let current_user_id = app.user.as_ref().map(|u| u.id.to_string());
    let messages: Vec<_> = app.messages().collect();
    let mut groups: Vec<MessageGroup> = Vec::new();
    for msg in messages {
        let is_own_message = current_user_id
            .as_ref()
            .map(|id| *id == msg.creator_id())
            .unwrap_or(false);
        let creator_id = msg.creator_id().to_string();
        let username = msg.creator_name();
        let is_pending = msg.pending;

        if let Some(last) = groups.last_mut() {
            if last.creator_id == creator_id && last.is_own == is_own_message {
                last.messages.push(msg);
                last.has_pending |= is_pending;
                continue;
            }
        }

        groups.push(MessageGroup {
            creator_id,
            username,
            is_own: is_own_message,
            has_pending: is_pending,
            messages: vec![msg],
        });
    }

    let has_messages = !groups.is_empty();

    div()
        .flex()
        .flex_col()
        .w_full()
        .gap_4()
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
        .children(
            groups
                .iter()
                .map(|group| render_message_group(group, window, cx).into_any_element()),
        )
        .into_any_element()
}

struct MessageGroup<'a> {
    creator_id: String,
    username: String,
    is_own: bool,
    has_pending: bool,
    messages: Vec<&'a crate::api::ChatMessage>,
}

fn render_message_group(
    group: &MessageGroup<'_>,
    window: &mut Window,
    cx: &mut Context<ClickLiteApp>,
) -> impl IntoElement {
    let header_inner = div()
        .flex()
        .items_center()
        .gap_3()
        .when(group.is_own, |this| this.flex_row_reverse())
        .child(render_message_avatar(&group.username, cx))
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .when(group.is_own, |this| this.flex_row_reverse())
                .child(
                    div()
                        .text_sm()
                        .font_weight(gpui::FontWeight::SEMIBOLD)
                        .text_color(cx.theme().foreground)
                        .child(group.username.clone()),
                )
                .when(group.has_pending, |this| {
                    this.child(
                        div()
                            .text_xs()
                            .text_color(cx.theme().muted_foreground)
                            .px_2()
                            .py_0p5()
                            .rounded_md()
                            .bg(cx.theme().muted.opacity(0.5))
                            .child("Sending"),
                    )
                }),
        );

    let header = div()
        .flex()
        .justify_start()
        .when(group.is_own, |this| this.justify_end())
        .child(header_inner);

    let bubble_indent = px(AVATAR_SIZE + 12.0);

    let message_stack =
        div()
            .flex()
            .flex_col()
            .gap_1()
            .w_full()
            .when(group.is_own, |this| this.items_end().pr(bubble_indent))
            .when(!group.is_own, |this| this.items_start().pl(bubble_indent))
            .children(group.messages.iter().map(|msg| {
                render_message_bubble(msg, group.is_own, window, cx).into_any_element()
            }));

    div()
        .flex()
        .flex_col()
        .gap_1()
        .w_full()
        .child(header)
        .child(message_stack)
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
                .max_w(px(MESSAGE_BUBBLE_MAX_WIDTH))
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
                        .max_w(px(MESSAGE_BUBBLE_MAX_WIDTH))
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
    let msg_id = stable_u64_hash(&msg.id);
    let message_content = msg.display_content();
    let is_pending = msg.pending;

    div()
        .id(("msg", msg_id))
        .flex()
        .when(is_pending, |this| this.opacity(0.6))
        .child(
            div()
                .px_3()
                .py_2()
                .rounded_lg()
                .bg(if is_own_message {
                    cx.theme().primary.opacity(0.18)
                } else {
                    cx.theme().secondary.opacity(0.6)
                })
                .text_sm()
                .text_color(cx.theme().foreground)
                .max_w(px(MESSAGE_BUBBLE_MAX_WIDTH))
                .child(render_message_content(
                    msg_id,
                    &message_content,
                    is_own_message,
                    window,
                    cx,
                )),
        )
}

fn render_message_content(
    msg_id: u64,
    content: &str,
    is_own_message: bool,
    window: &mut Window,
    cx: &mut Context<ClickLiteApp>,
) -> gpui::AnyElement {
    let base_text_color = cx.theme().foreground;
    let code_bg = cx.theme().muted.opacity(0.5);
    let link_color = if is_own_message {
        cx.theme().foreground
    } else {
        cx.theme().link
    };

    let rich_text = RichText::from_markdown(content, code_bg, link_color);
    rich_text.element(("msg_content", msg_id), base_text_color, window, cx)
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

fn render_message_avatar(username: &str, cx: &Context<ClickLiteApp>) -> impl IntoElement {
    let initials = extract_initials(username);
    let (avatar_bg, avatar_text) = avatar_color_for_name(username, cx);
    div()
        .size(px(AVATAR_SIZE))
        .flex()
        .items_center()
        .justify_center()
        .flex_shrink_0()
        .rounded_full()
        .bg(avatar_bg)
        .text_color(avatar_text)
        .text_xs()
        .font_weight(gpui::FontWeight::MEDIUM)
        .child(initials)
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
        .bg(cx.theme().background)
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

/// Extract initials from a name (e.g., "John Doe" -> "JD", "alice" -> "AL")
fn extract_initials(name: &str) -> String {
    let mut result: String = name
        .split_whitespace()
        .filter_map(|word| word.chars().next())
        .take(2)
        .collect();

    if result.len() == 1 {
        result = name.chars().take(2).collect();
    }

    result.to_uppercase()
}

/// Generate a consistent color for a username (same name = same color)
fn avatar_color_for_name(name: &str, cx: &Context<ClickLiteApp>) -> (gpui::Hsla, gpui::Hsla) {
    use gpui_component::ActiveTheme;

    // Hash the name to get a consistent index
    let hash = stable_u64_hash(name);

    // Use 24 different hues (every 15 degrees on the color wheel)
    let hue = ((hash % 24) * 15) as f32 / 360.0;

    // Get the base blue color and shift its hue
    let base_color = cx.theme().blue;
    let color = gpui::Hsla {
        h: hue,
        s: base_color.s,
        l: base_color.l,
        a: base_color.a,
    };

    // Background is the color at 20% opacity, text is the full color
    (color.opacity(0.2), color)
}
