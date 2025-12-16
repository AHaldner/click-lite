use crate::api::{ChatMessage, ClickUpApi, ClickUpChatChannel, ClickUpUser};
use crate::ui::{render_chat_area, render_header, render_sidebar};
use gpui::{
    AnyWindowHandle, Context, Entity, FocusHandle, Image, ScrollHandle, SharedString, Subscription,
    Window, div, prelude::*,
};
use gpui_component::ActiveTheme as _;
use gpui_component::input::{InputEvent, InputState};
use std::collections::HashSet;
use std::sync::Arc;

pub struct ClickLiteApp {
    pub clickup_status: SharedString,
    pub clickup_loading: bool,
    pub user: Option<ClickUpUser>,
    pub team_id: Option<u64>,
    pub channels: Vec<ClickUpChatChannel>,
    pub channels_loading: bool,
    pub selected_channel: Option<ClickUpChatChannel>,
    pub messages_loading: bool,
    pub focus_handle: FocusHandle,
    pub scroll_handle: ScrollHandle,
    pub window_handle: AnyWindowHandle,
    pub message_input: Entity<InputState>,
    server_messages: Vec<ChatMessage>,
    pending_messages: Vec<ChatMessage>,
    pending_ids: HashSet<String>,
    _subscriptions: Vec<Subscription>,
}

impl ClickLiteApp {
    pub fn new(
        team_id: Option<u64>,
        focus_handle: FocusHandle,
        window_handle: AnyWindowHandle,
        message_input: Entity<InputState>,
        cx: &mut Context<Self>,
    ) -> Self {
        let mut app = Self {
            clickup_status: "Connecting...".into(),
            clickup_loading: false,
            user: None,
            team_id,
            channels: Vec::new(),
            channels_loading: false,
            selected_channel: None,
            server_messages: Vec::new(),
            pending_messages: Vec::new(),
            pending_ids: HashSet::new(),
            messages_loading: false,
            focus_handle,
            scroll_handle: ScrollHandle::new(),
            window_handle,
            message_input: message_input.clone(),
            _subscriptions: Vec::new(),
        };

        app._subscriptions.push(cx.subscribe(
            &message_input,
            |this, _input, event: &InputEvent, cx| {
                if let InputEvent::PressEnter { secondary: false } = event {
                    this.send_message(cx);
                }
            },
        ));

        app
    }

    pub fn messages(&self) -> impl Iterator<Item = &ChatMessage> {
        self.server_messages
            .iter()
            .chain(self.pending_messages.iter())
    }

    pub fn sending_message(&self) -> bool {
        !self.pending_ids.is_empty()
    }

    fn set_message_input_placeholder(
        &self,
        placeholder: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        let placeholder: SharedString = placeholder.into();
        let input = self.message_input.clone();
        let window_handle = self.window_handle;
        let _ = cx.update_window(window_handle, move |_, window, cx| {
            input.update(cx, |state, cx| {
                state.set_placeholder(placeholder, window, cx)
            });
        });
    }

    fn clear_message_input(&self, cx: &mut Context<Self>) {
        let input = self.message_input.clone();
        let window_handle = self.window_handle;
        let _ = cx.update_window(window_handle, move |_, window, cx| {
            input.update(cx, |state, cx| state.set_value("", window, cx));
        });
    }

    fn show_error_dialog(
        &self,
        title: impl Into<SharedString>,
        message: impl Into<SharedString>,
        cx: &mut Context<Self>,
    ) {
        let title: SharedString = title.into();
        let message: SharedString = message.into();

        let _ = cx.update_window(self.window_handle, move |_, window, cx| {
            use gpui_component::WindowExt as _;

            window.open_dialog(cx, {
                let title = title.clone();
                let message = message.clone();
                move |dialog, _window, _cx| {
                    dialog
                        .title(title.clone())
                        .child(div().text_sm().child(message.clone()))
                        .alert()
                }
            });
        });
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
                self.show_error_dialog("Connection failed", format!("{err}"), cx);
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
                let status_for_dialog = status.clone();

                let _ = this.update(&mut cx, |view, cx| {
                    view.clickup_loading = false;
                    view.clickup_status = status.into();
                    view.user = user.clone();
                    if user.is_some() {
                        view.fetch_channels(cx);
                    } else {
                        view.show_error_dialog("Connection failed", status_for_dialog, cx);
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
            let msg = "Missing CLICKUP_WORKSPACE_ID (or CLICKUP_TEAM_ID) in .env";
            self.clickup_status = msg.into();
            self.show_error_dialog("Configuration error", msg, cx);
            cx.notify();
            return;
        };

        let api = match ClickUpApi::from_env() {
            Ok(api) => api,
            Err(err) => {
                self.clickup_status = format!("{err}").into();
                self.show_error_dialog("Configuration error", format!("{err}"), cx);
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
                            Err(err) => {
                                let msg = format!("Error: {err}");
                                view.clickup_status = msg.clone().into();
                                view.show_error_dialog("Failed to load chats", msg, cx);
                            }
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
        self.server_messages.clear();
        self.pending_messages.clear();
        self.pending_ids.clear();
        self.set_message_input_placeholder(
            format!(
                "Message {}{}",
                channel.icon_prefix(),
                channel.display_name()
            ),
            cx,
        );
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
                        if let Ok(mut new_messages) = result {
                            new_messages.reverse();

                            let confirmed_ids: HashSet<String> =
                                new_messages.iter().map(|m| m.id.clone()).collect();
                            view.pending_messages
                                .retain(|p| !confirmed_ids.contains(&p.id));
                            view.pending_ids.retain(|id| !confirmed_ids.contains(id));

                            view.server_messages = new_messages;
                            cx.notify();
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
            self.show_error_dialog(
                "Configuration error",
                "Missing CLICKUP_WORKSPACE_ID (or CLICKUP_TEAM_ID) in .env",
                cx,
            );
            return;
        };

        let api = match ClickUpApi::from_env() {
            Ok(api) => api,
            Err(err) => {
                self.show_error_dialog("Configuration error", format!("{err}"), cx);
                return;
            }
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
                        match result {
                            Ok(mut messages) => {
                                messages.reverse();
                                view.server_messages = messages;
                                view.scroll_to_bottom();
                            }
                            Err(err) => {
                                view.show_error_dialog(
                                    "Failed to load messages",
                                    format!("{err}"),
                                    cx,
                                );
                            }
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
        let content = self.message_input.read(cx).unmask_value().to_string();
        let content = content.trim().to_string();

        if content.is_empty() {
            self.clear_message_input(cx);
            return;
        }

        let Some(workspace_id) = self.team_id else {
            return;
        };

        let Some(ref channel) = self.selected_channel else {
            return;
        };

        let channel_id = channel.id.clone();

        let api = match ClickUpApi::from_env() {
            Ok(api) => api,
            Err(err) => {
                self.show_error_dialog("Configuration error", format!("{err}"), cx);
                return;
            }
        };

        let (user_id, username) = self
            .user
            .as_ref()
            .map(|u| (u.id.to_string(), u.username.clone()))
            .unwrap_or_else(|| ("0".to_string(), "You".to_string()));

        // Clone for use in the async block to enrich the confirmed message
        let user_id_for_enrichment = user_id.clone();
        let username_for_enrichment = username.clone();

        let temp_id = format!(
            "pending_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0)
        );

        let pending_message =
            ChatMessage::new_pending(temp_id.clone(), content.clone(), user_id, username);

        self.pending_messages.push(pending_message);
        self.pending_ids.insert(temp_id.clone());
        self.scroll_to_bottom();
        self.clear_message_input(cx);
        cx.notify();

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
                        match result {
                            Ok(mut confirmed) => {
                                // Enrich the confirmed message with the current user's info
                                // (API response only has user_id, not the full creator object)
                                if confirmed.creator.is_none() {
                                    confirmed.creator = Some(crate::api::MessageCreator {
                                        id: user_id_for_enrichment.clone(),
                                        username: Some(username_for_enrichment.clone()),
                                        email: None,
                                        profile_picture: None,
                                    });
                                }

                                // Remove pending and add confirmed immediately
                                view.pending_ids.remove(&temp_id);
                                view.pending_messages.retain(|m| m.id != temp_id);

                                // Check if this message is already in server_messages
                                // (could happen if a refresh beat us to it)
                                if !view.server_messages.iter().any(|m| m.id == confirmed.id) {
                                    view.server_messages.push(confirmed);
                                }

                                view.scroll_to_bottom();
                            }
                            Err(ref err) => {
                                // Remove the pending message on error
                                view.pending_ids.remove(&temp_id);
                                view.pending_messages.retain(|m| m.id != temp_id);
                                view.show_error_dialog(
                                    "Failed to send message",
                                    format!("{err}"),
                                    cx,
                                );
                            }
                        }
                        cx.notify();
                    });
                }
            },
        )
        .detach();
    }
}

impl gpui::Render for ClickLiteApp {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("root")
            .size_full()
            .flex()
            .flex_row()
            .bg(cx.theme().background)
            .text_color(cx.theme().foreground)
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
    }
}
