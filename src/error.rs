//! hc-http-gw error types

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use holochain_types::{dna::HoloHashError, prelude::SerializedBytesError};
use serde::Serialize;

/// Core HTTP Gateway error type
#[derive(thiserror::Error, Debug)]
pub enum HcHttpGatewayError {
    /// System-level I/O errors
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    /// Configuration parsing errors
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    /// URI path errors
    #[error("Path error: {0}")]
    PathError(#[from] axum::extract::rejection::PathRejection),
    /// Identifier length exceeded
    #[error("Identifier length exceeded: {0} has more than {1} characters")]
    IdentifierLengthExceeded(String, u8),
    /// Base64 decode errors
    #[error("Base64 decode error: {0}")]
    Base64DecodeError(#[from] base64::DecodeError),
    /// Holo hash errors
    #[error("HoloHash error: {0}")]
    DnaHashError(#[from] HoloHashError),
    /// Errors deserializing zome call payload to JSON
    #[error("Failed to deserialize JSON to serde_json::Value: {0}")]
    InvalidJSON(#[from] serde_json::Error),
    /// Payload size exceeded
    #[error("Payload size ({size} bytes) exceeds maximum allowed size ({limit} bytes)")]
    PayloadSizeLimitError {
        /// Current size of payload
        size: u32,
        /// Allowed payload size limit
        limit: u32,
    },
    /// ExternIO encoding error
    #[error("Failed to serialize payload to ExternIO: {0}")]
    PayloadSerializationError(#[from] SerializedBytesError),
    /// Calling an unauthorized function
    #[error("Function {fn_name} in zome {zome_name} in app {app_id} is not allowed")]
    UnauthorizedFunction {
        /// App id
        app_id: String,
        /// Zome name
        zome_name: String,
        /// Function name
        fn_name: String,
    },
    /// Handles Holochain errors
    #[error("Holochain error: {0}")]
    HolochainError(#[from] holochain_client::ConductorApiError),
    /// Error returned when a connection cannot be made to the upstream Holochain service
    #[error("The upstream Holochain service could not be reached")]
    UpstreamUnavailable,
}

/// Type aliased Result
pub type HcHttpGatewayResult<T> = Result<T, HcHttpGatewayError>;

/// Error format returned to the caller.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

impl From<String> for ErrorResponse {
    fn from(value: String) -> Self {
        Self { error: value }
    }
}

impl From<&str> for ErrorResponse {
    fn from(value: &str) -> Self {
        Self {
            error: value.to_owned(),
        }
    }
}

impl IntoResponse for HcHttpGatewayError {
    fn into_response(self) -> axum::response::Response {
        match self {
            HcHttpGatewayError::PathError(e) => {
                tracing::error!("Path error: {}", e);
                (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse::from("Invalid request path")),
                )
            }
            HcHttpGatewayError::IdentifierLengthExceeded(identifier, max_length) => {
                let message =
                    format!("Identifier {identifier} longer than {max_length} characters");
                tracing::error!(message);
                (StatusCode::BAD_REQUEST, Json(ErrorResponse::from(message)))
            }
            HcHttpGatewayError::Base64DecodeError(e) => {
                tracing::error!("Base64 decode error: {}", e);
                (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse::from(
                        "Failed to decode base64 encoded string",
                    )),
                )
            }
            HcHttpGatewayError::DnaHashError(e) => {
                tracing::error!("DnaHash error: {}", e);
                (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse::from("Invalid base64 DNA hash")),
                )
            }
            HcHttpGatewayError::InvalidJSON(e) => {
                tracing::error!("Invalid JSON: {}", e);
                (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse::from("Payload contains invalid JSON")),
                )
            }
            HcHttpGatewayError::PayloadSizeLimitError { size, limit } => {
                let message = format!(
                    "Payload size ({size} bytes) exceeds maximum allowed size ({limit} bytes)"
                );
                tracing::error!(message);
                (StatusCode::BAD_REQUEST, Json(ErrorResponse::from(message)))
            }
            HcHttpGatewayError::PayloadSerializationError(e) => {
                tracing::error!("Failed to serialize payload to ExternIO: {e}");
                (
                    StatusCode::BAD_REQUEST,
                    Json(ErrorResponse::from(
                        "Failed to serialize payload to ExternIO",
                    )),
                )
            }
            HcHttpGatewayError::UnauthorizedFunction {
                app_id,
                zome_name,
                fn_name,
            } => {
                let message = format!(
                    "Function {fn_name} in zome {zome_name} in app {app_id} is not allowed"
                );
                tracing::error!(message);
                (StatusCode::FORBIDDEN, Json(ErrorResponse::from(message)))
            }
            HcHttpGatewayError::UpstreamUnavailable => (
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse::from("Could not connect to Holochain")),
            ),
            e => {
                tracing::error!("Internal Error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ErrorResponse::from("Something went wrong")),
                )
            }
        }
        .into_response()
    }
}
