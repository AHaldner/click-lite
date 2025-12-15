use crate::error::AppError;
use reqwest::blocking::{Client, RequestBuilder, Response};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct ClickUpApi {
    pub(crate) base_v2_url: String,
    pub(crate) base_v3_url: String,
    pub(crate) token: String,
    pub(crate) client: Client,
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

    pub(crate) fn request_get(&self, url: String) -> Result<RequestBuilder, AppError> {
        Ok(self
            .client
            .get(url)
            .header(AUTHORIZATION, self.token.clone()))
    }

    pub(crate) fn request_post<T: Serialize>(
        &self,
        url: String,
        body: &T,
    ) -> Result<RequestBuilder, AppError> {
        Ok(self
            .client
            .post(url)
            .header(AUTHORIZATION, self.token.clone())
            .header(CONTENT_TYPE, "application/json")
            .json(body))
    }
}

pub fn ensure_success(response: Response) -> Result<Response, AppError> {
    let status = response.status();
    if status.is_success() {
        Ok(response)
    } else {
        let body = response.text().unwrap_or_default();
        Err(AppError::Api(format!("ClickUp returned {status}: {body}")))
    }
}

pub fn parse_json_ok<T: for<'de> Deserialize<'de>>(response: Response) -> Result<T, AppError> {
    let response = ensure_success(response)?;
    response
        .json()
        .map_err(|err: reqwest::Error| AppError::Parse(err.to_string()))
}
