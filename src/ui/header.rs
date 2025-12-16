use crate::app::ClickLiteApp;
use crate::ui::colors;
use gpui::{Context, IntoElement, div, img, prelude::*, px, rgb};

pub fn render_header(app: &mut ClickLiteApp, cx: &mut Context<ClickLiteApp>) -> impl IntoElement {
    div()
        .id("header")
        .h(px(56.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_between()
        .px_4()
        .bg(colors::header_bg())
        .border_b_1()
        .border_color(colors::divider())
        .child(render_channel_title(app))
        .child(render_user_chip(app, cx))
}

fn render_channel_title(app: &ClickLiteApp) -> impl IntoElement {
    div()
        .flex()
        .items_center()
        .gap_2()
        .child(
            div()
                .text_base()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .child(
                    app.selected_channel
                        .as_ref()
                        .map(|channel| {
                            format!("{}{}", channel.icon_prefix(), channel.display_name())
                        })
                        .unwrap_or_else(|| "ClickLite".to_string()),
                ),
        )
        .child(
            app.selected_channel
                .as_ref()
                .map(|channel| {
                    div()
                        .text_xs()
                        .text_color(colors::text_secondary())
                        .px_2()
                        .py_0p5()
                        .rounded_md()
                        .bg(colors::card_bg())
                        .child(channel.channel_type.clone())
                        .into_any_element()
                })
                .unwrap_or_else(|| div().into_any_element()),
        )
}

fn render_user_chip(app: &ClickLiteApp, cx: &mut Context<ClickLiteApp>) -> impl IntoElement {
    div()
        .id("user_chip")
        .flex()
        .items_center()
        .gap_2()
        .px_2()
        .py_1()
        .rounded_md()
        .bg(colors::card_bg())
        .on_click(cx.listener(|this, _ev, _window, cx| {
            this.fetch_clickup_user(cx);
        }))
        .child(render_user_avatar(app))
        .child(render_user_info(app))
}

fn render_user_avatar(app: &ClickLiteApp) -> impl IntoElement {
    if let Some(avatar) = app.user_avatar_image() {
        img(avatar)
            .size(px(28.0))
            .rounded_full()
            .border_1()
            .border_color(colors::divider())
            .into_any_element()
    } else {
        let initial = app
            .user
            .as_ref()
            .and_then(|user| user.username.chars().next())
            .unwrap_or('?')
            .to_string();
        div()
            .size(px(28.0))
            .rounded_full()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgb(0x3a3d42))
            .text_sm()
            .child(initial)
            .into_any_element()
    }
}

fn render_user_info(app: &ClickLiteApp) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_0p5()
        .child(app.user_display_name())
        .child(
            div()
                .text_xs()
                .text_color(rgb(0xb8bcc4))
                .child(if app.clickup_loading {
                    "Connecting…"
                } else {
                    "Click to reconnect"
                }),
        )
}
