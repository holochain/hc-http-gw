//! Tracing setup and configuration

use tracing_subscriber::{
    fmt::{self, time::UtcTime},
    layer::SubscriberExt,
    EnvFilter, Registry,
};

const DEFAULT_LOG_LEVEL: &str = "warn,holochain_http_gateway=info";

/// Initialize a global tracing subscriber
pub fn initialize_tracing_subscriber() {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_LOG_LEVEL));
    let formatting_layer = fmt::layer()
        .pretty()
        .with_timer(UtcTime::rfc_3339())
        .with_file(true)
        .with_line_number(true);

    let subscriber = Registry::default().with(env_filter).with(formatting_layer);

    tracing::subscriber::set_global_default(subscriber).ok();
}
