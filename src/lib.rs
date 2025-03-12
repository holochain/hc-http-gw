#![deny(missing_docs)]
//! # Holochain HTTP gateway

mod app_selection;
mod cli;
pub mod config;
mod error;
mod holochain;
mod resolve;
mod router;
mod routes;
mod service;
pub mod transcode;

#[cfg(any(test, feature = "test-utils"))]
pub mod test_tracing;

pub use cli::HcHttpGatewayArgs;
pub use error::{HcHttpGatewayError, HcHttpGatewayResult};
pub use holochain::*;
pub use resolve::resolve_address_from_url;
pub use service::HcHttpGatewayService;
