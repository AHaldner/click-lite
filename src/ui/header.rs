use crate::app::ClickLiteApp;
use gpui::{Context, IntoElement, div, img, prelude::*, px};
use gpui_component::ActiveTheme as _;
use gpui_component::avatar::Avatar;

pub fn render_header(app: &mut ClickLiteApp, cx: &mut Context<ClickLiteApp>) -> impl IntoElement {
    div()
        .id("header")
        .h(px(56.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_between()
        .px_4()
        .bg(cx.theme().secondary)
        .border_b_1()
        .border_color(cx.theme().border)
        .child(render_channel_title(app, cx))
        .child(render_user_chip(app, cx))
}

fn render_channel_title(app: &ClickLiteApp, cx: &Context<ClickLiteApp>) -> impl IntoElement {
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
                        .text_color(cx.theme().muted_foreground)
                        .px_2()
                        .py_0p5()
                        .rounded_md()
                        .bg(cx.theme().muted.opacity(0.25))
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
        .bg(cx.theme().popover)
        .border_1()
        .border_color(cx.theme().border)
        .on_click(cx.listener(|this, _ev, _window, cx| {
            this.fetch_clickup_user(cx);
        }))
        .child(render_user_avatar(app))
        .child(render_user_info(app, cx))
}

fn render_user_avatar(app: &ClickLiteApp) -> impl IntoElement {
    if let Some(avatar) = app.user_avatar_image() {
        img(avatar).size(px(28.0)).rounded_full().into_any_element()
    } else {
        let name = app
            .user
            .as_ref()
            .map(|user| user.username.clone())
            .unwrap_or_else(|| "User".to_string());
        Avatar::new().name(name).into_any_element()
    }
}

fn render_user_info(app: &ClickLiteApp, cx: &Context<ClickLiteApp>) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_0p5()
        .child(app.user_display_name())
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(if app.clickup_loading {
                    "Connecting…"
                } else {
                    "Click to reconnect"
                }),
        )
}
