use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Network request failed: {0}")]
    Network(#[from] reqwest::Error),

    #[error("API returned an error: {0}")]
    Api(String),

    #[error("Failed to parse response: {0}")]
    Parse(String),

    #[error("IO operation failed: {0}")]
    Io(#[from] std::io::Error),

    #[error("Unknown error: {0}")]
    Unknown(String),
}

impl From<AppError> for String {
    fn from(err: AppError) -> Self {
        err.to_string()
    }
}
