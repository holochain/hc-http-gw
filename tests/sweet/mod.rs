use holochain::conductor::Conductor;
use holochain_types::app::{AppBundleSource, InstallAppPayload};
use std::path::PathBuf;
use std::sync::Arc;

pub async fn install_fixture1(conductor: Arc<Conductor>) {
    let mut happ_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    happ_path.push("fixture/package/happ1/fixture1.happ");

    install_app_from_path(conductor.clone(), happ_path).await;
}

pub async fn install_fixture2(conductor: Arc<Conductor>) {
    let mut happ_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    happ_path.push("fixture/package/happ2/fixture2.happ");

    install_app_from_path(conductor.clone(), happ_path).await;
}

async fn install_app_from_path(conductor: Arc<Conductor>, happ_path: PathBuf) {
    conductor
        .install_app_bundle(InstallAppPayload {
            source: AppBundleSource::Path(happ_path),
            agent_key: None,
            installed_app_id: None,
            network_seed: None,
            roles_settings: None,
            ignore_genesis_failure: false,
            allow_throwaway_random_agent_key: true,
        })
        .await
        .unwrap();
}
