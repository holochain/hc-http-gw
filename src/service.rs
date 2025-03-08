//! HTTP gateway service for Holochain

use std::{
    net::{IpAddr, SocketAddr},
    vec::Vec,
};

use axum::{routing::get, Router};
use holochain_client::AppInfo;
use tokio::net::TcpListener;

use crate::{
    config::Configuration,
    error::HcHttpGatewayResult,
    routes::{app_selection, health_check, zome_call},
    AdminWebsocketWrapper, HcHttpGatewayError,
};

/// Core Holochain HTTP gateway service
#[derive(Debug)]
pub struct HcHttpGatewayService {
    listener: TcpListener,
    router: Router,
}

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub configuration: Configuration,
    pub admin_websocket: AdminWebsocketWrapper,
    pub installed_apps: Vec<AppInfo>,
}

impl AppState {
    async fn from_config(configuration: Configuration) -> Self {
        let socket_addr = configuration.admin_ws_url.to_string();
        Self {
            configuration,
            admin_websocket: AdminWebsocketWrapper::connect(&socket_addr).await,
            installed_apps: Default::default(),
        }
    }
}

impl HcHttpGatewayService {
    /// Create a new service instance bound to the given address and port
    pub async fn new(
        address: impl Into<IpAddr>,
        port: u16,
        configuration: Configuration,
    ) -> HcHttpGatewayResult<Self> {
        let address = SocketAddr::new(address.into(), port);

        let state = AppState::from_config(configuration).await;

        let router = Router::new()
            .route("/{dna_hash}/{coordinator_identifier}", get(app_selection))
            .route(
                "/{dna_hash}/{coordinator_identifier}/{zome_name}/{function_name}",
                get(zome_call),
            )
            .route("/health", get(health_check))
            .with_state(state.clone());

        tracing::info!("Configuration: {:?}", state.configuration);

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
