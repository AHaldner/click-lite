use crate::api::client::{ClickUpApi, ensure_success, parse_json_ok};
use crate::error::AppError;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize)]
pub struct ClickUpChatChannel {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub channel_type: String,
    #[serde(default)]
    pub visibility: Option<String>,
    #[serde(default)]
    pub latest_comment_at: Option<u64>,
}

impl ClickUpChatChannel {
    pub fn display_name(&self) -> String {
        if let Some(ref name) = self.name {
            if !name.is_empty() {
                return name.clone();
            }
        }

        match self.channel_type.as_str() {
            "DM" => "Direct Message".to_string(),
            "CHANNEL" => "Channel".to_string(),
            _ => self.channel_type.clone(),
        }
    }

    pub fn icon_prefix(&self) -> &'static str {
        match self.channel_type.as_str() {
            "DM" => "@",
            "CHANNEL" => "#",
            _ => "•",
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct ChannelMember {
    pub id: String,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub plain_text: Option<String>,
    #[serde(default)]
    pub user_id: Option<String>,
    #[serde(default)]
    pub date: Option<u64>,
    #[serde(default)]
    pub date_updated: Option<u64>,
    #[serde(default)]
    pub creator: Option<MessageCreator>,
    #[serde(default)]
    pub date_created: Option<String>,
}

impl ChatMessage {
    pub fn display_content(&self) -> String {
        self.plain_text
            .clone()
            .or_else(|| self.content.clone())
            .map(|s| {
                if s.is_empty() {
                    "[Empty message]".to_string()
                } else {
                    s
                }
            })
            .unwrap_or_else(|| "[No content]".to_string())
    }

    pub fn creator_name(&self) -> String {
        self.creator
            .as_ref()
            .and_then(|c| c.username.clone())
            .unwrap_or_else(|| "User".to_string())
    }

    pub fn creator_id(&self) -> String {
        self.user_id
            .clone()
            .or_else(|| self.creator.as_ref().map(|c| c.id.clone()))
            .unwrap_or_else(|| "0".to_string())
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct MessageCreator {
    pub id: String,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(rename = "profilePicture", default)]
    pub profile_picture: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GetChatChannelsResponse {
    #[serde(default)]
    data: Vec<ClickUpChatChannel>,
}

#[derive(Debug, Deserialize)]
struct GetChannelMembersResponse {
    #[serde(default)]
    data: Vec<ChannelMember>,
}

#[derive(Debug, Deserialize)]
struct GetMessagesResponse {
    #[serde(default)]
    data: Vec<ChatMessage>,
}

#[derive(Debug, Serialize)]
struct SendMessageRequest {
    content: String,
}

#[derive(Debug, Deserialize)]
struct SendMessageResponse {
    data: ChatMessage,
}

impl ClickUpApi {
    pub fn get_chat_channels(
        &self,
        workspace_id: u64,
        current_user_id: Option<u64>,
    ) -> Result<Vec<ClickUpChatChannel>, AppError> {
        let url = format!(
            "{}/workspaces/{workspace_id}/chat/channels?limit=10&is_follower=true&include_closed=false",
            self.base_v3_url
        );
        let response = self.request_get(url)?.send()?;
        let body: GetChatChannelsResponse = parse_json_ok(response)?;

        let mut channels = body.data;
        for channel in &mut channels {
            if channel.channel_type == "DM" && channel.name.is_none() {
                if let Ok(members) = self.get_channel_members(workspace_id, &channel.id) {
                    let other_members: Vec<_> = members
                        .iter()
                        .filter(|m| {
                            if let Some(current_id) = current_user_id {
                                m.id.parse::<u64>().ok() != Some(current_id)
                            } else {
                                true
                            }
                        })
                        .filter_map(|m| m.username.clone())
                        .collect();

                    if !other_members.is_empty() {
                        channel.name = Some(other_members.join(", "));
                    }
                }
            }
        }

        Ok(channels)
    }

    pub fn get_channel_members(
        &self,
        workspace_id: u64,
        channel_id: &str,
    ) -> Result<Vec<ChannelMember>, AppError> {
        let url = format!(
            "{}/workspaces/{workspace_id}/chat/channels/{channel_id}/members",
            self.base_v3_url
        );
        let response = self.request_get(url)?.send()?;
        let body: GetChannelMembersResponse = parse_json_ok(response)?;
        Ok(body.data)
    }

    pub fn get_channel_messages(
        &self,
        workspace_id: u64,
        channel_id: &str,
    ) -> Result<Vec<ChatMessage>, AppError> {
        let url = format!(
            "{}/workspaces/{workspace_id}/chat/channels/{channel_id}/messages?limit=50",
            self.base_v3_url
        );
        let response = self.request_get(url)?.send()?;
        let response = ensure_success(response)?;
        let text = response
            .text()
            .map_err(|e| AppError::Parse(e.to_string()))?;

        eprintln!("API Response: {}", text);

        if let Ok(body) = serde_json::from_str::<GetMessagesResponse>(&text) {
            return Ok(body.data);
        }

        if let Ok(messages) = serde_json::from_str::<Vec<ChatMessage>>(&text) {
            return Ok(messages);
        }

        Err(AppError::Parse(
            "Failed to parse messages response".to_string(),
        ))
    }

    pub fn send_message(
        &self,
        workspace_id: u64,
        channel_id: &str,
        content: &str,
    ) -> Result<ChatMessage, AppError> {
        let url = format!(
            "{}/workspaces/{workspace_id}/chat/channels/{channel_id}/messages",
            self.base_v3_url
        );
        let body = SendMessageRequest {
            content: content.to_string(),
        };
        let response = self.request_post(url, &body)?.send()?;
        let body: SendMessageResponse = parse_json_ok(response)?;
        Ok(body.data)
    }
}
