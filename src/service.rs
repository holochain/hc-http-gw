//! HTTP gateway service for Holochain

use crate::holochain::{AdminCall, AppCall};
use crate::{
    config::Configuration, error::HcHttpGatewayResult, router::hc_http_gateway_router,
    HcHttpGatewayError,
};
use axum::Router;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
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
    #[allow(
        dead_code,
        reason = "This will be used when we start making zome calls"
    )]
    pub admin_call: Arc<dyn AdminCall>,
    #[allow(
        dead_code,
        reason = "This will be used when we start making zome calls"
    )]
    pub app_call: Arc<dyn AppCall>,
}

impl HcHttpGatewayService {
    /// Create a new service instance bound to the given address and port
    pub async fn new(
        address: impl Into<IpAddr>,
        port: u16,
        configuration: Configuration,
        admin_call: Arc<dyn AdminCall>,
        app_call: Arc<dyn AppCall>,
    ) -> HcHttpGatewayResult<Self> {
        tracing::info!("Configuration: {:?}", configuration);

        let router = hc_http_gateway_router(configuration, admin_call, app_call);
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
