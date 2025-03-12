use anyhow::Context;
use clap::Parser;
use holochain_http_gateway::{
    config::{AllowedAppIds, AllowedFns, Configuration},
    resolve_address_from_url,
    AdminConn, AppConnPool, HcHttpGatewayArgs, HcHttpGatewayService,
};
use std::sync::Arc;
use std::{collections::HashMap, env, str::FromStr};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan, time::UtcTime},
    layer::SubscriberExt,
    EnvFilter, Registry,
};

const DEFAULT_LOG_LEVEL: &str = "info";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    initialize_tracing_subscriber()?;

    let configuration = load_config_from_env().await?;

    let args = HcHttpGatewayArgs::parse();

    let admin_call = Arc::new(AdminConn::new(configuration.admin_socket_addr));
    let app_call = Arc::new(AppConnPool::new(configuration.clone(), admin_call.clone()));

    let service =
        HcHttpGatewayService::new(args.address, args.port, configuration, admin_call, app_call)
            .await?;

    service.run().await?;

    Ok(())
}

async fn load_config_from_env() -> anyhow::Result<Configuration> {
    let admin_ws_url = env::var("HC_GW_ADMIN_WS_URL").context("HC_GW_ADMIN_WS_URL is not set")?;
    let admin_socket_addr = resolve_address_from_url(&admin_ws_url)
        .await
        .context("Failed to extract socket address from the admin websocket URL")?;
    tracing::info!("Resolved admin socket address: {}", admin_socket_addr);

    let payload_limit_bytes = env::var("HC_GW_PAYLOAD_LIMIT_BYTES").unwrap_or_default();

    let allowed_app_ids = env::var("HC_GW_ALLOWED_APP_IDS").unwrap_or_default();

    let mut allowed_fns = HashMap::new();

    let app_ids = AllowedAppIds::from_str(&allowed_app_ids)?;
    for app_id in app_ids.iter() {
        let fns = env::var(format!("HC_GW_ALLOWED_FNS_{}", app_id))
            .context(format!("Missing HC_GW_ALLOWED_FNS_{} env var", app_id))?;
        let fns = AllowedFns::from_str(&fns)?;
        allowed_fns.insert(app_id.to_owned(), fns);
    }

    let max_app_connections = env::var("HC_GW_MAX_APP_CONNECTIONS").unwrap_or_default();

    let zome_call_timeout = env::var("HC_GW_ZOME_CALL_TIMEOUT_MS").unwrap_or_default();

    let config = Configuration::try_new(
        admin_socket_addr,
        &payload_limit_bytes,
        &allowed_app_ids,
        allowed_fns,
        &max_app_connections,
        &zome_call_timeout,
    )?;

    Ok(config)
}

/// Initialize a global tracing subscriber
pub fn initialize_tracing_subscriber() -> Result<(), tracing::subscriber::SetGlobalDefaultError> {
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(DEFAULT_LOG_LEVEL));
    let formatting_layer = fmt::layer()
        .with_timer(UtcTime::rfc_3339())
        .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
        .with_file(true)
        .with_line_number(true);

    let subscriber = Registry::default().with(env_filter).with(formatting_layer);

    tracing::subscriber::set_global_default(subscriber)
}
