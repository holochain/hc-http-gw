use crate::{AdminCall, HcHttpGatewayResult};
use futures::future::BoxFuture;
use holochain_client::{
    AdminWebsocket, AuthorizeSigningCredentialsPayload, ConductorApiError, ConductorApiResult,
    SigningCredentials,
};
use holochain_conductor_api::{
    AppAuthenticationTokenIssued, AppInterfaceInfo, IssueAppAuthenticationTokenPayload,
};
use holochain_types::websocket::AllowedOrigins;
use std::sync::Arc;
use tokio::sync::RwLock;
use url2::Url2;

use crate::HcHttpGatewayError;

const ADMIN_WS_CONNECTION_MAX_RETRIES: usize = 1;

/// A wrapper around AdminWebsocket that automatically handles reconnection
/// when the connection is lost due to network issues or other failures.
#[derive(Debug, Clone)]
pub struct AdminConn {
    /// The WebSocket URL to connect to
    url: Url2,
    /// The handle to the AdminWebsocket connection - always contains a valid connection
    connection_handle: Arc<RwLock<AdminWebsocket>>,
}

impl AdminConn {
    /// Creates a new AdminConn by establishing a connection to the given URL
    ///
    /// This will make multiple attempts according to the max retries setting.
    pub async fn connect(url: &Url2) -> HcHttpGatewayResult<Self> {
        let admin_ws_url = Self::format_ws_url(url)?;
        let mut current_retries = 0;

        while current_retries < ADMIN_WS_CONNECTION_MAX_RETRIES {
            match AdminWebsocket::connect(&admin_ws_url).await {
                Ok(conn) => {
                    return Ok(Self {
                        url: url.clone(),
                        connection_handle: Arc::new(RwLock::new(conn)),
                    });
                }
                Err(e) => {
                    current_retries += 1;
                    tracing::warn!(
                        "Failed to connect to WebSocket (attempt {}/{}): {:?}",
                        current_retries,
                        ADMIN_WS_CONNECTION_MAX_RETRIES,
                        e
                    );

                    if current_retries < ADMIN_WS_CONNECTION_MAX_RETRIES {
                        continue;
                    } else {
                        return Err(HcHttpGatewayError::UpstreamUnavailable);
                    }
                }
            }
        }

        Err(HcHttpGatewayError::InternalError(format!(
            "Maximum connection retry attempts ({}) reached",
            ADMIN_WS_CONNECTION_MAX_RETRIES
        )))
    }

    /// Formats the WebSocket URL from the provided Url2
    fn format_ws_url(url: &Url2) -> HcHttpGatewayResult<String> {
        let host = url.host_str().ok_or_else(|| {
            HcHttpGatewayError::InternalError("Invalid admin ws host".to_string())
        })?;

        let port = url.port().ok_or_else(|| {
            HcHttpGatewayError::InternalError("Port is absent from the admin ws url".to_string())
        })?;

        Ok(format!("{}:{}", host, port))
    }

    /// Attempts to reconnect to the AdminWebsocket when a connection failure is detected
    async fn reconnect(&self) -> HcHttpGatewayResult<()> {
        let admin_ws_url = Self::format_ws_url(&self.url)?;
        let mut current_retries = 0;

        while current_retries < ADMIN_WS_CONNECTION_MAX_RETRIES {
            match AdminWebsocket::connect(&admin_ws_url).await {
                Ok(conn) => {
                    // Replace the existing connection with the new one
                    let mut connection = self.connection_handle.write().await;
                    *connection = conn;
                    return Ok(());
                }
                Err(e) => {
                    current_retries += 1;
                    tracing::warn!(
                        "Failed to reconnect to WebSocket (attempt {}/{}): {:?}",
                        current_retries,
                        ADMIN_WS_CONNECTION_MAX_RETRIES,
                        e
                    );

                    if current_retries < ADMIN_WS_CONNECTION_MAX_RETRIES {
                        continue;
                    } else {
                        return Err(HcHttpGatewayError::UpstreamUnavailable);
                    }
                }
            }
        }

        Err(HcHttpGatewayError::InternalError(format!(
            "Maximum reconnection retry attempts ({}) reached",
            ADMIN_WS_CONNECTION_MAX_RETRIES
        )))
    }

    /// Allows calling a method on the AdminWebsocket, with automatic reconnection if needed
    pub async fn call<T, F, FnFactory>(&self, fn_factory: FnFactory) -> HcHttpGatewayResult<T>
    where
        F: FnOnce(Arc<AdminWebsocket>) -> BoxFuture<'static, ConductorApiResult<T>> + Send,
        FnFactory: Fn() -> F + Send + Sync + 'static,
        T: Send + 'static,
    {
        // First attempt
        let connection = {
            let connection = self.connection_handle.read().await;
            Arc::new(connection.clone())
        };

        // Create and call the first closure
        match fn_factory()(connection).await {
            Ok(result) => Ok(result),
            Err(e) if matches!(e, ConductorApiError::WebsocketError(_)) => {
                tracing::warn!("Detected disconnection. Attempting to reconnect...");

                match self.reconnect().await {
                    Ok(()) => {
                        tracing::info!("Reconnected successfully. Retrying operation.");

                        // Get a new connection after reconnection
                        let connection = {
                            let connection = self.connection_handle.read().await;
                            Arc::new(connection.clone())
                        };

                        // Create and call a new closure for the retry
                        fn_factory()(connection)
                            .await
                            .map_err(HcHttpGatewayError::from)
                    }
                    Err(connect_err) => Err(connect_err),
                }
            }
            Err(e) => Err(HcHttpGatewayError::from(e)),
        }
    }
}

impl AdminCall for AdminConn {
    fn list_app_interfaces(
        &self,
    ) -> BoxFuture<'static, HcHttpGatewayResult<Vec<AppInterfaceInfo>>> {
        let this = self.clone();

        Box::pin(async move {
            let factory = move || {
                move |admin_ws: Arc<AdminWebsocket>| -> BoxFuture<'static, ConductorApiResult<Vec<AppInterfaceInfo>>> {
                let admin_ws = admin_ws.clone();

                Box::pin(async move {
                    admin_ws.list_app_interfaces().await
                })
            }
            };
            this.call(factory).await
        })
    }

    fn issue_app_auth_token(
        &self,
        _payload: IssueAppAuthenticationTokenPayload,
    ) -> BoxFuture<'static, HcHttpGatewayResult<AppAuthenticationTokenIssued>> {
        todo!()
    }

    fn authorize_signing_credentials(
        &self,
        payload: AuthorizeSigningCredentialsPayload,
    ) -> BoxFuture<'static, HcHttpGatewayResult<SigningCredentials>> {
        let this = self.clone();

        Box::pin(async move {
            let factory = move || {
                let payload_clone = payload.clone();

                move |admin_ws: Arc<AdminWebsocket>| -> BoxFuture<'static, ConductorApiResult<SigningCredentials>> {
                let admin_ws = admin_ws.clone();

                Box::pin(async move {
                    admin_ws.authorize_signing_credentials(payload_clone).await
                })
            }
            };

            this.call(factory).await
        })
    }

    fn attach_app_interface(
        &self,
        port: u16,
        allowed_origins: AllowedOrigins,
        installed_app_id: Option<String>,
    ) -> BoxFuture<'static, HcHttpGatewayResult<u16>> {
        let this = self.clone();

        Box::pin(async move {
            let factory = move || {
                let allowed_origins = allowed_origins.clone();
                let installed_app_id = installed_app_id.clone();

                move |admin_ws: Arc<AdminWebsocket>| -> BoxFuture<'static, ConductorApiResult<u16>> {
                    let admin_ws = admin_ws.clone();

                    Box::pin(async move {
                        admin_ws
                            .attach_app_interface(port, allowed_origins, installed_app_id)
                            .await
                    })
                }
            };

            this.call(factory).await
        })
    }

    #[cfg(feature = "test-utils")]
    fn set_admin_ws(&self, _admin_ws: holochain_client::AdminWebsocket) -> BoxFuture<'static, ()> {
        todo!()
    }
}
