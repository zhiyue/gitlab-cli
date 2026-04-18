use serde::Serialize;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidArgs,
    Unauthorized,
    Forbidden,
    NotFound,
    Conflict,
    BadRequest,
    RateLimited,
    ServerError,
    Network,
    Timeout,
    Unknown,
}

impl ErrorCode {
    #[must_use]
    #[allow(clippy::match_same_arms)]
    pub fn exit_code(self) -> i32 {
        match self {
            Self::InvalidArgs => 2,
            Self::Unauthorized => 3,
            Self::Forbidden => 4,
            Self::NotFound => 5,
            Self::Conflict => 6,
            Self::BadRequest => 6,
            Self::RateLimited => 7,
            Self::ServerError => 8,
            Self::Network | Self::Timeout => 9,
            Self::Unknown => 1,
        }
    }

    #[must_use]
    pub fn retryable(self) -> bool {
        matches!(
            self,
            Self::RateLimited | Self::ServerError | Self::Network | Self::Timeout
        )
    }
}

#[derive(Debug, Serialize)]
pub struct ErrorPayload {
    pub code: ErrorCode,
    pub status: Option<u16>,
    pub message: String,
    pub request_id: Option<String>,
    pub retryable: bool,
    pub details: serde_json::Value,
}

#[derive(Debug, thiserror::Error)]
pub enum GitlabError {
    #[error("{message}")]
    Http {
        code: ErrorCode,
        status: u16,
        message: String,
        request_id: Option<String>,
        details: serde_json::Value,
    },
    #[error("network: {0}")]
    Network(String),
    #[error("timeout after {0}")]
    Timeout(String),
    #[error("invalid arguments: {0}")]
    InvalidArgs(String),
    #[error("config: {0}")]
    Config(String),
}

impl GitlabError {
    #[must_use]
    pub fn from_status(status: u16, message: String, request_id: Option<String>) -> Self {
        let code = match status {
            400 => ErrorCode::BadRequest,
            401 => ErrorCode::Unauthorized,
            403 => ErrorCode::Forbidden,
            404 => ErrorCode::NotFound,
            409 | 422 => ErrorCode::Conflict,
            429 => ErrorCode::RateLimited,
            500..=599 => ErrorCode::ServerError,
            _ => ErrorCode::Unknown,
        };
        Self::Http {
            code,
            status,
            message,
            request_id,
            details: serde_json::Value::Null,
        }
    }

    #[must_use]
    pub fn network(msg: String) -> Self {
        Self::Network(msg)
    }

    #[must_use]
    #[allow(clippy::match_same_arms)]
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::Http { code, .. } => *code,
            Self::Network(_) => ErrorCode::Network,
            Self::Timeout(_) => ErrorCode::Timeout,
            Self::InvalidArgs(_) => ErrorCode::InvalidArgs,
            Self::Config(_) => ErrorCode::InvalidArgs,
        }
    }

    #[must_use]
    pub fn exit_code(&self) -> i32 {
        self.code().exit_code()
    }

    #[must_use]
    pub fn retryable(&self) -> bool {
        self.code().retryable()
    }

    #[must_use]
    pub fn to_payload(&self) -> ErrorPayload {
        match self {
            Self::Http {
                code,
                status,
                message,
                request_id,
                details,
            } => ErrorPayload {
                code: *code,
                status: Some(*status),
                message: message.clone(),
                request_id: request_id.clone(),
                retryable: code.retryable(),
                details: details.clone(),
            },
            other => ErrorPayload {
                code: other.code(),
                status: None,
                message: other.to_string(),
                request_id: None,
                retryable: other.retryable(),
                details: serde_json::Value::Null,
            },
        }
    }
}

pub type Result<T> = std::result::Result<T, GitlabError>;

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = serde_json::to_value(self)
            .unwrap()
            .as_str()
            .unwrap()
            .to_owned();
        f.write_str(&s)
    }
}
