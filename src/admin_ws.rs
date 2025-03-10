use std::{
    future::Future,
    sync::{Arc, Mutex},
};
use tokio::time::{sleep, Duration};

use holochain_client::{AdminWebsocket, ConductorApiError, ConductorApiResult};

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
        let conn = AdminWebsocket::connect(&self.url).await?;

        let mut connection = self.handle.lock().unwrap();
        *connection = Some(conn);

        self.current_retries = 0;
        Ok(())
    }

    /// Checks if there is an active connection
    fn is_connected(&self) -> HcHttpGatewayResult<bool> {
        let connection = self.handle.lock().unwrap();
        Ok(connection.is_some())
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
                    let mut connection = self.handle.lock().unwrap();
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
                            ADMIN_WS_CONNECTION_RETRY_DELAY_MS
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
    /// This is a convenience wrapper around `call_inner` that automatically converts
    /// ConductorApiError to HcHttpGatewayError.
    ///
    /// # Arguments
    ///
    /// * `f` - A function that takes a reference to an AdminWebsocket and returns a Result with ConductorApiError
    ///
    /// # Returns
    ///
    /// * `Ok(T)` - If the function executed successfully
    /// * `Err(HcHttpGatewayError)` - If an error occurred that could not be recovered from
    pub async fn call<F, Fut, T>(&mut self, f: F) -> HcHttpGatewayResult<T>
    where
        F: Fn(&AdminWebsocket) -> Fut + Send,
        Fut: Future<Output = ConductorApiResult<T>> + Send,
    {
        // Ensure we're connected before proceeding
        self.ensure_connected().await?;

        // Execute the provided function
        let result = {
            let connection = self.handle.lock().unwrap();
            let conn = connection.as_ref().unwrap();
            match f(conn).await {
                Ok(value) => Ok(value),
                Err(err) => Err(HcHttpGatewayError::from(err)),
            }
        };

        // Handle potential disconnection
        match result {
            Ok(res) => Ok(res),
            Err(e) => {
                // If the error indicates a disconnect, try to reconnect and retry once
                if e.is_disconnect_error() {
                    tracing::warn!("Detected disconnection. Attempting to reconnect...");
                    if let Ok(()) = self.reconnect().await {
                        tracing::info!("Reconnected successfully. Retrying operation.");
                        let connection = self.handle.lock().unwrap();
                        let conn = connection.as_ref().unwrap();
                        // Retry the operation with the new connection
                        match f(conn).await {
                            Ok(value) => Ok(value),
                            Err(err) => Err(HcHttpGatewayError::from(err)),
                        }
                    } else {
                        Err(e)
                    }
                } else {
                    Err(e)
                }
            }
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
