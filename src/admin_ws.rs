use std::{
    future::Future,
    sync::{Arc, Mutex},
};

use holochain_client::{AdminWebsocket, ConductorApiError, ConductorApiResult};

use crate::{HcHttpGatewayError, HcHttpGatewayResult};

const ADMIN_WS_CONNECTION_MAX_RETRIES: usize = 1;

/// A wrapper around AdminWebsocket that automatically handles reconnection
/// when the connection is lost due to network issues or other failures.
#[derive(Debug, Clone)]
pub struct HcHttpGwAdminWebsocket {
    /// The WebSocket URL to connect to
    url: String,
    /// The handle to the AdminWebsocket connection
    connection_handle: Arc<Mutex<Option<AdminWebsocket>>>,
    /// Current retry attempt counter
    current_retries: usize,
}

impl HcHttpGwAdminWebsocket {
    /// Creates a new ReconnectingAdminWebsocket with the specified parameters
    pub fn new(url: &str) -> Self {
        HcHttpGwAdminWebsocket {
            url: url.to_string(),
            connection_handle: Arc::new(Mutex::new(None)),
            current_retries: 0,
        }
    }

    /// Checks if there is an active connection
    fn is_connected(&self) -> HcHttpGatewayResult<bool> {
        let connection = self.connection_handle.lock().unwrap();

        Ok(connection.is_some())
    }

    /// Gets reconnection retry count
    pub fn get_reconnection_retries(&self) -> usize {
        self.current_retries
    }

    /// Ensures that a connection is established before proceeding
    async fn ensure_connected(&mut self) -> HcHttpGatewayResult<()> {
        if self.is_connected()? {
            return Ok(());
        }

        self.connect().await
    }

    /// Attempts to connect to the AdminWebsocket
    ///
    /// This will make multiple attempts according to the `max_retries` and
    /// `retry_delay_ms` settings, with exponential backoff between retries.
    pub async fn connect(&mut self) -> HcHttpGatewayResult<()> {
        self.current_retries = 0;

        while self.current_retries < ADMIN_WS_CONNECTION_MAX_RETRIES {
            match AdminWebsocket::connect(&self.url).await {
                Ok(conn) => {
                    let mut connection = self.connection_handle.lock().unwrap();
                    *connection = Some(conn);
                    self.current_retries = 0;

                    return Ok(());
                }
                Err(e) => {
                    self.current_retries += 1;
                    tracing::warn!(
                        "Failed to connect to WebSocket (attempt {}/{}): {:?}",
                        self.current_retries,
                        ADMIN_WS_CONNECTION_MAX_RETRIES,
                        e
                    );

                    if self.current_retries < ADMIN_WS_CONNECTION_MAX_RETRIES {
                        continue;
                    } else {
                        return Err(HcHttpGatewayError::UpstreamUnavailable);
                    }
                }
            }
        }

        Err(HcHttpGatewayError::ConfigurationError(format!(
            "Maximum connection retry attempts ({}) reached",
            ADMIN_WS_CONNECTION_MAX_RETRIES
        )))
    }

    /// Allows calling a method on the AdminWebsocket, with automatic reconnection if needed
    ///
    /// Accepts a function `f` that takes a AdminWebsocket and returns a Result with ConductorApiError
    pub async fn call<T, F, Fut>(&mut self, f: F) -> HcHttpGatewayResult<T>
    where
        F: Fn(Arc<AdminWebsocket>) -> Fut + Send + Clone,
        Fut: Future<Output = ConductorApiResult<T>> + Send,
    {
        self.ensure_connected().await?;

        // Try to execute the operation
        match self.exec(&f).await {
            Ok(result) => Ok(result),
            Err(e) if e.is_disconnect_error() => self.handle_disconnection(f).await,
            Err(e) => Err(e),
        }
    }

    // Helper method to execute an operation with the current connection
    async fn exec<T, F, Fut>(&mut self, f: &F) -> HcHttpGatewayResult<T>
    where
        F: Fn(Arc<AdminWebsocket>) -> Fut + Send,
        Fut: Future<Output = ConductorApiResult<T>> + Send,
    {
        // block scope to limit MutexGuard lifetime
        let connection = {
            let mut connection = self.connection_handle.lock().unwrap();
            let connection = connection.take().ok_or_else(|| {
                HcHttpGatewayError::InternalError(
                    "No connection available despite ensure_connected check".to_string(),
                )
            })?;

            Arc::new(connection)
        };

        f(connection).await.map_err(HcHttpGatewayError::from)
    }

    async fn handle_disconnection<T, F, Fut>(&mut self, f: F) -> HcHttpGatewayResult<T>
    where
        F: Fn(Arc<AdminWebsocket>) -> Fut + Send,
        Fut: Future<Output = ConductorApiResult<T>> + Send,
    {
        tracing::warn!("Detected disconnection. Attempting to connect...");

        match self.connect().await {
            Ok(()) => {
                tracing::info!("Reconnected successfully. Retrying operation.");

                // Retry the operation with the new connection
                self.exec(&f).await
            }
            Err(connect_err) => Err(connect_err),
        }
    }
}

/// Extension on HcHttpGatewayError to check if it's a disconnect error
trait HcHttpGatewayErrorExt {
    fn is_disconnect_error(&self) -> bool;
}

impl HcHttpGatewayErrorExt for HcHttpGatewayError {
    fn is_disconnect_error(&self) -> bool {
        match self {
            // Specifically checking for WebsocketError (inside ConductorApiError) and IoError as mentioned
            HcHttpGatewayError::HolochainError(api_error) => {
                // Check for websocket errors inside the ConductorApiError
                matches!(api_error, ConductorApiError::WebsocketError(_))
            }
            HcHttpGatewayError::IoError(_) => true,
            // All other errors don't trigger reconnection
            _ => false,
        }
    }
}
