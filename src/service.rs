//! HTTP gateway service for Holochain

use std::net::{IpAddr, SocketAddr};

use axum::{middleware, routing::get, Router};
use tokio::net::TcpListener;

use crate::{
    config::Configuration,
    error::HcHttpGatewayResult,
    middleware::validate_zome_call_payload_limit,
    routes::{health_check, zome_call},
    HcHttpGatewayError,
};

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
}

impl HcHttpGatewayService {
    /// Create a new service instance bound to the given address and port
    pub async fn new(
        address: impl Into<IpAddr>,
        port: u16,
        configuration: Configuration,
    ) -> HcHttpGatewayResult<Self> {
        let address = SocketAddr::new(address.into(), port);

        let state = AppState { configuration };

        let zome_call_routes = Router::new()
            .route(
                "/{dna_hash}/{coordinator_identifier}/{zome_name}/{function_name}",
                get(zome_call),
            )
            .layer(middleware::from_fn_with_state(
                state.clone(),
                validate_zome_call_payload_limit,
            ));

        let router = Router::new()
            .merge(zome_call_routes)
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
            .map_err(|e| HcHttpGatewayError::IoError(e))
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
