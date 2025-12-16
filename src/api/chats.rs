use crate::api::client::{ClickUpApi, ensure_success, parse_json_ok};
use crate::error::AppError;
use serde::de::Deserializer;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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

#[derive(Clone, Debug)]
pub struct ChatMessage {
    pub id: String,
    text: Option<String>,
    pub user_id: Option<String>,
    pub date: Option<u64>,
    pub date_updated: Option<u64>,
    pub creator: Option<MessageCreator>,
    pub date_created: Option<String>,
}

impl ChatMessage {
    pub fn display_content(&self) -> String {
        match self.text.as_deref() {
            None => "[No content]".to_string(),
            Some(message) if is_effectively_empty(message) => "[Empty message]".to_string(),
            Some(message) => message.to_string(),
        }
    }

    pub fn creator_name(&self) -> String {
        self.creator
            .as_ref()
            .and_then(|creator| creator.username.clone().or_else(|| creator.email.clone()))
            .unwrap_or_else(|| "Unknown User".to_string())
    }

    pub fn creator_id(&self) -> String {
        self.user_id
            .clone()
            .or_else(|| self.creator.as_ref().map(|creator| creator.id.clone()))
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

#[derive(Clone, Debug, Deserialize)]
struct ChatMessageWire {
    id: String,
    #[serde(default)]
    content: Option<String>,
    #[serde(
        default,
        alias = "creator_id",
        alias = "creatorId",
        alias = "user_id",
        alias = "userId",
        deserialize_with = "deserialize_opt_string_or_number"
    )]
    user_id: Option<String>,
    #[serde(default)]
    date: Option<u64>,
    #[serde(default)]
    date_updated: Option<u64>,
    #[serde(default)]
    creator: Option<MessageCreator>,
    #[serde(default)]
    date_created: Option<String>,
}

impl<'de> Deserialize<'de> for ChatMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = ChatMessageWire::deserialize(deserializer)?;
        Ok(Self {
            id: wire.id,
            text: wire.content,
            user_id: wire.user_id,
            date: wire.date,
            date_updated: wire.date_updated,
            creator: wire.creator,
            date_created: wire.date_created,
        })
    }
}

fn deserialize_opt_string_or_number<'de, Des>(
    deserializer: Des,
) -> Result<Option<String>, Des::Error>
where
    Des: Deserializer<'de>,
{
    let maybe_value = Option::<serde_json::Value>::deserialize(deserializer)?;

    Ok(match maybe_value {
        None => None,
        Some(serde_json::Value::String(text)) => Some(text),
        Some(serde_json::Value::Number(number)) => Some(number.to_string()),
        _ => None,
    })
}

fn is_effectively_empty(input: &str) -> bool {
    input
        .chars()
        .all(|ch| ch.is_whitespace() || ch == '\u{00A0}')
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
                        .filter_map(|member| member.username.clone())
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

        let mut messages = match serde_json::from_str::<GetMessagesResponse>(&text) {
            Ok(body) => body.data,
            Err(_) => match serde_json::from_str::<Vec<ChatMessage>>(&text) {
                Ok(messages) => messages,
                Err(_) => {
                    return Err(AppError::Parse(
                        "Failed to parse messages response".to_string(),
                    ));
                }
            },
        };

        let needs_creator_enrichment =
            messages
                .iter()
                .any(|message| match message.creator.as_ref() {
                    None => true,
                    Some(creator) => creator.username.is_none() && creator.email.is_none(),
                });

        if needs_creator_enrichment {
            if let Ok(members) = self.get_channel_members(workspace_id, channel_id) {
                let members_by_id: HashMap<&str, &ChannelMember> = members
                    .iter()
                    .map(|member| (member.id.as_str(), member))
                    .collect();

                for message in &mut messages {
                    if matches!(
                        message.creator.as_ref(),
                        Some(creator) if creator.username.is_some() || creator.email.is_some()
                    ) {
                        continue;
                    }

                    let creator_id = message.creator_id();
                    if creator_id == "0" {
                        continue;
                    }

                    if let Some(member) = members_by_id.get(creator_id.as_str()) {
                        message.creator = Some(MessageCreator {
                            id: (*member).id.clone(),
                            username: (*member).username.clone(),
                            email: (*member).email.clone(),
                            profile_picture: None,
                        });
                    }
                }
            }
        }

        Ok(messages)
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
