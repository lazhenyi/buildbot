//! Error types for Buildbot

use thiserror::Error;

/// Main error type for Buildbot
#[derive(Error, Debug)]
pub enum BuildbotError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Worker error: {0}")]
    Worker(String),

    #[error("Builder error: {0}")]
    Builder(String),

    #[error("Builder busy: {0}")]
    BuilderBusy(String),

    #[error("Scheduler error: {0}")]
    Scheduler(String),

    #[error("Protocol error: {0}")]
    Protocol(String),

    #[error("Channel error: {0}")]
    Channel(String),

    #[error("Build request error: {0}")]
    BuildRequest(String),

    #[error("Build error: {0}")]
    Build(String),

    #[error("Property error: {0}")]
    Property(String),

    #[error("Lock error: {0}")]
    Lock(String),

    #[error("Change source error: {0}")]
    ChangeSource(String),

    #[error("Master error: {0}")]
    Master(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    #[error("Base64 decode error: {0}")]
    Base64(#[from] base64::DecodeError),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Already exists: {0}")]
    AlreadyExists(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<sea_orm::DbErr> for BuildbotError {
    fn from(e: sea_orm::DbErr) -> Self {
        BuildbotError::Database(e.to_string())
    }
}

impl From<sqlx::Error> for BuildbotError {
    fn from(e: sqlx::Error) -> Self {
        BuildbotError::Database(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, BuildbotError>;
