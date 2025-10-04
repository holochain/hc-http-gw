#![allow(dead_code)]

use holochain::conductor::api::error::ConductorApiResult;
use holochain::conductor::error::ConductorResult;
use holochain::conductor::Conductor;
use holochain::prelude::InitCallbackResult;
use holochain_types::app::{AppBundleSource, InstallAppPayload, InstalledApp, InstalledAppId};
use std::path::PathBuf;
use std::sync::Arc;
// TODO `SerializedBytes` has an unclean macro reference to `holochain_serial!`
use holochain_serialized_bytes::prelude::*;
use holochain_types::prelude::ActionHashB64;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, SerializedBytes)]
pub struct TestType {
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateResponse {
    pub created: ActionHashB64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetWithLimitRequest {
    pub limit: usize,
}

pub async fn install_fixture1(
    conductor: Arc<Conductor>,
    installed_app_id: Option<InstalledAppId>,
) -> ConductorResult<InstalledApp> {
    let mut happ_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    happ_path.push("fixture/package/happ1/fixture1.happ");

    install_app_from_path(conductor.clone(), happ_path, installed_app_id).await
}

pub async fn install_fixture2(
    conductor: Arc<Conductor>,
    installed_app_id: Option<InstalledAppId>,
) -> ConductorResult<InstalledApp> {
    let mut happ_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    happ_path.push("fixture/package/happ2/fixture2.happ");

    install_app_from_path(conductor.clone(), happ_path, installed_app_id).await
}

pub async fn init_zome(
    conductor: Arc<Conductor>,
    app: &InstalledApp,
    zome_name: String,
) -> ConductorApiResult<()> {
    conductor
        .easy_call_zome::<_, InitCallbackResult, _>(
            &app.agent_key,
            None,
            app.all_cells().next().unwrap(),
            zome_name,
            "init",
            (),
        )
        .await?;

    Ok(())
}

async fn install_app_from_path(
    conductor: Arc<Conductor>,
    happ_path: PathBuf,
    installed_app_id: Option<InstalledAppId>,
) -> ConductorResult<InstalledApp> {
    let app = conductor
        .clone()
        .install_app_bundle(InstallAppPayload {
            source: AppBundleSource::Path(happ_path),
            agent_key: None,
            installed_app_id,
            network_seed: None,
            roles_settings: None,
            ignore_genesis_failure: false,
        })
        .await?;

    conductor.enable_app(app.installed_app_id.clone()).await?;

    Ok(app)
}
