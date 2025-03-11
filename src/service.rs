//! HTTP gateway service for Holochain

use crate::{config::Configuration, router::hc_http_gateway_router, HcHttpGwAdminWebsocket};
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
    #[allow(unused, reason = "Temporary")]
    pub admin_ws: HcHttpGwAdminWebsocket,
}

impl HcHttpGatewayService {
    /// Create a new service instance bound to the given address and port
    pub async fn new(
        address: impl Into<IpAddr>,
        port: u16,
        configuration: Configuration,
    ) -> std::io::Result<Self> {
        tracing::info!("Configuration: {:?}", configuration);

        let router = hc_http_gateway_router(configuration).await.map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to setup router: {e}"),
            )
        })?;
        let address = SocketAddr::new(address.into(), port);
        let listener = TcpListener::bind(address).await?;

        Ok(HcHttpGatewayService { router, listener })
    }

    /// Get the socket address the service is configured to use
    pub fn address(&self) -> std::io::Result<SocketAddr> {
        self.listener.local_addr()
    }

    /// Start the HTTP server and run until terminated
    pub async fn run(self) -> std::io::Result<()> {
        let address = self.address()?;

        tracing::info!("Starting server on {}", address);
        axum::serve(self.listener, self.router).await?;

        Ok(())
    }
}
