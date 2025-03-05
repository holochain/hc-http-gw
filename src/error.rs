//! hc-http-gw error types

/// Core HTTP Gateway error type
#[derive(thiserror::Error, Debug)]
pub enum HcHttpGatewayError {
    /// Handles system-level I/O errors
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    /// Handles configuration parsing errors
    #[error("Configuration error: {0}")]
    ConfigurationError(String),
    /// Handles conductor api errors
    #[error("Conductor API error: {0:?}")]
    ConductorApiError(holochain_client::ConductorApiError),
    /// Handles other errors
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl From<holochain_client::ConductorApiError> for HcHttpGatewayError {
    fn from(value: holochain_client::ConductorApiError) -> Self {
        HcHttpGatewayError::ConductorApiError(value)
    }
}

/// Type aliased Result
pub type HcHttpGatewayResult<T> = Result<T, HcHttpGatewayError>;
