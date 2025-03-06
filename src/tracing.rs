//! Tracing setup and configuration

use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_subscriber::{layer::SubscriberExt, EnvFilter, Registry};

const DEFAULT_LEVEL: &str = "info";

/// Initialize a global tracing subscriber with Bunyan JSON formatting.
pub fn initialize_tracing_subscriber() {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_LEVEL));
    let formatting_layer =
        BunyanFormattingLayer::new(env!("CARGO_PKG_NAME").into(), std::io::stdout);
    let subscriber = Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer);
    tracing::subscriber::set_global_default(subscriber).ok();
}
