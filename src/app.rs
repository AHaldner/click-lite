use crate::actions::{Backspace, SendMessage};
use crate::api::{ChatMessage, ClickUpApi, ClickUpChatChannel, ClickUpUser};
use crate::ui::{colors, render_chat_area, render_header, render_sidebar};
use gpui::{Context, FocusHandle, Image, ScrollHandle, SharedString, Window, div, prelude::*};
use std::sync::Arc;

pub struct ClickLiteApp {
    pub clickup_status: SharedString,
    pub clickup_loading: bool,
    pub user: Option<ClickUpUser>,
    pub team_id: Option<u64>,
    pub channels: Vec<ClickUpChatChannel>,
    pub channels_loading: bool,
    pub selected_channel: Option<ClickUpChatChannel>,
    pub messages: Vec<ChatMessage>,
    pub messages_loading: bool,
    pub message_input: String,
    pub sending_message: bool,
    pub focus_handle: FocusHandle,
    pub scroll_handle: ScrollHandle,
}

impl ClickLiteApp {
    pub fn new(team_id: Option<u64>, focus_handle: FocusHandle) -> Self {
        Self {
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
            scroll_handle: ScrollHandle::new(),
        }
    }

    pub fn start_message_refresh(&mut self, cx: &mut Context<Self>) {
        cx.spawn(|this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
            let mut cx = cx.clone();
            async move {
                loop {
                    cx.background_executor()
                        .timer(std::time::Duration::from_secs(5))
                        .await;

                    let should_continue = this
                        .update(&mut cx, |view, cx| {
                            view.refresh_messages(cx);
                            true
                        })
                        .unwrap_or(false);

                    if !should_continue {
                        break;
                    }
                }
            }
        })
        .detach();
    }

    pub fn user_display_name(&self) -> SharedString {
        match self.user.as_ref() {
            Some(user) => user.username.clone().into(),
            None => "Not connected".into(),
        }
    }

    pub fn user_avatar_image(&self) -> Option<Arc<Image>> {
        self.user.as_ref().and_then(|u| u.avatar_image.clone())
    }

    pub fn fetch_clickup_user(&mut self, cx: &mut Context<Self>) {
        if self.clickup_loading {
            return;
        }

        self.clickup_loading = true;
        self.clickup_status = "Connecting to ClickUp…".into();
        cx.notify();

        let api = match ClickUpApi::from_env() {
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

                let (status, user): (String, Option<ClickUpUser>) = match result {
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

    pub fn fetch_channels(&mut self, cx: &mut Context<Self>) {
        if self.channels_loading {
            return;
        }

        let Some(workspace_id) = self.team_id else {
            self.clickup_status =
                "Missing CLICKUP_WORKSPACE_ID (or CLICKUP_TEAM_ID) in .env".into();
            cx.notify();
            return;
        };

        let api = match ClickUpApi::from_env() {
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

    pub fn select_channel(&mut self, channel: ClickUpChatChannel, cx: &mut Context<Self>) {
        self.selected_channel = Some(channel.clone());
        self.messages.clear();
        self.fetch_messages(&channel.id, cx);
        cx.notify();
    }

    pub fn refresh_messages(&mut self, cx: &mut Context<Self>) {
        if let Some(ref channel) = self.selected_channel {
            let channel_id = channel.id.clone();
            self.fetch_messages_silent(&channel_id, cx);
        }
    }

    fn fetch_messages_silent(&mut self, channel_id: &str, cx: &mut Context<Self>) {
        let Some(workspace_id) = self.team_id else {
            return;
        };

        let api = match ClickUpApi::from_env() {
            Ok(api) => api,
            Err(_) => return,
        };

        let channel_id = channel_id.to_string();
        let current_count = self.messages.len();

        cx.spawn(
            move |this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
                let mut cx = cx.clone();
                async move {
                    let result: Result<Vec<ChatMessage>, _> = cx
                        .background_spawn(async move {
                            api.get_channel_messages(workspace_id, &channel_id)
                        })
                        .await;

                    let _ = this.update(&mut cx, |view, cx| {
                        if let Ok(mut messages) = result {
                            messages.reverse();
                            if messages.len() != current_count {
                                view.messages = messages;
                                view.scroll_to_bottom();
                                cx.notify();
                            }
                        }
                    });
                }
            },
        )
        .detach();
    }

    pub fn fetch_messages(&mut self, channel_id: &str, cx: &mut Context<Self>) {
        if self.messages_loading {
            return;
        }

        let Some(workspace_id) = self.team_id else {
            return;
        };

        let api = match ClickUpApi::from_env() {
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
                    let result: Result<Vec<ChatMessage>, _> = cx
                        .background_spawn(async move {
                            api.get_channel_messages(workspace_id, &channel_id)
                        })
                        .await;

                    let _ = this.update(&mut cx, |view, cx| {
                        view.messages_loading = false;
                        if let Ok(mut messages) = result {
                            messages.reverse();
                            view.messages = messages;
                            view.scroll_to_bottom();
                        }
                        cx.notify();
                    });
                }
            },
        )
        .detach();
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_handle.scroll_to_bottom();
    }

    pub fn send_message(&mut self, cx: &mut Context<Self>) {
        if self.sending_message || self.message_input.trim().is_empty() {
            return;
        }

        let Some(workspace_id) = self.team_id else {
            return;
        };

        let Some(ref channel) = self.selected_channel else {
            return;
        };

        let api = match ClickUpApi::from_env() {
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
                            view.scroll_to_bottom();
                        }
                        cx.notify();
                    });
                }
            },
        )
        .detach();
    }

    pub fn handle_input(&mut self, text: &str, cx: &mut Context<Self>) {
        if self.selected_channel.is_some() && !self.sending_message {
            self.message_input.push_str(text);
            cx.notify();
        }
    }

    pub fn handle_backspace(&mut self, cx: &mut Context<Self>) {
        if self.selected_channel.is_some() && !self.sending_message {
            self.message_input.pop();
            cx.notify();
        }
    }
}

impl gpui::Render for ClickLiteApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("root")
            .size_full()
            .flex()
            .flex_row()
            .bg(colors::main_bg())
            .text_color(colors::text_primary())
            .track_focus(&self.focus_handle)
            .child(render_sidebar(self, cx))
            .child(
                div()
                    .id("main")
                    .flex_1()
                    .min_h_0()
                    .flex()
                    .flex_col()
                    .child(render_header(self, cx))
                    .child(render_chat_area(self, window, cx)),
            )
            .on_action(cx.listener(|this, _: &SendMessage, _window, cx| {
                this.send_message(cx);
            }))
            .on_action(cx.listener(|this, _: &Backspace, _window, cx| {
                this.handle_backspace(cx);
            }))
            .on_key_down(
                cx.listener(|this, event: &gpui::KeyDownEvent, _window, cx| {
                    if let Some(input) = event.keystroke.key_char.as_ref() {
                        this.handle_input(&input.to_string(), cx);
                    }
                }),
            )
    }
}
