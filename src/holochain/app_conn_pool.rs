use crate::config::{AllowedFns, Configuration};
use crate::holochain::{AdminCall, AppCall};
use crate::{HcHttpGatewayError, HcHttpGatewayResult};
use futures::future::BoxFuture;
use holochain_client::{
    AppWebsocket, AuthorizeSigningCredentialsPayload, CellInfo, ClientAgentSigner,
    ConductorApiError, ConnectRequest, ExternIO, GrantedFunctions,
    IssueAppAuthenticationTokenPayload, Timestamp, WebsocketConfig,
};
use holochain_types::app::InstalledAppId;
use holochain_types::websocket::AllowedOrigins;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};

/// The origin that the gateway will use when connecting to Holochain app interfaces.
pub const HTTP_GW_ORIGIN: &str = "hc-http-gw";

/// A wrapper around an app websocket connection that includes state required to manage the
/// connection.
#[derive(Debug, Clone)]
pub struct AppWebsocketWithState {
    /// The app websocket connection.
    pub app_ws: AppWebsocket,
    /// The time at which the connection was opened.
    pub opened_at: Timestamp,
}

/// A connection pool for app connections.
///
/// This is a pool in the sense that it manages multiple connections to Holochain app interfaces,
/// but it will manage exactly one connection per installed app.
#[derive(Debug, Clone)]
pub struct AppConnPool {
    configuration: Configuration,
    admin_call: Arc<dyn AdminCall>,
    cached_app_port: Arc<RwLock<Option<u16>>>,
    app_clients: Arc<tokio::sync::RwLock<HashMap<InstalledAppId, AppWebsocketWithState>>>,
}

impl AppConnPool {
    /// Create a new app connection pool with the given configuration.
    pub fn new(configuration: Configuration, admin_call: Arc<dyn AdminCall>) -> Self {
        Self {
            configuration,
            admin_call,
            cached_app_port: Default::default(),
            app_clients: Default::default(),
        }
    }

    /// Call a function with an app client for the given installed app ID.
    ///
    /// This function takes care of reconnecting to the app client if the connection is lost. Your
    /// function is free to operate on the app client without worrying about the connection state.
    ///
    /// Exactly one attempt is made to reconnect and re-run the provided callback. If there is a
    /// second failure after reconnecting, the error is returned to the caller.
    pub async fn call<T>(
        &self,
        installed_app_id: InstalledAppId,
        execute: impl Fn(AppWebsocket) -> BoxFuture<'static, HcHttpGatewayResult<T>>,
    ) -> HcHttpGatewayResult<T> {
        // The first attempt may discover that the connection is invalid
        // On the second attempt, we will reconnect without using a cached app port
        // On the third attempt, we will reconnect permitting that a new app interface can be created
        for _ in 0..3 {
            let app_ws = match self
                .get_or_connect_app_client(installed_app_id.clone())
                .await
            {
                Ok(app_ws) => app_ws,
                Err(HcHttpGatewayError::UpstreamUnavailable) => {
                    tracing::info!("Unable to connect app client, attempting to reconnect without cached settings");

                    // In this case, we tried and failed to open a new connection to Holochain.
                    // Assume that this was because the port we used is no longer available.
                    continue;
                }
                Err(e) => return Err(e),
            };
            match execute(app_ws).await {
                Ok(response) => {
                    return Ok(response);
                }
                Err(HcHttpGatewayError::HolochainError(ConductorApiError::WebsocketError(e))) => {
                    tracing::error!(
                        "Websocket error while executing call, attempting to reconnect: {}",
                        e
                    );
                    self.remove_app_client(&installed_app_id).await;

                    // This is the first error we expect to encounter, that the app websocket
                    // connection is no longer valid. We should attempt to reconnect.
                    continue;
                }
                Err(e) => return Err(e),
            }
        }

        // Must mean we could not get anything other than a websocket error, otherwise we'd have
        // got a response or some other error.
        Err(HcHttpGatewayError::UpstreamUnavailable)
    }

    /// Get or connect an app client for the given installed app ID.
    ///
    /// If the returned connection is invalid, it is the caller's responsibility to call
    /// [`AppConnPool::remove_app_client`] to remove it from the connection list. The next call to this
    /// function will attempt to reconnect.
    pub async fn get_or_connect_app_client(
        &self,
        installed_app_id: InstalledAppId,
    ) -> HcHttpGatewayResult<AppWebsocket> {
        {
            let app_clients = self.app_clients.read().await;

            if let Some(client) = app_clients.get(&installed_app_id) {
                return Ok(client.app_ws.clone());
            }
        }

        let mut app_client_lock = self.app_clients.write().await;

        let app_ws = match app_client_lock.entry(installed_app_id.clone()) {
            std::collections::hash_map::Entry::Occupied(client) => {
                // Created by another thread while we were waiting for the lock
                client.get().app_ws.clone()
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                let app_ws = self.attempt_connect_app_ws(installed_app_id).await?;

                entry.insert(AppWebsocketWithState {
                    app_ws: app_ws.clone(),
                    opened_at: Timestamp::now(),
                });

                app_ws
            }
        };

        if app_client_lock.len() > self.configuration.max_app_connections as usize {
            // Find and remove the oldest connection
            let installed_app_id = app_client_lock
                .iter()
                .min_by_key(|(_, v)| v.opened_at)
                .map(|(k, _)| k.clone())
                .expect("Invalid lock");

            tracing::warn!(
                "Reached maximum app connections, removing connection for app: {}",
                installed_app_id
            );

            app_client_lock.remove(&installed_app_id);
        }

        Ok(app_ws)
    }

    /// Remove an app client from the pool.
    pub async fn remove_app_client(&self, installed_app_id: &InstalledAppId) {
        self.app_clients.write().await.remove(installed_app_id);
    }

    async fn attempt_connect_app_ws(
        &self,
        installed_app_id: InstalledAppId,
    ) -> HcHttpGatewayResult<AppWebsocket> {
        tracing::debug!(
            "Attempting to connect to app client for {}",
            installed_app_id
        );

        // Get the app port for a compatible app interface, which may be a cached value.
        let app_port = self.get_app_port(&installed_app_id).await?;
        tracing::debug!("Using app port {}", app_port);

        // Issue an app authentication token to allow us to connect a new client.
        let issued = self
            .admin_call
            .issue_app_auth_token(IssueAppAuthenticationTokenPayload::for_installed_app_id(
                installed_app_id.clone(),
            ))
            .await?;

        // Presence of host must have been checked to get an admin connection
        let host = self
            .configuration
            .admin_ws_url
            .host_str()
            .expect("Must have a host");

        tracing::info!("Connecting to app websocket at {}:{}", host, app_port);

        // Build a connection request
        let request =
            ConnectRequest::from(format!("{host}:{app_port}").parse::<SocketAddr>().map_err(
                |e| {
                    HcHttpGatewayError::ConfigurationError(format!("Invalid socket address: {}", e))
                },
            )?)
            .try_set_header("Origin", HTTP_GW_ORIGIN)?;

        // Create a websocket client configuration and lower the default timeout. We are connecting
        // locally to a running Holochain. If requests take longer than the configured timeout then
        // we want to free up the HTTP gateway to handle other requests.
        // Note that the zome call timeout that we're configuring here also applies to the
        // connection timeout. There's no way to set them separately.
        let mut config = WebsocketConfig::CLIENT_DEFAULT;
        config.default_request_timeout = self.configuration.zome_call_timeout;

        let client_signer = ClientAgentSigner::default();

        // Attempt to connect to the app websocket
        let app_ws = match AppWebsocket::connect_with_request_and_config(
            request,
            Arc::new(config),
            issued.token,
            client_signer.clone().into(),
        )
        .await
        {
            Ok(client) => client,
            Err(e) => {
                tracing::error!("Failed to connect to app websocket: {}", e);

                // If we failed to make a connection, clear the cached app port so that the next
                // attempt will re-check the app interfaces.
                *self.cached_app_port.write().expect("Invalid lock") = None;

                // Mark the upstream as unavailable so that the caller can retry
                return Err(HcHttpGatewayError::UpstreamUnavailable);
            }
        };
        tracing::debug!("Connected to app websocket");

        let app_info = app_ws.cached_app_info();
        let cells = app_info
            .cell_info
            .values()
            .flat_map(|cell_infos| {
                cell_infos.iter().flat_map(|cell_info| {
                    match cell_info {
                        CellInfo::Provisioned(provisioned) => Some(provisioned.cell_id.clone()),
                        // TODO Provisioning of these wouldn't be dynamic, you'd have to
                        //      restart the gateway or Holochain to get new credentials for
                        //      new clones...
                        //      See https://github.com/holochain/holochain/issues/4595
                        // CellInfo::Cloned(clone_cell) => Some(clone_cell.cell_id.clone()),
                        _ => None,
                    }
                })
            })
            .collect::<Vec<_>>();
        tracing::debug!("Collected cells to authorize: {:?}", cells);

        // Map the allowed functions to granted functions
        //
        // Direct access because we should already have checked that a zome call is allowed
        // for this app before getting an app connection.
        let granted_functions = match &self.configuration.allowed_fns[&installed_app_id] {
            AllowedFns::All => GrantedFunctions::All,
            AllowedFns::Restricted(fns) => GrantedFunctions::Listed(
                fns.iter()
                    .map(|zf| (zf.zome_name.clone().into(), zf.fn_name.clone().into()))
                    .collect(),
            ),
        };
        tracing::debug!("Granting access to functions: {:?}", granted_functions);

        // For each cell in the app, authorize signing credentials for the granted functions
        for cell_id in cells {
            let credentials = self
                .admin_call
                .authorize_signing_credentials(AuthorizeSigningCredentialsPayload {
                    cell_id: cell_id.clone(),
                    functions: Some(granted_functions.clone()),
                })
                .await?;
            tracing::debug!("Authorized credentials for cell {}", cell_id);

            client_signer.add_credentials(cell_id, credentials);
        }

        Ok(app_ws)
    }

    async fn get_app_port(&self, installed_app_id: &InstalledAppId) -> HcHttpGatewayResult<u16> {
        {
            if let Some(app_port) = self.cached_app_port.read().expect("Invalid lock").as_ref() {
                return Ok(*app_port);
            }
        }

        let app_interfaces = self.admin_call.list_app_interfaces().await?;

        let selected_app_interface = app_interfaces.into_iter().find(|app_interface| {
            if let Some(ref for_app_id) = app_interface.installed_app_id {
                if for_app_id != installed_app_id {
                    return false;
                }
            }

            app_interface.allowed_origins.is_allowed(HTTP_GW_ORIGIN)
        });

        let app_port = match selected_app_interface {
            Some(app_interface) => app_interface.port,
            None => {
                self.admin_call
                    .attach_app_interface(0, AllowedOrigins::from(HTTP_GW_ORIGIN.to_string()), None)
                    .await?
            }
        };
        *self.cached_app_port.write().expect("Invalid app port") = Some(app_port);

        Ok(app_port)
    }

    /// Get the inner pool for testing purposes.
    #[cfg(feature = "test-utils")]
    pub fn get_inner_pool(
        &self,
    ) -> Arc<tokio::sync::RwLock<HashMap<InstalledAppId, AppWebsocketWithState>>> {
        self.app_clients.clone()
    }

    /// Set the admin websocket for testing purposes.
    ///
    /// This is temporary, we should be using the admin websocket that can reconnect.
    #[cfg(feature = "test-utils")]
    pub async fn set_admin_ws(&mut self, admin_ws: holochain_client::AdminWebsocket) {
        self.admin_call.set_admin_ws(admin_ws).await;
    }
}

impl AppCall for AppConnPool {
    fn handle_zome_call(
        &self,
        installed_app_id: InstalledAppId,
        execute: fn(AppWebsocket) -> BoxFuture<'static, HcHttpGatewayResult<ExternIO>>,
    ) -> BoxFuture<'static, HcHttpGatewayResult<ExternIO>> {
        let this = self.clone();
        Box::pin(async move { this.call(installed_app_id, execute).await })
    }
}
