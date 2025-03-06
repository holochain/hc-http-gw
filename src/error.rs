//! hc-http-gw error types

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use holochain_types::dna::HoloHashError;

use crate::http::HcGwErrorResponse;

/// Core HTTP Gateway error type
#[derive(thiserror::Error, Debug)]
pub enum HcHttpGatewayError {
    /// Handles system-level I/O errors
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    /// Handles configuration parsing errors
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    /// Handle path parsing errors
    #[error("Path parsing error: {0}")]
    PathParsingError(#[from] axum::extract::rejection::PathRejection),
    /// Handle base64 decode errors
    #[error("Base64 decode error: {0}")]
    Base64DecodeError(#[from] base64::DecodeError),
    /// Handle holo hash errors
    #[error("HoloHash error: {0}")]
    HoloHashError(#[from] HoloHashError),
    /// Handle errors deserializing zome call payload to json
    #[error("Failed to deserialize JSON to serde_json::Value: {0}")]
    InvalidJSON(#[from] serde_json::Error),
    /// Handle invalid payload size errors
    #[error("Payload size ({size} bytes) exceeds maximum allowed size ({limit} bytes)")]
    PayloadSizeLimitError {
        /// Current size of payload
        size: u32,
        /// Allowed payload size limit
        limit: u32,
    },
}

/// Type aliased Result
pub type HcHttpGatewayResult<T> = Result<T, HcHttpGatewayError>;

impl IntoResponse for HcHttpGatewayError {
    fn into_response(self) -> axum::response::Response {
        match self {
            HcHttpGatewayError::PathParsingError(e) => {
                tracing::error!("Path deserialization error: {}", e);
                error_response(400, Some("Invalid request path"))
            }
            HcHttpGatewayError::Base64DecodeError(e) => {
                tracing::error!("Base64 decode error: {}", e);
                error_response(400, Some("Failed to decode base64 encoded string"))
            }
            HcHttpGatewayError::HoloHashError(e) => {
                tracing::error!("HoloHash error: {}", e);
                error_response(400, Some("Invalid base64 DNA hash"))
            }
            HcHttpGatewayError::InvalidJSON(e) => {
                tracing::error!("Invalid JSON: {}", e);
                error_response(400, Some("Payload contains invalid JSON"))
            }
            HcHttpGatewayError::PayloadSizeLimitError { size, limit } => {
                tracing::error!(
                    "Payload size ({} bytes) exceeds maximum allowed size ({} bytes)",
                    size,
                    limit
                );
                error_response(
                    400,
                    Some(&format!(
                        "Payload size exceeds maximum allowed size ({limit} bytes)"
                    )),
                )
            }
            e => {
                tracing::error!("Internal Error: {}", e);
                error_response(500, None)
            }
        }
    }
}

/// Construct an axum http error from a status code and optional message
pub fn error_response(status_code: u16, message: Option<&str>) -> axum::response::Response {
    let error_response = HcGwErrorResponse {
        error: message.unwrap_or("Something Went Wrong").to_string(),
    };

    (
        StatusCode::from_u16(status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
        Json(error_response),
    )
        .into_response()
}
