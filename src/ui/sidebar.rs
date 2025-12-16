use crate::app::ClickLiteApp;
use crate::ui::stable_u64_hash;
use gpui::{Context, IntoElement, div, prelude::*, px};
use gpui_component::ActiveTheme as _;
use gpui_component::button::{Button, ButtonVariants as _};
use gpui_component::Selectable;
use gpui_component::skeleton::Skeleton;

pub fn render_sidebar(app: &mut ClickLiteApp, cx: &mut Context<ClickLiteApp>) -> impl IntoElement {
    div()
        .id("sidebar")
        .w(px(260.0))
        .flex_none()
        .flex()
        .flex_col()
        .bg(cx.theme().secondary)
        .border_r_1()
        .border_color(cx.theme().border)
        .child(render_sidebar_header(cx))
        .child(render_channels_header(app, cx))
        .child(render_channel_list(app, cx))
        .child(render_status_bar(app, cx))
}

fn render_sidebar_header(cx: &Context<ClickLiteApp>) -> impl IntoElement {
    div()
        .px_4()
        .py_3()
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

fn render_channels_header(app: &ClickLiteApp, cx: &Context<ClickLiteApp>) -> impl IntoElement {
    div()
        .px_3()
        .py_2()
        .text_xs()
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(cx.theme().muted_foreground)
        .child(if app.channels_loading {
            "CHATS (loading...)"
        } else {
            "CHATS"
        })
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
                    .h(px(20.))
                    .w_full()
                    .when(ix % 2 == 0, |s| s.secondary())
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

fn render_status_bar(app: &ClickLiteApp, cx: &Context<ClickLiteApp>) -> impl IntoElement {
    div()
        .px_3()
        .py_2()
        .border_t_1()
        .border_color(cx.theme().border)
        .text_xs()
        .text_color(cx.theme().muted_foreground)
        .child(app.clickup_status.clone())
}
