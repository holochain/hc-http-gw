//! HTTP gateway service for Holochain

use crate::{
    config::Configuration, error::HcHttpGatewayResult, router::hc_http_gateway_router,
    HcHttpGatewayError,
};
use axum::Router;
use std::net::{IpAddr, SocketAddr};
use tokio::net::TcpListener;

/// Core Holochain HTTP gateway service
#[derive(Debug)]
pub struct HcHttpGatewayService {
    listener: TcpListener,
    router: Router,
}

/// Shared application state
#[derive(Debug, Clone)]
pub struct AppState {
    pub configuration: Configuration,
    cached_app_port: Arc<RwLock<Option<u16>>>,
    app_clients: Arc<tokio::sync::RwLock<HashMap<InstalledAppId, AppWebsocket>>>,
}

impl HcHttpGatewayService {
    /// Create a new service instance bound to the given address and port
    pub async fn new(
        address: impl Into<IpAddr>,
        port: u16,
        configuration: Configuration,
    ) -> HcHttpGatewayResult<Self> {
        tracing::info!("Configuration: {:?}", configuration);

        let router = hc_http_gateway_router(configuration);
        let address = SocketAddr::new(address.into(), port);
        let listener = TcpListener::bind(address).await?;

        Ok(HcHttpGatewayService { router, listener })
    }

    /// Get the socket address the service is configured to use
    pub fn address(&self) -> HcHttpGatewayResult<SocketAddr> {
        self.listener
            .local_addr()
            .map_err(HcHttpGatewayError::IoError)
    }

    /// Start the HTTP server and run until terminated
    pub async fn run(self) -> HcHttpGatewayResult<()> {
        let address = self.address()?;

        tracing::info!("Starting server on {}", address);
        axum::serve(self.listener, self.router)
            .await
            .inspect_err(|e| tracing::error!("Failed to bind to {}: {}", address, e))?;

        Ok(())
    }
}

impl AppState {
    pub async fn get_or_connect_app_client(
        &self,
        installed_app_id: InstalledAppId,
        admin_ws: AdminWebsocket,
    ) -> HcHttpGatewayResult<AppWebsocket> {
        {
            let app_clients = self.app_clients.read().await;
            if let Some(client) = app_clients.get(&installed_app_id) {
                return Ok(client.clone());
            }
        }

        match self
            .app_clients
            .write()
            .await
            .entry(installed_app_id.clone())
        {
            std::collections::hash_map::Entry::Occupied(client) => {
                // Created by another thread while we were waiting for the lock
                Ok(client.get().clone())
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                let app_client = self.connect_app_client(installed_app_id, admin_ws).await?;

                entry.insert(app_client.clone());

                Ok(app_client)
            }
        }
    }

    async fn connect_app_client(
        &self,
        installed_app_id: InstalledAppId,
        admin_ws: AdminWebsocket,
    ) -> HcHttpGatewayResult<AppWebsocket> {
        let app_port = self
            .get_app_port(&installed_app_id, admin_ws.clone())
            .await?;

        let issued = admin_ws
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
        let request = ConnectRequest::from(
            format!("ws://{host}:{app_port}")
                .parse::<SocketAddr>()
                .map_err(|e| {
                    HcHttpGatewayError::ConfigurationError(format!("Invalid socket address: {}", e))
                })?,
        )
        .try_set_header("Origin", HTTP_GW_ORIGIN)?;

        let mut config = WebsocketConfig::CLIENT_DEFAULT;
        config.default_request_timeout = std::time::Duration::from_secs(5);

        let client_signer = ClientAgentSigner::default();
        let app_client = AppWebsocket::connect_with_request_and_config(
            request,
            Arc::new(config),
            issued.token,
            client_signer.clone().into(),
        )
        .await?;

        // TODO use cached
        let app_info = app_client.cached_app_info();
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

        for cell_id in cells {
            let credentials = admin_ws
                .authorize_signing_credentials(AuthorizeSigningCredentialsPayload {
                    cell_id: cell_id.clone(),
                    functions: Some(granted_functions.clone()),
                })
                .await?;

            client_signer.add_credentials(cell_id, credentials);
        }
        Ok(app_client)
    }

    async fn get_app_port(
        &self,
        installed_app_id: &InstalledAppId,
        admin_ws: AdminWebsocket,
    ) -> HcHttpGatewayResult<u16> {
        if let Some(app_port) = self.cached_app_port.read().expect("Invalid lock").as_ref() {
            return Ok(*app_port);
        }

        let app_interfaces = admin_ws.list_app_interfaces().await?;

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
                admin_ws
                    .attach_app_interface(0, AllowedOrigins::from(HTTP_GW_ORIGIN.to_string()), None)
                    .await?
            }
        };
        *self.cached_app_port.write().expect("Invalid app port") = Some(app_port);

        Ok(app_port)
    }
}
