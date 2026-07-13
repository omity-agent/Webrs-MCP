use thiserror::Error;
#[derive(Clone, Debug, Error)]
#[expect(
    clippy::exhaustive_enums,
    reason = "Application error variants are closed and keep the AppError API explicit."
)]
pub enum AppError {
    #[error("{0}")]
    Client(String),
    #[error("configuration error: {0}")]
    Config(String),
    #[error("{0}")]
    Upstream(String),
    #[error("internal server error: {0}")]
    Internal(String),
}
impl AppError {
    #[inline]
    #[must_use]
    pub fn client<Message>(message: Message) -> Self
    where
        Message: Into<String>,
    {
        Self::Client(message.into())
    }
    #[inline]
    #[must_use]
    pub fn config<Message>(message: Message) -> Self
    where
        Message: Into<String>,
    {
        Self::Config(message.into())
    }
    #[inline]
    #[must_use]
    pub fn internal<Message>(message: Message) -> Self
    where
        Message: Into<String>,
    {
        Self::Internal(message.into())
    }
    #[inline]
    #[must_use]
    #[expect(
        clippy::pattern_type_mismatch,
        reason = "Matching borrowed variants keeps client messages available without moving self."
    )]
    pub fn client_message(&self) -> String {
        match self {
            Self::Client(message) | Self::Upstream(message) | Self::Config(message) => {
                message.clone()
            }
            Self::Internal(_) => {
                "Unexpected server error. Retry the request or contact the service operator."
                    .to_owned()
            }
        }
    }
}
impl From<reqwest::Error> for AppError {
    #[inline]
    fn from(error: reqwest::Error) -> Self {
        if error.is_timeout() {
            return Self::Upstream("upstream request timed out. Retry later.".to_owned());
        }
        Self::Upstream(
            "Could not reach upstream service. Check network connectivity and retry.".to_owned(),
        )
    }
}
#[inline]
#[must_use]
pub fn http_service_error(service: &str, status_code: u16) -> AppError {
    if matches!(status_code, 401 | 403) {
        return AppError::client(format!(
            "{service} request was rejected. Check the API key header and permissions."
        ));
    }
    if status_code == 429 {
        return AppError::client(format!(
            "{service} rate limit was reached. Retry later or use another API key."
        ));
    }
    if (400..500).contains(&status_code) {
        return AppError::client(format!(
            "{service} rejected the request with HTTP {status_code}."
        ));
    }
    AppError::client(format!(
        "{service} returned HTTP {status_code}. Retry later or contact the upstream service."
    ))
}
