use crate::app::ClickLiteApp;
use crate::ui::{colors, stable_u64_hash};
use gpui::{Context, IntoElement, div, prelude::*, px, rgb};

pub fn render_sidebar(app: &mut ClickLiteApp, cx: &mut Context<ClickLiteApp>) -> impl IntoElement {
    div()
        .id("sidebar")
        .w(px(260.0))
        .flex_none()
        .flex()
        .flex_col()
        .bg(colors::sidebar_bg())
        .border_r_1()
        .border_color(colors::sidebar_border())
        .child(render_sidebar_header())
        .child(render_channels_header(app))
        .child(render_channel_list(app, cx))
        .child(render_status_bar(app))
}

fn render_sidebar_header() -> impl IntoElement {
    div()
        .px_4()
        .py_3()
        .border_b_1()
        .border_color(colors::sidebar_border())
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

fn render_channels_header(app: &ClickLiteApp) -> impl IntoElement {
    div()
        .px_3()
        .py_2()
        .text_xs()
        .font_weight(gpui::FontWeight::SEMIBOLD)
        .text_color(colors::sidebar_text())
        .child(if app.channels_loading {
            "CHATS (loading...)"
        } else {
            "CHATS"
        })
}

fn render_channel_list(app: &ClickLiteApp, cx: &mut Context<ClickLiteApp>) -> impl IntoElement {
    div()
        .flex_1()
        .px_2()
        .flex()
        .flex_col()
        .gap_0p5()
        .children(app.channels.iter().map(|channel| {
            let channel_clone = channel.clone();
            let element_id = channel
                .id
                .parse::<u64>()
                .unwrap_or_else(|_| stable_u64_hash(&channel.id));
            let display_name = channel.display_name();
            let icon = channel.icon_prefix();
            let is_selected = app
                .selected_channel
                .as_ref()
                .map(|chat| chat.id == channel.id)
                .unwrap_or(false);
            let is_dm = channel.channel_type == "DM";

            div()
                .id(("channel", element_id))
                .px_2()
                .py_1p5()
                .rounded_md()
                .text_sm()
                .text_color(if is_selected {
                    rgb(0xffffff)
                } else {
                    colors::sidebar_text()
                })
                .when(is_selected, |this| this.bg(colors::accent()))
                .when(!is_selected, |this| {
                    this.hover(|h| h.bg(gpui::white().opacity(0.08)))
                })
                .flex()
                .items_center()
                .gap_2()
                .child(
                    div()
                        .text_color(if is_selected {
                            rgb(0xffffff)
                        } else {
                            colors::sidebar_icon()
                        })
                        .child(icon),
                )
                .child(
                    div()
                        .flex_1()
                        .overflow_hidden()
                        .text_ellipsis()
                        .when(is_dm, |this| this.font_weight(gpui::FontWeight::NORMAL))
                        .child(display_name),
                )
                .on_click(cx.listener(move |this, _ev, _window, cx| {
                    this.select_channel(channel_clone.clone(), cx);
                }))
        }))
}

fn render_status_bar(app: &ClickLiteApp) -> impl IntoElement {
    div()
        .px_3()
        .py_2()
        .border_t_1()
        .border_color(colors::sidebar_border())
        .text_xs()
        .text_color(colors::sidebar_icon())
        .child(app.clickup_status.clone())
}
