//! Error types for Igris Inertial SDK.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum IgrisError {
    #[error("Authentication failed: {message}")]
    Authentication { message: String, status_code: u16 },

    #[error("Rate limit exceeded: {message}")]
    RateLimit { message: String },

    #[error("Validation error: {message}")]
    Validation { message: String, status_code: u16 },

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("API error ({status_code}): {message}")]
    Api { message: String, status_code: u16 },

    #[error("Deserialization error: {0}")]
    Deserialization(#[from] serde_json::Error),
}
