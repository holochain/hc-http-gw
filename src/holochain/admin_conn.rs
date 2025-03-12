use crate::HcHttpGatewayError;
use crate::{AdminCall, HcHttpGatewayResult};
use futures::future::BoxFuture;
use holochain_client::{
    AdminWebsocket, AppInfo, AuthorizeSigningCredentialsPayload, ConductorApiError,
    SigningCredentials,
};
use holochain_conductor_api::{
    AppAuthenticationTokenIssued, AppInterfaceInfo, AppStatusFilter,
    IssueAppAuthenticationTokenPayload,
};
use holochain_types::websocket::AllowedOrigins;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;

const ADMIN_WS_CONNECTION_MAX_RETRIES: usize = 1;

/// A wrapper around AdminWebsocket that automatically handles reconnection
/// when the connection is lost due to network issues or other failures.
#[derive(Debug, Clone)]
pub struct AdminConn {
    /// The WebSocket URL to connect to
    socket_addr: SocketAddr,

    /// The handle to the AdminWebsocket connection - always contains a valid connection
    handle: Arc<RwLock<Option<AdminWebsocket>>>,
}

impl AdminConn {
    /// Creates a new [`AdminConn`] that will attempt to maintain an [`AdminWebsocket`] connection
    /// to the specified socket address.
    pub fn new(socket_addr: SocketAddr) -> Self {
        Self {
            socket_addr,
            handle: Default::default(),
        }
    }

    /// Allows calling a method on the [`AdminWebsocket`], with automatic reconnection if needed
    pub async fn call<T>(
        &self,
        execute: impl Fn(AdminWebsocket) -> BoxFuture<'static, HcHttpGatewayResult<T>>,
    ) -> HcHttpGatewayResult<T> {
        for _ in 0..=ADMIN_WS_CONNECTION_MAX_RETRIES {
            let admin_ws = self.get_admin_ws().await?;

            match execute(admin_ws).await {
                Ok(output) => return Ok(output),
                Err(HcHttpGatewayError::HolochainError(ConductorApiError::WebsocketError(e))) => {
                    tracing::warn!(
                        ?e,
                        "Detected admin websocket disconnection. Attempting to reconnect"
                    );
                    *self.handle.write().await = None;
                    continue;
                }
                Err(e) => return Err(e),
            }
        }

        Err(HcHttpGatewayError::UpstreamUnavailable)
    }

    async fn get_admin_ws(&self) -> HcHttpGatewayResult<AdminWebsocket> {
        {
            let lock = self.handle.read().await;

            if let Some(admin_ws) = lock.as_ref() {
                return Ok(admin_ws.clone());
            }
        }

        let mut lock = self.handle.write().await;

        match AdminWebsocket::connect(self.socket_addr).await {
            Ok(admin_ws) => {
                tracing::info!("Connected a new Holochain admin websocket");
                *lock = Some(admin_ws.clone());
                Ok(admin_ws)
            }
            Err(e) => {
                tracing::error!(?e, "Failed to connect Holochain admin websocket");
                Err(HcHttpGatewayError::UpstreamUnavailable)
            }
        }
    }
}

impl AdminCall for AdminConn {
    fn list_app_interfaces(
        &self,
    ) -> BoxFuture<'static, HcHttpGatewayResult<Vec<AppInterfaceInfo>>> {
        let this = self.clone();
        Box::pin(async move {
            this.call(|admin_ws| Box::pin(async move { Ok(admin_ws.list_app_interfaces().await?) }))
                .await
        })
    }

    fn issue_app_auth_token(
        &self,
        payload: IssueAppAuthenticationTokenPayload,
    ) -> BoxFuture<'static, HcHttpGatewayResult<AppAuthenticationTokenIssued>> {
        let this = self.clone();
        Box::pin(async move {
            this.call(|admin_ws| {
                let payload = IssueAppAuthenticationTokenPayload {
                    installed_app_id: payload.installed_app_id.clone(),
                    expiry_seconds: payload.expiry_seconds,
                    single_use: payload.single_use,
                };

                Box::pin(async move { Ok(admin_ws.issue_app_auth_token(payload).await?) })
            })
            .await
        })
    }

    fn authorize_signing_credentials(
        &self,
        payload: AuthorizeSigningCredentialsPayload,
    ) -> BoxFuture<'static, HcHttpGatewayResult<SigningCredentials>> {
        let this = self.clone();
        Box::pin(async move {
            this.call(|admin_ws| {
                let payload = payload.clone();

                Box::pin(async move { Ok(admin_ws.authorize_signing_credentials(payload).await?) })
            })
            .await
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
            this.call(|admin_ws| {
                let allowed_origins = allowed_origins.clone();
                let installed_app_id = installed_app_id.clone();

                Box::pin(async move {
                    Ok(admin_ws
                        .attach_app_interface(port, allowed_origins, installed_app_id)
                        .await?)
                })
            })
            .await
        })
    }

    fn list_apps(
        &self,
        status_filter: Option<AppStatusFilter>,
    ) -> BoxFuture<'static, HcHttpGatewayResult<Vec<AppInfo>>> {
        let this = self.clone();
        Box::pin(async move {
            this.call(|admin_ws| {
                let status_filter = status_filter.clone();

                Box::pin(async move { Ok(admin_ws.list_apps(status_filter).await?) })
            })
            .await
        })
    }
}
