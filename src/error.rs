//! hc-http-gw error types

/// Core HTTP Gateway error type
#[derive(thiserror::Error, Debug)]
pub enum HcHttpGatewayError {
    /// Handles system-level I/O errors
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    /// Handles configuration parsing errors
    #[error("{0}")]
    ConfigurationError(String),
}

/// Type aliased Result
pub type HcHttpGatewayResult<T> = Result<T, HcHttpGatewayError>;
