use crate::error::AppError;
use gpui::{Image, ImageFormat};
use reqwest::blocking::{Client, Response};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct ClickUpApi {
    base_v2_url: String,
    base_v3_url: String,
    token: String,
    client: Client,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ClickUpUser {
    pub id: u64,
    pub username: String,
    pub email: String,
    #[serde(rename = "profilePicture")]
    pub profile_picture_url: Option<String>,
    #[serde(skip)]
    pub avatar_image: Option<Arc<Image>>,
}

#[derive(Debug, Deserialize)]
struct GetUserResponse {
    user: ClickUpUser,
}

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

#[derive(Debug, Deserialize)]
struct GetChatChannelsResponse {
    #[serde(default)]
    data: Vec<ClickUpChatChannel>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ChannelMember {
    pub id: String,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
}

#[derive(Debug, Deserialize)]
struct GetChannelMembersResponse {
    #[serde(default)]
    data: Vec<ChannelMember>,
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

#[derive(Debug, Deserialize)]
struct GetTeamResponse {
    team: GetTeamInner,
}

#[derive(Debug, Deserialize)]
struct GetTeamInner {
    #[serde(default)]
    members: Vec<GetTeamMember>,
}

#[derive(Debug, Deserialize)]
struct GetTeamMember {
    user: ClickUpUser,
}

impl ClickUpApi {
    pub fn new(token: impl Into<String>) -> Result<Self, AppError> {
        Ok(Self {
            base_v2_url: "https://api.clickup.com/api/v2".to_string(),
            base_v3_url: "https://api.clickup.com/api/v3".to_string(),
            token: token.into(),
            client: Client::new(),
        })
    }

    pub fn from_env() -> Result<Self, AppError> {
        let token = std::env::var("CLICKUP_ACCESS_TOKEN")
            .or_else(|_| std::env::var("CLICKUP_TOKEN"))
            .map_err(|_| {
                AppError::Config(
                    "missing CLICKUP_ACCESS_TOKEN (or CLICKUP_TOKEN) in environment".to_string(),
                )
            })?;
        Self::new(token)
    }

    pub fn get_current_user(&self) -> Result<ClickUpUser, AppError> {
        let url = format!("{}/user", self.base_v2_url);
        let response = self.request_get(url)?.send()?;
        let body: GetUserResponse = parse_json_ok(response)?;

        let mut user = body.user;
        user.avatar_image = self.get_user_avatar(&user)?;

        Ok(user)
    }

    fn get_user_avatar(&self, user: &ClickUpUser) -> Result<Option<Arc<Image>>, AppError> {
        if let Some(ref avatar_url) = user.profile_picture_url {
            let response = self.client.get(avatar_url).send()?;
            let response = ensure_success(response)?;
            let bytes = response.bytes()?;
            let image = Image::from_bytes(ImageFormat::Jpeg, bytes.to_vec());
            return Ok(Some(Arc::new(image)));
        }

        Ok(None)
    }

    pub fn get_team_members(&self, workspace_id: u64) -> Result<Vec<ClickUpUser>, AppError> {
        let url = format!("{}/team/{workspace_id}", self.base_v2_url);
        let response = self.request_get(url)?.send()?;
        let body: GetTeamResponse = parse_json_ok(response)?;

        Ok(body.team.members.into_iter().map(|m| m.user).collect())
    }

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

        if let Ok(body) = serde_json::from_str::<GetMessagesResponse>(&text) {
            return Ok(body.data);
        }

        if let Ok(messages) = serde_json::from_str::<Vec<ChatMessage>>(&text) {
            return Ok(messages);
        }

        eprintln!(
            "Failed to parse messages response: {}",
            &text[..text.len().min(500)]
        );
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

    fn request_get(&self, url: String) -> Result<reqwest::blocking::RequestBuilder, AppError> {
        Ok(self
            .client
            .get(url)
            .header(AUTHORIZATION, self.token.clone()))
    }

    fn request_post<T: Serialize>(
        &self,
        url: String,
        body: &T,
    ) -> Result<reqwest::blocking::RequestBuilder, AppError> {
        Ok(self
            .client
            .post(url)
            .header(AUTHORIZATION, self.token.clone())
            .header(CONTENT_TYPE, "application/json")
            .json(body))
    }
}

fn ensure_success(response: Response) -> Result<Response, AppError> {
    let status = response.status();
    if status.is_success() {
        Ok(response)
    } else {
        let body = response.text().unwrap_or_default();
        Err(AppError::Api(format!("ClickUp returned {status}: {body}")))
    }
}

fn parse_json_ok<T: for<'de> Deserialize<'de>>(response: Response) -> Result<T, AppError> {
    let response = ensure_success(response)?;
    response
        .json()
        .map_err(|err: reqwest::Error| AppError::Parse(err.to_string()))
}
