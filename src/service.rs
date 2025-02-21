//! HTTP gateway service for Holochain

use std::net::{IpAddr, SocketAddr};

use axum::{routing::get, Router};
use tokio::net::TcpListener;

use crate::{error::HcHttpGatewayResult, routes::health_check};

/// Core Holochain HTTP gateway service
#[derive(Debug)]
pub struct HcHttpGatewayService {
    address: SocketAddr,
    router: Router,
}

/// Shared application state
#[derive(Debug, Clone)]
pub struct AppState {}

impl HcHttpGatewayService {
    /// Create a new service instance bound to the given address and port
    pub fn new(address: impl Into<IpAddr>, port: u16) -> Self {
        let address = SocketAddr::new(address.into(), port);

        let router = Router::new()
            .route("/health", get(health_check))
            .with_state(AppState {});

        HcHttpGatewayService { router, address }
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
