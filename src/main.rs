use click_lite::api;
use gpui::{
    App, Application, Bounds, Context, FocusHandle, Image, KeyBinding, SharedString, Window,
    WindowBounds, WindowOptions, actions, div, img, prelude::*, px, rgb, size,
};
use std::hash::{Hash, Hasher};
use std::sync::Arc;

actions!(click_lite, [SendMessage, Backspace]);

fn stable_u64_hash(value: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

struct ClickLiteApp {
    clickup_status: SharedString,
    clickup_loading: bool,
    user: Option<api::clickup::ClickUpUser>,
    team_id: Option<u64>,
    channels: Vec<api::clickup::ClickUpChatChannel>,
    channels_loading: bool,
    selected_channel: Option<api::clickup::ClickUpChatChannel>,
    messages: Vec<api::clickup::ChatMessage>,
    messages_loading: bool,
    message_input: String,
    sending_message: bool,
    focus_handle: FocusHandle,
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
                    view.user = user.clone();
                    if user.is_some() {
                        view.fetch_channels(cx);
                    }
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
        self.user.as_ref().and_then(|u| u.avatar_image.clone())
    }

    fn fetch_channels(&mut self, cx: &mut Context<Self>) {
        if self.channels_loading {
            return;
        }

        let Some(workspace_id) = self.team_id else {
            self.clickup_status =
                "Missing CLICKUP_WORKSPACE_ID (or CLICKUP_TEAM_ID) in .env".into();
            cx.notify();
            return;
        };

        let api = match api::clickup::ClickUpApi::from_env() {
            Ok(api) => api,
            Err(err) => {
                self.clickup_status = format!("{err}").into();
                cx.notify();
                return;
            }
        };

        self.channels_loading = true;
        self.clickup_status = "Loading chats…".into();
        cx.notify();

        let current_user_id = self.user.as_ref().map(|u| u.id);

        cx.spawn(
            move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
                let mut cx = cx.clone();
                async move {
                    let result = cx
                        .background_spawn(async move {
                            api.get_chat_channels(workspace_id, current_user_id)
                        })
                        .await;

                    let _ = this.update(&mut cx, |view, cx| {
                        view.channels_loading = false;
                        match result {
                            Ok(channels) => {
                                view.channels = channels;
                                view.clickup_status = "Ready".into();
                            }
                            Err(err) => view.clickup_status = format!("Error: {err}").into(),
                        }
                        cx.notify();
                    });
                }
            },
        )
        .detach();
    }

    fn select_channel(
        &mut self,
        channel: api::clickup::ClickUpChatChannel,
        cx: &mut Context<Self>,
    ) {
        self.selected_channel = Some(channel.clone());
        self.messages.clear();
        self.fetch_messages(&channel.id, cx);
        cx.notify();
    }

    fn fetch_messages(&mut self, channel_id: &str, cx: &mut Context<Self>) {
        if self.messages_loading {
            return;
        }

        let Some(workspace_id) = self.team_id else {
            return;
        };

        let api = match api::clickup::ClickUpApi::from_env() {
            Ok(api) => api,
            Err(_) => return,
        };

        self.messages_loading = true;
        cx.notify();

        let channel_id = channel_id.to_string();

        cx.spawn(
            move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
                let mut cx = cx.clone();
                async move {
                    let result = cx
                        .background_spawn(async move {
                            api.get_channel_messages(workspace_id, &channel_id)
                        })
                        .await;

                    let _ = this.update(&mut cx, |view, cx| {
                        view.messages_loading = false;
                        if let Ok(mut messages) = result {
                            messages.reverse();
                            view.messages = messages;
                        }
                        cx.notify();
                    });
                }
            },
        )
        .detach();
    }

    fn send_message(&mut self, cx: &mut Context<Self>) {
        if self.sending_message || self.message_input.trim().is_empty() {
            return;
        }

        let Some(workspace_id) = self.team_id else {
            return;
        };

        let Some(ref channel) = self.selected_channel else {
            return;
        };

        let api = match api::clickup::ClickUpApi::from_env() {
            Ok(api) => api,
            Err(_) => return,
        };

        self.sending_message = true;
        let content = self.message_input.clone();
        self.message_input.clear();
        cx.notify();

        let channel_id = channel.id.clone();

        cx.spawn(
            move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
                let mut cx = cx.clone();
                async move {
                    let result = cx
                        .background_spawn(async move {
                            api.send_message(workspace_id, &channel_id, &content)
                        })
                        .await;

                    let _ = this.update(&mut cx, |view, cx| {
                        view.sending_message = false;
                        if let Ok(message) = result {
                            view.messages.push(message);
                        }
                        cx.notify();
                    });
                }
            },
        )
        .detach();
    }

    fn handle_input(&mut self, text: &str, cx: &mut Context<Self>) {
        if self.selected_channel.is_some() && !self.sending_message {
            self.message_input.push_str(text);
            cx.notify();
        }
    }

    fn handle_backspace(&mut self, cx: &mut Context<Self>) {
        if self.selected_channel.is_some() && !self.sending_message {
            self.message_input.pop();
            cx.notify();
        }
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
            .track_focus(&self.focus_handle)
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
                            .flex()
                            .items_center()
                            .gap_2()
                            .child(
                                div()
                                    .text_lg()
                                    .font_weight(gpui::FontWeight::SEMIBOLD)
                                    .child("ClickLite"),
                            ),
                    )
                    .child(
                        div()
                            .px_3()
                            .py_2()
                            .text_xs()
                            .font_weight(gpui::FontWeight::SEMIBOLD)
                            .text_color(rgb(0xd1cbd4))
                            .child(if self.channels_loading {
                                "CHATS (loading...)"
                            } else {
                                "CHATS"
                            }),
                    )
                    .child(div().flex_1().px_2().flex().flex_col().gap_0p5().children(
                        self.channels.iter().map(|channel| {
                            let channel_clone = channel.clone();
                            let element_id = channel
                                .id
                                .parse::<u64>()
                                .unwrap_or_else(|_| stable_u64_hash(&channel.id));
                            let display_name = channel.display_name();
                            let icon = channel.icon_prefix();
                            let is_selected = self
                                .selected_channel
                                .as_ref()
                                .map(|c| c.id == channel.id)
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
                                    rgb(0xd1cbd4)
                                })
                                .when(is_selected, |this| this.bg(rgb(0x1164a3)))
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
                                            rgb(0x9d9da0)
                                        })
                                        .child(icon),
                                )
                                .child(
                                    div()
                                        .flex_1()
                                        .overflow_hidden()
                                        .text_ellipsis()
                                        .when(is_dm, |this| {
                                            this.font_weight(gpui::FontWeight::NORMAL)
                                        })
                                        .child(display_name),
                                )
                                .on_click(cx.listener(move |this, _ev, _window, cx| {
                                    this.select_channel(channel_clone.clone(), cx);
                                }))
                        }),
                    ))
                    .child(
                        div()
                            .px_3()
                            .py_2()
                            .border_t_1()
                            .border_color(sidebar_border)
                            .text_xs()
                            .text_color(rgb(0x9d9da0))
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
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(
                                        div()
                                            .text_base()
                                            .font_weight(gpui::FontWeight::SEMIBOLD)
                                            .child(
                                                self.selected_channel
                                                    .as_ref()
                                                    .map(|c| {
                                                        format!(
                                                            "{}{}",
                                                            c.icon_prefix(),
                                                            c.display_name()
                                                        )
                                                    })
                                                    .unwrap_or_else(|| "ClickLite".to_string()),
                                            ),
                                    )
                                    .child(
                                        self.selected_channel
                                            .as_ref()
                                            .map(|c| {
                                                div()
                                                    .text_xs()
                                                    .text_color(rgb(0x8e9297))
                                                    .px_2()
                                                    .py_0p5()
                                                    .rounded_md()
                                                    .bg(rgb(0x2a2d31))
                                                    .child(c.channel_type.clone())
                                                    .into_any_element()
                                            })
                                            .unwrap_or_else(|| div().into_any_element()),
                                    ),
                            )
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
                                                        "Click to reconnect"
                                                    },
                                                ),
                                            ),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .id("chat_messages")
                            .flex_1()
                            .overflow_y_scroll()
                            .p_4()
                            .child(if self.selected_channel.is_some() {
                                let current_user_id = self.user.as_ref().map(|u| u.id.to_string());
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap_3()
                                    .when(self.messages_loading, |this| {
                                        this.child(
                                            div()
                                                .text_sm()
                                                .text_color(rgb(0x8e9297))
                                                .child("Loading messages..."),
                                        )
                                    })
                                    .when(!self.messages_loading && self.messages.is_empty(), |this| {
                                        this.child(
                                            div()
                                                .p_4()
                                                .rounded_lg()
                                                .bg(rgb(0x222529))
                                                .child(
                                                    div()
                                                        .text_sm()
                                                        .text_color(rgb(0x8e9297))
                                                        .child("This is the beginning of the conversation."),
                                                ),
                                        )
                                    })
                                    .children(self.messages.iter().map(|msg| {
                                        let is_own_message = current_user_id
                                            .as_ref()
                                            .map(|id| *id == msg.creator_id())
                                            .unwrap_or(false);
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
                                                        rgb(0x1164a3)
                                                    } else {
                                                        rgb(0x3a3d42)
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
                                                                    .text_color(rgb(0xf2f2f2))
                                                                    .child(username),
                                                            ),
                                                    )
                                                    .child(
                                                        div()
                                                            .px_3()
                                                            .py_2()
                                                            .rounded_lg()
                                                            .bg(if is_own_message {
                                                                rgb(0x1164a3)
                                                            } else {
                                                                rgb(0x2a2d31)
                                                            })
                                                            .text_sm()
                                                            .text_color(rgb(0xf2f2f2))
                                                            .child(msg.display_content()),
                                                    ),
                                            )
                                    }))
                                    .into_any_element()
                            } else {
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
                                            .text_color(rgb(0x8e9297))
                                            .child("👋 Welcome to ClickLite"),
                                    )
                                    .child(
                                        div().text_sm().text_color(rgb(0x6e7177)).child(
                                            "Select a chat from the sidebar to get started.",
                                        ),
                                    )
                                    .into_any_element()
                            }),
                    )
                    .child({
                        let has_channel = self.selected_channel.is_some();
                        let placeholder = self
                            .selected_channel
                            .as_ref()
                            .map(|c| format!("Message {}{}", c.icon_prefix(), c.display_name()))
                            .unwrap_or_else(|| "Select a chat to start messaging...".to_string());

                        div()
                            .id("chat_input")
                            .px_4()
                            .py_3()
                            .border_t_1()
                            .border_color(divider)
                            .flex()
                            .gap_2()
                            .child({
                                let focus_handle = self.focus_handle.clone();
                                div()
                                    .id("message_input_container")
                                    .flex_1()
                                    .px_4()
                                    .py_3()
                                    .rounded_lg()
                                    .bg(rgb(0x222529))
                                    .border_1()
                                    .border_color(rgb(0x3a3d42))
                                    .text_sm()
                                    .cursor_text()
                                    .on_click(move |_ev, window, _cx| {
                                        focus_handle.focus(window);
                                    })
                                    .child(if self.message_input.is_empty() {
                                        div()
                                            .text_color(rgb(0x8e9297))
                                            .child(placeholder)
                                            .into_any_element()
                                    } else {
                                        div()
                                            .text_color(rgb(0xf2f2f2))
                                            .flex()
                                            .child(self.message_input.clone())
                                            .child(
                                                div()
                                                    .w(px(2.0))
                                                    .h(px(16.0))
                                                    .bg(rgb(0x1164a3))
                                                    .ml_0p5()
                                            )
                                            .into_any_element()
                                    })
                            })
                            .when(has_channel, |this| {
                                this.child(
                                    div()
                                        .id("send_button")
                                        .px_4()
                                        .py_3()
                                        .rounded_lg()
                                        .bg(if self.sending_message || self.message_input.is_empty() {
                                            rgb(0x2a2d31)
                                        } else {
                                            rgb(0x1164a3)
                                        })
                                        .text_sm()
                                        .text_color(if self.sending_message || self.message_input.is_empty() {
                                            rgb(0x8e9297)
                                        } else {
                                            rgb(0xffffff)
                                        })
                                        .child(if self.sending_message { "Sending..." } else { "Send" })
                                        .when(!self.sending_message && !self.message_input.is_empty(), |this| {
                                            this.hover(|h| h.bg(rgb(0x0d5a8c)))
                                                .on_click(cx.listener(|this, _ev, _window, cx| {
                                                    this.send_message(cx);
                                                }))
                                        }),
                                )
                            })
                    }),
            )
            .on_action(cx.listener(|this, _: &SendMessage, _window, cx| {
                this.send_message(cx);
            }))
            .on_action(cx.listener(|this, _: &Backspace, _window, cx| {
                this.handle_backspace(cx);
            }))
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _window, cx| {
                if let Some(input) = event.keystroke.key_char.as_ref() {
                    this.handle_input(&input.to_string(), cx);
                }
            }))
    }
}

fn main() {
    let _ = dotenvy::dotenv();
    Application::new().run(|cx: &mut App| {
        cx.bind_keys([
            KeyBinding::new("enter", SendMessage, None),
            KeyBinding::new("backspace", Backspace, None),
        ]);

        let bounds = Bounds::centered(None, size(px(980.), px(640.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                ..Default::default()
            },
            |window, cx| {
                cx.new(|cx| {
                    let team_id = std::env::var("CLICKUP_WORKSPACE_ID")
                        .or_else(|_| std::env::var("CLICKUP_TEAM_ID"))
                        .ok()
                        .and_then(|v| v.parse::<u64>().ok());

                    let focus_handle = cx.focus_handle();
                    focus_handle.focus(window);

                    let mut view = ClickLiteApp {
                        clickup_status: "Connecting...".into(),
                        clickup_loading: false,
                        user: None,
                        team_id,
                        channels: Vec::new(),
                        channels_loading: false,
                        selected_channel: None,
                        messages: Vec::new(),
                        messages_loading: false,
                        message_input: String::new(),
                        sending_message: false,
                        focus_handle,
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
