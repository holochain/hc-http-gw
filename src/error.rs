//! hc-http-gw error types

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Serialize;

use crate::app_selection::AppSelectionError;

/// Core HTTP Gateway error type
#[derive(thiserror::Error, Debug)]
pub enum HcHttpGatewayError {
    /// Request malformed error. This includes all request validation errors such as
    /// invalid request path, excess identifier length, invalid DNA hash and invalid
    /// encodings.
    #[error("Request is malformed: {0}")]
    RequestMalformed(String),
    /// Calling an unauthorized function
    #[error("Function {fn_name} in zome {zome_name} in app {app_id} is not authorized")]
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
            HcHttpGatewayError::RequestMalformed(e) => (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::from(format!("Request is malformed: {e}"))),
            ),
            HcHttpGatewayError::UnauthorizedFunction {
                app_id,
                zome_name,
                fn_name,
            } => (
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::from(format!(
                    "Function {fn_name} in zome {zome_name} in app {app_id} is not allowed"
                ))),
            ),
            HcHttpGatewayError::UpstreamUnavailable => (
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse::from("Could not connect to Holochain")),
            ),
            HcHttpGatewayError::AppSelectionError(AppSelectionError::NotInstalled) => (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::from(self.to_string())),
            ),
            HcHttpGatewayError::AppSelectionError(AppSelectionError::NotAllowed) => (
                StatusCode::FORBIDDEN,
                Json(ErrorResponse::from(self.to_string())),
            ),
            HcHttpGatewayError::AppSelectionError(AppSelectionError::MultipleMatching) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::from(self.to_string())),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::from("Something went wrong")),
            ),
        }
        .into_response()
    }
}
