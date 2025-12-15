use click_lite::api;
use gpui::{
    App, Application, Bounds, Context, Image, ImageFormat, SharedString, Window, WindowBounds,
    WindowOptions, div, img, prelude::*, px, rgb, size,
};
use std::sync::Arc;

struct ClickLiteApp {
    clickup_status: SharedString,
    clickup_loading: bool,
    user: Option<api::clickup::ClickUpUser>,
}

impl ClickLiteApp {
    fn fetch_clickup_user(&mut self, cx: &mut Context<Self>) {
        if self.clickup_loading {
            return;
        }

        self.clickup_loading = true;
        self.clickup_status = "Connecting to ClickUp…".into();
        cx.notify();

        let api = match api::clickup::ClickUpApi::from_env() {
            Ok(api) => api,
            Err(err) => {
                self.clickup_loading = false;
                self.clickup_status = format!("{err}").into();
                cx.notify();
                return;
            }
        };
        cx.spawn(|this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
            let mut cx = cx.clone();
            async move {
                let result = cx
                    .background_spawn(async move { api.get_current_user() })
                    .await;

                let (status, user) = match result {
                    Ok(user) => (format!("Connected as {}", user.username), Some(user)),
                    Err(err) => (format!("Connection failed: {err}"), None),
                };

                let _ = this.update(&mut cx, |view, cx| {
                    view.clickup_loading = false;
                    view.clickup_status = status.into();
                    view.user = user;
                    cx.notify();
                });
            }
        })
        .detach();
    }

    fn user_display_name(&self) -> SharedString {
        match self.user.as_ref() {
            Some(user) => user.username.clone().into(),
            None => "Not connected".into(),
        }
    }

    fn user_avatar_image(&self) -> Option<Arc<Image>> {
        self.user
            .as_ref()
            .and_then(|u| u.avatar_data.as_ref())
            .map(|bytes| Arc::new(Image::from_bytes(ImageFormat::Jpeg, bytes.to_vec())))
    }
}

impl Render for ClickLiteApp {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let sidebar_bg = rgb(0x3f0e40);
        let sidebar_border = rgb(0x2b0a2c);
        let main_bg = rgb(0x1a1d21);
        let header_bg = rgb(0x222529);
        let divider = rgb(0x2c2f33);

        div()
            .id("root")
            .size_full()
            .flex()
            .flex_row()
            .bg(main_bg)
            .text_color(rgb(0xf2f2f2))
            .child(
                div()
                    .id("sidebar")
                    .w(px(260.0))
                    .flex_none()
                    .flex()
                    .flex_col()
                    .bg(sidebar_bg)
                    .border_r_1()
                    .border_color(sidebar_border)
                    .child(
                        div()
                            .px_4()
                            .py_3()
                            .border_b_1()
                            .border_color(sidebar_border)
                            .text_lg()
                            .child("ClickLite"),
                    )
                    .child(
                        div()
                            .p_3()
                            .text_sm()
                            .text_color(rgb(0xd1cbd4))
                            .child("Sidebar (channels, DMs, etc.)"),
                    )
                    .child(
                        div()
                            .mt_auto()
                            .px_3()
                            .py_2()
                            .border_t_1()
                            .border_color(sidebar_border)
                            .text_xs()
                            .text_color(rgb(0xd1cbd4))
                            .child(self.clickup_status.clone()),
                    ),
            )
            .child(
                div()
                    .id("main")
                    .flex_1()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .id("header")
                            .h(px(56.0))
                            .flex_none()
                            .flex()
                            .items_center()
                            .justify_between()
                            .px_4()
                            .bg(header_bg)
                            .border_b_1()
                            .border_color(divider)
                            .child(div().text_base().child("Chat Room"))
                            .child(
                                div()
                                    .id("user_chip")
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .px_2()
                                    .py_1()
                                    .rounded_md()
                                    .bg(rgb(0x2a2d31))
                                    .on_click(cx.listener(|this, _ev, _window, cx| {
                                        this.fetch_clickup_user(cx);
                                    }))
                                    .child({
                                        if let Some(avatar) = self.user_avatar_image() {
                                            img(avatar)
                                                .size(px(28.0))
                                                .rounded_full()
                                                .border_1()
                                                .border_color(divider)
                                                .into_any_element()
                                        } else {
                                            let initial = self
                                                .user
                                                .as_ref()
                                                .and_then(|u| u.username.chars().next())
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
                                    })
                                    .child(
                                        div()
                                            .flex()
                                            .flex_col()
                                            .gap_0p5()
                                            .child(self.user_display_name())
                                            .child(
                                                div().text_xs().text_color(rgb(0xb8bcc4)).child(
                                                    if self.clickup_loading {
                                                        "Connecting…"
                                                    } else {
                                                        "ClickUp"
                                                    },
                                                ),
                                            ),
                                    ),
                            ),
                    )
                    .child(
                        div().id("chat").flex_1().flex().flex_col().p_4().child(
                            div()
                                .text_sm()
                                .text_color(rgb(0xb8bcc4))
                                .child("Chat content area (messages will go here)."),
                        ),
                    ),
            )
    }
}

fn main() {
    let _ = dotenvy::dotenv();
    Application::new().run(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(980.), px(640.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |_, cx| {
                cx.new(|cx| {
                    let mut view = ClickLiteApp {
                        clickup_status: "Not connected".into(),
                        clickup_loading: false,
                        user: None,
                    };
                    view.fetch_clickup_user(cx);
                    view
                })
            },
        )
        .unwrap();
        cx.activate(true);
    });
}
