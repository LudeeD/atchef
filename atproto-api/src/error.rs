use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("URL parse error: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("XRPC error: {error} (status {status})")]
    Xrpc {
        status: u16,
        error: String,
        message: Option<String>,
    },

    #[error("Invalid TID: {0}")]
    InvalidTid(String),

    #[error("Invalid AT URI: {0}")]
    InvalidAtUri(String),

    #[error("Invalid DID: {0}")]
    InvalidDid(String),

    #[error("Invalid handle: {0}")]
    InvalidHandle(String),

    #[error("Session error: {0}")]
    Session(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

#[derive(Debug, serde::Deserialize)]
pub struct XrpcErrorResponse {
    pub error: String,
    pub message: Option<String>,
}
