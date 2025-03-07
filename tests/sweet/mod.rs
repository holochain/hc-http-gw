use holochain::conductor::error::ConductorResult;
use holochain::conductor::Conductor;
use holochain_client::SerializedBytes;
use holochain_types::app::{AppBundleSource, InstallAppPayload};
use std::path::PathBuf;
use std::sync::Arc;
// TODO `SerializedBytes` has an unclean macro reference to `holochain_serial!`
use holochain_serialized_bytes::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, SerializedBytes)]
pub struct TestType {
    pub value: String,
}

pub async fn install_fixture1(conductor: Arc<Conductor>) -> ConductorResult<()> {
    let mut happ_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    happ_path.push("fixture/package/happ1/fixture1.happ");

    install_app_from_path(conductor.clone(), happ_path).await
}

pub async fn install_fixture2(conductor: Arc<Conductor>) -> ConductorResult<()> {
    let mut happ_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    happ_path.push("fixture/package/happ2/fixture2.happ");

    install_app_from_path(conductor.clone(), happ_path).await
}

async fn install_app_from_path(
    conductor: Arc<Conductor>,
    happ_path: PathBuf,
) -> ConductorResult<()> {
    let app = conductor
        .clone()
        .install_app_bundle(InstallAppPayload {
            source: AppBundleSource::Path(happ_path),
            agent_key: None,
            installed_app_id: None,
            network_seed: None,
            roles_settings: None,
            ignore_genesis_failure: false,
            allow_throwaway_random_agent_key: true,
        })
        .await?;

    conductor.enable_app(app.installed_app_id.clone()).await?;

    Ok(())
}
