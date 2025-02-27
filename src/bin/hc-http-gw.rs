use std::{collections::HashMap, env, str::FromStr};

use anyhow::Context;
use clap::Parser;
use holochain_http_gateway::{
    config::{AllowedAppIds, AllowedFns, Configuration, PayloadLimitBytes},
    tracing::initialize_tracing_subscriber,
    HcHttpGatewayArgs, HcHttpGatewayService,
};
use url2::Url2;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    initialize_tracing_subscriber("info");

    let configuration = load_config_from_env()?;

    let args = HcHttpGatewayArgs::parse();
    let service = HcHttpGatewayService::new(args.address, args.port, configuration);

    service.run().await?;

    Ok(())
}

fn load_config_from_env() -> anyhow::Result<Configuration> {
    let admin_ws_url = env::var("HC_GW_ADMIN_WS_URL").context("HC_GW_ADMIN_WS_URL is not set")?;
    let admin_ws_url = Url2::try_parse(admin_ws_url)?;

    let payload_limit_bytes =
        PayloadLimitBytes::from_str(&env::var("HC_GW_PAYLOAD_LIMIT_BYTES").unwrap_or_default())?;

    let allowed_app_ids =
        AllowedAppIds::from_str(&env::var("HC_GW_ALLOWED_APP_IDS").unwrap_or_default())?;

    let mut allowed_fns = HashMap::new();
    for app_id in allowed_app_ids.iter() {
        let fns = env::var(format!("HC_GW_ALLOWED_FNS_{}", app_id))
            .context(format!("Missing HC_GW_ALLOWED_FNS_{} env var", app_id))?;
        let fns = AllowedFns::from_str(&fns)?;
        allowed_fns.insert(app_id.to_owned(), fns);
    }

    Ok(Configuration {
        admin_ws_url,
        payload_limit_bytes,
        allowed_app_ids,
        allowed_fns,
    })
}
