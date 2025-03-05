//! HTTP gateway service for Holochain

use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};

use axum::{routing::get, Router};
use tokio::net::TcpListener;

use crate::{
    config::Configuration, error::HcHttpGatewayResult, routes::health_check,
    ReconnectingAdminWebsocket,
};

/// Core Holochain HTTP gateway service
#[derive(Debug)]
pub struct HcHttpGatewayService {
    address: SocketAddr,
    router: Router,
}

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    configuration: Configuration,
    #[allow(unused, reason = "Temporary")]
    admin_ws: ReconnectingAdminWebsocket,
}

impl HcHttpGatewayService {
    /// Create a new service instance bound to the given address and port
    pub async fn new(
        address: impl Into<IpAddr>,
        port: u16,
        configuration: Configuration,
    ) -> HcHttpGatewayResult<Self> {
        let address = SocketAddr::new(address.into(), port);
        let mut admin_ws = ReconnectingAdminWebsocket::new(&configuration.admin_ws_url.as_ref());
        admin_ws.connect().await?;

        let state = Arc::new(AppState {
            configuration,
            admin_ws,
        });

        let router = Router::new()
            .route("/health", get(health_check))
            .with_state(state.clone());

        tracing::info!("Configuration: {:?}", state.configuration);

        Ok(HcHttpGatewayService { router, address })
    }

    /// Get the socket address the service is configured to use
    pub fn address(&self) -> SocketAddr {
        self.address
    }

    /// Start the HTTP server and run until terminated
    pub async fn run(self) -> HcHttpGatewayResult<()> {
        let address = self.address();
        let listener = TcpListener::bind(self.address).await?;

        tracing::info!("Starting server on {}", address);
        axum::serve(listener, self.router)
            .await
            .inspect_err(|e| tracing::error!("Failed to bind to {}: {}", address, e))?;

        Ok(())
    }
}
