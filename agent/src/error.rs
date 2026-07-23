use thiserror::Error;
use std::io;

#[derive(Error, Debug)]
pub enum AgentError {
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("IO error: {0}")]
    IoError(#[from] io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    #[error("Mesh network error: {0}")]
    MeshError(String),

    #[error("State management error: {0}")]
    StateError(String),

    #[error("Command execution failed: {0}")]
    CommandExecutionError(String),

    #[error("System metrics collection failed: {0}")]
    MetricsError(String),
}

pub type Result<T> = std::result::Result<T, AgentError>;