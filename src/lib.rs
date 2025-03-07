#![deny(missing_docs)]
//! # Holochain HTTP gateway

mod admin_ws;
mod cli;
pub mod config;
mod error;
mod http;
mod routes;
mod service;
pub mod tracing;

pub use admin_ws::ReconnectingAdminWebsocket;
pub use cli::HcHttpGatewayArgs;
pub use error::{HcHttpGatewayError, HcHttpGatewayResult};
pub use service::HcHttpGatewayService;
