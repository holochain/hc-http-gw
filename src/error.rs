//! hc-http-gw error types

use crate::app_selection::AppSelectionError;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use holochain_client::ConductorApiError;
use holochain_conductor_api::ExternalApiWireError;
use serde::{Deserialize, Serialize};

/// Core HTTP Gateway error type
#[derive(thiserror::Error, Debug)]
pub enum HcHttpGatewayError {
    /// Request malformed error. This includes all request validation errors such as
    /// invalid request path, excess identifier length, invalid DNA hash and invalid
    /// encodings.
    #[error("Request is malformed: {0}")]
    RequestMalformed(String),
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
    /// Holochain errors
    #[error("Holochain error: {0}")]
    HolochainError(#[from] holochain_client::ConductorApiError),
    /// Error returned when a connection cannot be made to the upstream Holochain service
    #[error("The upstream Holochain service could not be reached")]
    UpstreamUnavailable,
    /// Handle errors specific to app selection
    #[error("Error selecting a valid app: {0}")]
    AppSelectionError(#[from] AppSelectionError),
}

/// Gateway result type.
pub type HcHttpGatewayResult<T> = Result<T, HcHttpGatewayError>;

/// Error format returned to the caller.
#[derive(Debug, Deserialize, Serialize)]
pub struct ErrorResponse {
    /// The error message
    pub error: String,
}

impl From<String> for ErrorResponse {
    fn from(value: String) -> Self {
        Self { error: value }
    }
}

impl HcHttpGatewayError {
    /// Convert error into HTTP status code and error message.
    pub fn into_status_code_and_body(self) -> (StatusCode, String) {
        match self {
            HcHttpGatewayError::RequestMalformed(_) => (StatusCode::BAD_REQUEST, self.to_string()),
            HcHttpGatewayError::UnauthorizedFunction { .. } => {
                (StatusCode::FORBIDDEN, self.to_string())
            }
            HcHttpGatewayError::UpstreamUnavailable => (
                StatusCode::BAD_GATEWAY,
                "Could not connect to Holochain".to_string(),
            ),
            HcHttpGatewayError::AppSelectionError(AppSelectionError::NotInstalled) => {
                (StatusCode::NOT_FOUND, self.to_string())
            }
            HcHttpGatewayError::AppSelectionError(AppSelectionError::NotAllowed) => {
                (StatusCode::FORBIDDEN, self.to_string())
            }
            HcHttpGatewayError::AppSelectionError(AppSelectionError::MultipleMatching) => {
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
            HcHttpGatewayError::HolochainError(ConductorApiError::ExternalApiWireError(
                ExternalApiWireError::RibosomeError(e),
            )) => (StatusCode::INTERNAL_SERVER_ERROR, e),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Something went wrong".to_string(),
            ),
        }
    }
}

impl IntoResponse for HcHttpGatewayError {
    fn into_response(self) -> axum::response::Response {
        let (status_code, body) = self.into_status_code_and_body();
        (status_code, Json(ErrorResponse::from(body))).into_response()
    }
}
