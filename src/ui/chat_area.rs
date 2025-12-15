use crate::app::ClickLiteApp;
use crate::ui::{colors, stable_u64_hash};
use gpui::{Context, IntoElement, Window, div, prelude::*, px, rgb};

pub fn render_chat_area(
    app: &mut ClickLiteApp,
    window: &Window,
    cx: &mut Context<ClickLiteApp>,
) -> impl IntoElement {
    div()
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .child(render_messages(app))
        .child(render_input_area(app, window, cx))
}

fn render_messages(app: &ClickLiteApp) -> impl IntoElement {
    let scroll_handle = app.scroll_handle.clone();

    div()
        .id("chat_messages")
        .flex_1()
        .min_h_0()
        .overflow_y_scroll()
        .track_scroll(&scroll_handle)
        .p_4()
        .child(if app.selected_channel.is_some() {
            render_message_list(app)
        } else {
            render_welcome_message()
        })
}

fn render_message_list(app: &ClickLiteApp) -> gpui::AnyElement {
    let current_user_id = app.user.as_ref().map(|u| u.id.to_string());

    div()
        .flex()
        .flex_col()
        .gap_3()
        .when(app.messages_loading, |this| {
            this.child(
                div()
                    .text_sm()
                    .text_color(colors::text_secondary())
                    .child("Loading messages..."),
            )
        })
        .when(!app.messages_loading && app.messages.is_empty(), |this| {
            this.child(
                div().p_4().rounded_lg().bg(colors::header_bg()).child(
                    div()
                        .text_sm()
                        .text_color(colors::text_secondary())
                        .child("This is the beginning of the conversation."),
                ),
            )
        })
        .children(app.messages.iter().map(|msg| {
            let is_own_message = current_user_id
                .as_ref()
                .map(|id| *id == msg.creator_id())
                .unwrap_or(false);
            render_message_bubble(msg, is_own_message)
        }))
        .into_any_element()
}

fn render_message_bubble(msg: &crate::api::ChatMessage, is_own_message: bool) -> impl IntoElement {
    let username = msg.creator_name();
    let initial = username.chars().next().unwrap_or('?').to_string();
    let msg_id = stable_u64_hash(&msg.id);

    div()
        .id(("msg", msg_id))
        .flex()
        .gap_3()
        .when(is_own_message, |this| this.flex_row_reverse())
        .child(
            div()
                .size(px(36.0))
                .rounded_full()
                .flex()
                .flex_none()
                .items_center()
                .justify_center()
                .bg(if is_own_message {
                    colors::accent()
                } else {
                    rgb(0x3a3d42).into()
                })
                .text_sm()
                .text_color(rgb(0xffffff))
                .child(initial),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .max_w(px(500.0))
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
                                .text_color(colors::text_primary())
                                .child(username),
                        ),
                )
                .child(
                    div()
                        .px_3()
                        .py_2()
                        .rounded_lg()
                        .bg(if is_own_message {
                            colors::accent()
                        } else {
                            colors::card_bg()
                        })
                        .text_sm()
                        .text_color(colors::text_primary())
                        .child(msg.display_content()),
                ),
        )
}

fn render_welcome_message() -> gpui::AnyElement {
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
                .text_color(colors::text_secondary())
                .child("👋 Welcome to ClickLite"),
        )
        .child(
            div()
                .text_sm()
                .text_color(colors::text_muted())
                .child("Select a chat from the sidebar to get started."),
        )
        .into_any_element()
}

fn render_input_area(
    app: &ClickLiteApp,
    window: &Window,
    cx: &mut Context<ClickLiteApp>,
) -> impl IntoElement {
    let has_channel = app.selected_channel.is_some();
    let placeholder = app
        .selected_channel
        .as_ref()
        .map(|c| format!("Message {}{}", c.icon_prefix(), c.display_name()))
        .unwrap_or_else(|| "Select a chat to start messaging...".to_string());

    div()
        .id("chat_input")
        .px_4()
        .py_3()
        .border_t_1()
        .border_color(colors::divider())
        .flex()
        .gap_2()
        .child(render_text_input(app, placeholder, window))
        .when(has_channel, |this| this.child(render_send_button(app, cx)))
}

fn render_text_input(app: &ClickLiteApp, placeholder: String, window: &Window) -> impl IntoElement {
    let focus_handle = app.focus_handle.clone();
    let is_focused = app.focus_handle.is_focused(window);

    div()
        .id("message_input_container")
        .flex_1()
        .px_4()
        .py_3()
        .rounded_lg()
        .bg(colors::header_bg())
        .border_1()
        .border_color(if is_focused {
            colors::accent()
        } else {
            rgb(0x3a3d42).into()
        })
        .text_sm()
        .cursor_text()
        .on_click(move |_ev, window, _cx| {
            focus_handle.focus(window);
        })
        .child(
            div()
                .flex()
                .items_center()
                .child(if app.message_input.is_empty() {
                    div()
                        .text_color(colors::text_secondary())
                        .child(placeholder)
                        .into_any_element()
                } else {
                    div()
                        .text_color(colors::text_primary())
                        .child(app.message_input.clone())
                        .into_any_element()
                })
                .when(is_focused, |this| {
                    this.child(
                        div()
                            .w(px(2.0))
                            .h(px(18.0))
                            .bg(colors::accent())
                            .flex_none(),
                    )
                }),
        )
}

fn render_send_button(app: &ClickLiteApp, cx: &mut Context<ClickLiteApp>) -> impl IntoElement {
    let can_send = !app.sending_message && !app.message_input.is_empty();

    div()
        .id("send_button")
        .px_4()
        .py_3()
        .rounded_lg()
        .bg(if can_send {
            colors::accent()
        } else {
            colors::card_bg()
        })
        .text_sm()
        .text_color(if can_send {
            rgb(0xffffff).into()
        } else {
            colors::text_secondary()
        })
        .child(if app.sending_message {
            "Sending..."
        } else {
            "Send"
        })
        .when(can_send, |this| {
            this.hover(|h| h.bg(colors::accent_hover()))
                .on_click(cx.listener(|this, _ev, _window, cx| {
                    this.send_message(cx);
                }))
        })
}
