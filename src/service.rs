//! HTTP gateway service for Holochain

use crate::{
    config::Configuration, error::HcHttpGatewayResult, router::hc_http_gateway_router,
    HcHttpGatewayError, HcHttpGwAdminWebsocket,
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
#[derive(Clone)]
pub struct AppState {
    pub configuration: Configuration,
    #[allow(unused, reason = "Temporary")]
    pub admin_ws: HcHttpGwAdminWebsocket,
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
