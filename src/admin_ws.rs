use std::{
    future::Future,
    sync::{Arc, Mutex},
};

use holochain_client::{AdminWebsocket, ConductorApiError, ConductorApiResult};
use tokio::time::{sleep, Duration};

use crate::{HcHttpGatewayError, HcHttpGatewayResult};

const ADMIN_WS_CONNECTION_MAX_RETRIES: usize = 1;
const ADMIN_WS_CONNECTION_RETRY_DELAY_MS: u64 = 1000;

/// A wrapper around AdminWebsocket that automatically handles reconnection
/// when the connection is lost due to network issues or other failures.
#[derive(Clone)]
pub struct ReconnectingAdminWebsocket {
    /// The WebSocket URL to connect to
    url: String,
    /// The handle to the AdminWebsocket connection
    handle: Arc<Mutex<Option<AdminWebsocket>>>,
    /// Current retry attempt counter
    current_retries: usize,
}

impl ReconnectingAdminWebsocket {
    /// Creates a new ReconnectingAdminWebsocket with the specified parameters
    ///
    /// # Returns
    ///
    /// A new ReconnectingAdminWebsocket instance (not yet connected)
    pub fn new(url: &str) -> Self {
        ReconnectingAdminWebsocket {
            url: url.to_string(),
            handle: Arc::new(Mutex::new(None)),
            current_retries: 0,
        }
    }

    /// Establishes the initial connection to the AdminWebsocket
    ///
    /// This should be called before making any calls to the AdminWebsocket.
    /// It will attempt to establish a connection and store it internally.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If the connection was established successfully
    /// * `Err(HcHttpGatewayError)` - If the connection could not be established
    pub async fn connect(&mut self) -> HcHttpGatewayResult<()> {
        let conn = AdminWebsocket::connect(&self.url)
            .await
            .map_err(HcHttpGatewayError::from)?;

        let mut connection = self.handle.lock().map_err(|e| {
            HcHttpGatewayError::InternalError(format!("Mutex was poisoned during connect: {}", e))
        })?;

        *connection = Some(conn);
        self.current_retries = 0;

        Ok(())
    }

    /// Checks if there is an active connection
    fn is_connected(&self) -> HcHttpGatewayResult<bool> {
        let connection = self.handle.lock().map_err(|e| {
            HcHttpGatewayError::InternalError(format!(
                "Mutex was poisoned during is_connected check: {}",
                e
            ))
        })?;

        Ok(connection.is_some())
    }

    /// Gets reconnection retry count
    pub fn get_reconnection_retries(&self) -> usize {
        self.current_retries
    }

    /// Ensures that a connection is established before proceeding
    ///
    /// If a connection already exists, this is a no-op.
    /// If no connection exists, it will attempt to establish one.
    async fn ensure_connected(&mut self) -> HcHttpGatewayResult<()> {
        if self.is_connected()? {
            return Ok(());
        }

        self.reconnect().await
    }

    /// Attempts to reconnect to the AdminWebsocket
    ///
    /// This will make multiple attempts according to the `max_retries` and
    /// `retry_delay_ms` settings, with exponential backoff between retries.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - If reconnection was successful
    /// * `Err(HcHttpGatewayError)` - If reconnection failed after all retry attempts
    pub async fn reconnect(&mut self) -> HcHttpGatewayResult<()> {
        self.current_retries = 0;

        while self.current_retries < ADMIN_WS_CONNECTION_MAX_RETRIES {
            match AdminWebsocket::connect(&self.url).await {
                Ok(conn) => {
                    let mut connection = self.handle.lock().map_err(|e| {
                        HcHttpGatewayError::InternalError(format!(
                            "Mutex was poisoned during is_connected check: {}",
                            e
                        ))
                    })?;

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
                        sleep(Duration::from_millis(ADMIN_WS_CONNECTION_RETRY_DELAY_MS)).await;
                    } else {
                        return Err(HcHttpGatewayError::ConfigurationError(format!(
                            "Maximum connection retry attempts ({}) reached",
                            ADMIN_WS_CONNECTION_MAX_RETRIES
                        )));
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
    /// # Arguments
    ///
    /// * `f` - A function that takes a Boxed AdminWebsocket and returns a Result with ConductorApiError
    ///
    /// # Returns
    ///
    /// * `Ok(T)` - If the function executed successfully
    /// * `Err(HcHttpGatewayError)` - If an error occurred that could not be recovered from
    pub async fn call<T, F, Fut>(&mut self, f: F) -> HcHttpGatewayResult<T>
    where
        F: Fn(Arc<AdminWebsocket>) -> Fut + Send + Clone,
        Fut: Future<Output = ConductorApiResult<T>> + Send,
    {
        self.ensure_connected().await?;

        // Try to execute the operation
        match self.execute_operation(&f).await {
            Ok(result) => Ok(result),
            Err(e) if e.is_disconnect_error() => self.handle_disconnection(f).await,
            Err(e) => Err(e),
        }
    }

    // Helper method to execute an operation with the current connection
    async fn execute_operation<T, F, Fut>(&mut self, f: &F) -> HcHttpGatewayResult<T>
    where
        F: Fn(Arc<AdminWebsocket>) -> Fut + Send,
        Fut: Future<Output = ConductorApiResult<T>> + Send,
    {
        // block scope to limit MutexGuard lifetime
        let connection = {
            let mut connection = self.handle.lock().map_err(|e| {
                HcHttpGatewayError::InternalError(format!("Connection mutex was poisoned: {e}"))
            })?;

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
        tracing::warn!("Detected disconnection. Attempting to reconnect...");

        match self.reconnect().await {
            Ok(()) => {
                tracing::info!("Reconnected successfully. Retrying operation.");

                // Retry the operation with the new connection
                self.execute_operation(&f).await
            }
            Err(reconnect_err) => Err(HcHttpGatewayError::InternalError(format!(
                "Disconnection detected but reconnection failed: {}",
                reconnect_err
            ))),
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
            HcHttpGatewayError::ConductorApiError(api_error) => {
                // Check for websocket errors inside the ConductorApiError
                matches!(api_error, ConductorApiError::WebsocketError(_))
            }
            HcHttpGatewayError::IoError(_) => true,
            // All other errors don't trigger reconnection
            _ => false,
        }
    }
}
