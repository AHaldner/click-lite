use crate::app::ClickLiteApp;
use gpui::{Context, IntoElement, div, prelude::*, px};
use gpui_component::ActiveTheme as _;

pub fn render_header(app: &mut ClickLiteApp, cx: &mut Context<ClickLiteApp>) -> impl IntoElement {
    div()
        .id("header")
        .h(px(56.0))
        .flex_none()
        .flex()
        .items_center()
        .justify_between()
        .px_4()
        .bg(cx.theme().background)
        .border_b_1()
        .border_color(cx.theme().border)
        .child(render_channel_title(app, cx))
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
