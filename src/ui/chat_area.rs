use crate::app::ClickLiteApp;
use crate::ui::stable_u64_hash;
use gpui::{Context, IntoElement, Window, div, prelude::*, px};
use gpui_component::ActiveTheme as _;
use gpui_component::avatar::Avatar;
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::{Disableable, Sizable};
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
    _window: &mut Window,
    cx: &mut Context<ClickLiteApp>,
) -> gpui::AnyElement {
    let current_user_id = app.user.as_ref().map(|u| u.id.to_string());
    let mut rendered_messages = Vec::with_capacity(app.messages.len());
    for msg in &app.messages {
        let is_own_message = current_user_id
            .as_ref()
            .map(|id| *id == msg.creator_id())
            .unwrap_or(false);
        rendered_messages
            .push(render_message_bubble(msg, is_own_message, cx).into_any_element());
    }

    div()
        .flex()
        .flex_col()
        .w_full()
        .gap_3()
        .when(app.messages_loading, |this| {
            this.child(
                div()
                    .flex()
                    .flex_col()
                    .gap_2()
                    .child(Skeleton::new().h(px(14.)).w(px(260.)))
                    .child(Skeleton::new().h(px(14.)).w(px(420.)).secondary())
                    .child(Skeleton::new().h(px(14.)).w(px(360.)))
                    .child(Skeleton::new().h(px(14.)).w(px(180.)).secondary()),
            )
        })
        .when(!app.messages_loading && app.messages.is_empty(), |this| {
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

fn render_message_bubble(
    msg: &crate::api::ChatMessage,
    is_own_message: bool,
    cx: &mut Context<ClickLiteApp>,
) -> impl IntoElement {
    let username = msg.creator_name();
    let msg_id = stable_u64_hash(&msg.id);
    let message_content = msg.display_content();

    div()
        .id(("msg", msg_id))
        .flex()
        .gap_3()
        .w_full()
        .when(is_own_message, |this| this.flex_row_reverse())
        .child(
            Avatar::new().name(username.clone()).with_size(gpui_component::Size::Small),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .gap_1()
                .flex_1()
                .min_w_0()
                .w_full()
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
                                .text_color(cx.theme().foreground)
                                .child(username),
                        ),
                )
                .child(
                    div()
                        .px_3()
                        .py_2()
                        .rounded_lg()
                        .bg(if is_own_message { cx.theme().primary } else { cx.theme().secondary })
                        .text_sm()
                        .text_color(if is_own_message {
                            cx.theme().primary_foreground
                        } else {
                            cx.theme().secondary_foreground
                        })
                        .w_full()
                        .min_w_0()
                        .child(render_message_content(&message_content)),
                ),
        )
}

fn render_message_content(content: &str) -> gpui::AnyElement {
    let mut lines = Vec::new();

    for line in content.lines() {
        if line.trim().is_empty() {
            lines.push(div().h(px(8.)).into_any_element());
            continue;
        }

        let mut normalized = String::with_capacity(line.len());
        let mut in_prefix = true;

        for ch in line.chars() {
            if in_prefix {
                match ch {
                    ' ' => normalized.push('\u{00A0}'),
                    '\t' => normalized.extend(std::iter::repeat('\u{00A0}').take(4)),
                    _ => {
                        in_prefix = false;
                        normalized.push(ch);
                    }
                }
            } else {
                normalized.push(ch);
            }
        }

        lines.push(
            div()
                .w_full()
                .min_w_0()
                .whitespace_normal()
                .child(normalized)
                .into_any_element(),
        );
    }

    div()
        .flex()
        .flex_col()
        .gap_1()
        .w_full()
        .min_w_0()
        .children(lines)
        .into_any_element()
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
        && !app.sending_message
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
                    .disabled(!can_send)
                    .loading(app.sending_message)
                    .on_click(move |_ev, _window, cx| {
                        app_entity.update(cx, |this, cx| this.send_message(cx));
                    }),
            )
        })
}

fn render_text_input(app: &ClickLiteApp) -> impl IntoElement {
    Input::new(&app.message_input)
        .cleanable(true)
        .disabled(app.selected_channel.is_none() || app.sending_message)
        .w_full()
        .flex_1()
}

 
