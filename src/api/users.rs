use crate::api::client::{ClickUpApi, parse_json_ok};
use crate::error::AppError;
use gpui::{Image, ImageFormat};
use serde::Deserialize;
use std::sync::Arc;

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
            let response = crate::api::client::ensure_success(response)?;
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

        Ok(body
            .team
            .members
            .into_iter()
            .map(|member| member.user)
            .collect())
    }
}
