use crate::app::ClickLiteApp;
use crate::ui::stable_u64_hash;
use gpui::{Context, IntoElement, div, img, prelude::*, px};
use gpui_component::ActiveTheme as _;
use gpui_component::Selectable;
use gpui_component::Sizable;
use gpui_component::avatar::Avatar;
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::skeleton::Skeleton;

pub fn render_sidebar(app: &mut ClickLiteApp, cx: &mut Context<ClickLiteApp>) -> impl IntoElement {
    div()
        .id("sidebar")
        .w(px(260.0))
        .flex_none()
        .flex()
        .flex_col()
        .bg(cx.theme().background)
        .border_r_1()
        .border_color(cx.theme().border)
        .child(render_sidebar_header(cx))
        .child(render_channels_header(cx))
        .child(render_channel_list(app, cx))
        .child(render_sidebar_footer(app, cx))
}

fn render_sidebar_header(cx: &Context<ClickLiteApp>) -> impl IntoElement {
    div()
        .h(px(56.0))
        .flex_none()
        .px_4()
        .py_0()
        .border_b_1()
        .border_color(cx.theme().border)
        .flex()
        .items_center()
        .gap_2()
        .child(
            div()
                .text_lg()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .child("ClickLite"),
        )
}

fn render_channels_header(cx: &Context<ClickLiteApp>) -> impl IntoElement {
    div()
        .px_3()
        .py_2()
        .text_xs()
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(cx.theme().muted_foreground)
        .child("CHATS")
}

fn render_channel_list(app: &ClickLiteApp, cx: &mut Context<ClickLiteApp>) -> impl IntoElement {
    let app_entity = cx.entity();
    let channels = if app.channels_loading {
        div()
            .flex()
            .flex_col()
            .gap_1()
            .px_2()
            .children((0..10).map(|ix| {
                Skeleton::new()
                    .h(px(25.))
                    .w_full()
                    .when(ix % 2 == 0, |skeleton| skeleton.secondary())
                    .into_any_element()
            }))
            .into_any_element()
    } else {
        div()
            .children(app.channels.iter().map(|channel| {
                let channel_clone = channel.clone();
                let element_id = channel
                    .id
                    .parse::<u64>()
                    .unwrap_or_else(|_| stable_u64_hash(&channel.id));
                let display_name = channel.display_name();
                let is_selected = app
                    .selected_channel
                    .as_ref()
                    .map(|chat| chat.id == channel.id)
                    .unwrap_or(false);

                Button::new(("channel", element_id))
                    .ghost()
                    .selected(is_selected)
                    .w_full()
                    .justify_start()
                    .label(format!("{}{}", channel_clone.icon_prefix(), display_name))
                    .on_click({
                        let app_entity = app_entity.clone();
                        move |_ev, _window, cx| {
                            app_entity.update(cx, |this, cx| {
                                this.select_channel(channel_clone.clone(), cx);
                            });
                        }
                    })
            }))
            .into_any_element()
    };

    div()
        .flex_1()
        .px_2()
        .flex()
        .flex_col()
        .gap_0p5()
        .child(channels)
}

fn render_sidebar_footer(app: &ClickLiteApp, cx: &mut Context<ClickLiteApp>) -> impl IntoElement {
    div()
        .p_3()
        .flex()
        .flex_col()
        .gap_2()
        .child(render_user_chip(app, cx))
}

fn render_user_chip(app: &ClickLiteApp, cx: &mut Context<ClickLiteApp>) -> impl IntoElement {
    let status_color = match app {
        _ if app.clickup_loading => cx.theme().warning,
        _ if app.user.is_some() => cx.theme().success,
        _ => cx.theme().danger,
    };

    div()
        .id("user_chip")
        .flex()
        .items_center()
        .justify_between()
        .px_3()
        .py_1()
        .rounded_md()
        .bg(cx.theme().accent.opacity(0.18))
        .border_1()
        .border_color(cx.theme().border.opacity(0.85))
        .cursor_pointer()
        .on_click(cx.listener(|this, _ev, _window, cx| {
            this.fetch_clickup_user(cx);
        }))
        .child(
            div()
                .flex()
                .items_center()
                .gap_2()
                .min_w_0()
                .child(render_user_avatar(app))
                .child(render_user_info(app, cx)),
        )
        .child(div().size(px(8.)).rounded_full().bg(status_color))
}

fn render_user_avatar(app: &ClickLiteApp) -> impl IntoElement {
    if let Some(avatar) = app.user_avatar_image() {
        img(avatar).size(px(26.0)).rounded_full().into_any_element()
    } else {
        let name = app
            .user
            .as_ref()
            .map(|user| user.username.clone())
            .unwrap_or_else(|| "User".to_string());
        Avatar::new()
            .name(name)
            .with_size(gpui_component::Size::Small)
            .into_any_element()
    }
}

fn render_user_info(app: &ClickLiteApp, cx: &Context<ClickLiteApp>) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap_0p5()
        .child(
            div()
                .text_sm()
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .child(app.user_display_name()),
        )
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(if app.clickup_loading {
                    "Connecting…"
                } else if app.user.is_some() {
                    "Connected"
                } else {
                    "Click to connect"
                }),
        )
}
