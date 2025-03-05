//! hc-http-gw error types

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;

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
}

/// Type aliased Result
pub type HcHttpGatewayResult<T> = Result<T, HcHttpGatewayError>;

impl IntoResponse for HcHttpGatewayError {
    fn into_response(self) -> axum::response::Response {
        match self {
            e => {
                tracing::error!("Internal Error: {}", e);
                error_from_status(500, None)
            }
        }
    }
}

/// Construct an axum http error from a status code and optional message
pub fn error_from_status(status_code: u16, message: Option<&str>) -> axum::response::Response {
    let error_response = HcGwErrorResponse {
        error: message.unwrap_or("Something Went Wrong").to_string(),
    };

    (
        StatusCode::from_u16(status_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR),
        Json(error_response),
    )
        .into_response()
}
