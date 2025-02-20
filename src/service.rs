use std::net::{IpAddr, SocketAddr};

use axum::{routing::get, Router};
use tokio::net::TcpListener;

use crate::{routes::healthz, tracing::initialize_tracing_subscriber};

#[derive(Debug)]
pub struct HcHttpGatewayService {
    address: SocketAddr,
    router: Router,
}

#[derive(Debug, Clone)]
pub struct AppState {}

impl HcHttpGatewayService {
    pub fn new(address: impl Into<IpAddr>, port: u16) -> Self {
        let address = SocketAddr::new(address.into(), port);

        let router = Router::new()
            .route("/healthz", get(healthz))
            .with_state(AppState {});

        HcHttpGatewayService { router, address }
    }

    pub fn address(&self) -> SocketAddr {
        self.address
    }

    pub async fn run(self) -> anyhow::Result<()> {
        initialize_tracing_subscriber("info");

        let address = self.address();
        let listener = TcpListener::bind(self.address).await?;

        tracing::info!("Starting server on {}", address);
        axum::serve(listener, self.router)
            .await
            .inspect_err(|e| tracing::error!("Failed to bind to {}: {}", address, e))?;

        Ok(())
    }
}
