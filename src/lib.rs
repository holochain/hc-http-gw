#![deny(missing_docs)]
//! # holochain http gateway

mod cli;
mod error;
mod routes;
mod service;
mod tracing;

pub use cli::HcHttpGatewayArgs;
pub use error::HcHttpGatewayError;
pub use service::HcHttpGatewayService;
