#![deny(missing_docs)]
//! # Holochain HTTP gateway

mod cli;
pub mod config;
mod error;
mod http;
mod router;
mod routes;
mod service;
pub mod tracing;
pub mod transcode;

pub use cli::HcHttpGatewayArgs;
pub use error::{HcHttpGatewayError, HcHttpGatewayResult};
pub use service::HcHttpGatewayService;
