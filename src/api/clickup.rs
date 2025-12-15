use crate::error::AppError;
use reqwest::blocking::Client;
use reqwest::header::AUTHORIZATION;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct ClickUpApi {
    pub base_url: String,
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
    pub avatar_data: Option<Arc<Vec<u8>>>,
}

#[derive(Debug, Deserialize)]
struct GetUserResponse {
    user: ClickUpUser,
}

impl ClickUpApi {
    pub fn new(base_url: impl Into<String>, token: impl Into<String>) -> Result<Self, AppError> {
        Ok(Self {
            base_url: base_url.into(),
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
        Self::new("https://api.clickup.com/api/v2", token)
    }

    pub fn get_current_user(&self) -> Result<ClickUpUser, AppError> {
        let base_url = self.base_url.trim_end_matches('/');
        let url = format!("{base_url}/user");

        let response = self
            .client
            .get(url)
            .header(AUTHORIZATION, self.token.clone())
            .send()?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().unwrap_or_default();
            return Err(AppError::Api(format!("ClickUp returned {status}: {body}")));
        }

        let body: GetUserResponse = response
            .json()
            .map_err(|err: reqwest::Error| AppError::Parse(err.to_string()))?;

        let mut user = body.user;
        user.avatar_data = self.get_user_avatar(&user)?;

        Ok(user)
    }

    fn get_user_avatar(&self, user: &ClickUpUser) -> Result<Option<Arc<Vec<u8>>>, AppError> {
        if let Some(ref avatar_url) = user.profile_picture_url {
            let response = self.client.get(avatar_url).send()?;

            if response.status().is_success() {
                let bytes = response.bytes()?;
                return Ok(Some(Arc::new(bytes.to_vec())));
            }
        }

        Ok(None)
    }
}
