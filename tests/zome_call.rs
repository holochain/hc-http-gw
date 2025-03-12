use crate::sweet::install_fixture1;
use base64::{prelude::BASE64_URL_SAFE, Engine};
use holochain::sweettest::SweetConductor;
use holochain_conductor_api::CellInfo;
use holochain_http_gateway::tracing::initialize_tracing_subscriber;
use reqwest::StatusCode;
use setup::TestApp;

mod setup;
mod sweet;

#[tokio::test(flavor = "multi_thread")]
async fn zome_call_with_valid_params() {
    initialize_tracing_subscriber();

    let sweet_conductor = SweetConductor::from_standard_config().await;

    let app = install_fixture1(sweet_conductor.clone(), None)
        .await
        .unwrap();

    let app_info = sweet_conductor
        .list_apps(None)
        .await
        .unwrap()
        .into_iter()
        .find(|a| &a.installed_app_id == app.id())
        .unwrap();

    let cell_id = match app_info
        .cell_info
        .values()
        .next()
        .unwrap()
        .iter()
        .next()
        .unwrap()
    {
        CellInfo::Provisioned(provisioned) => provisioned.cell_id.clone(),
        _ => panic!("Expected a provisioned cell"),
    };

    let app = TestApp::spawn(sweet_conductor.clone()).await;

    let payload = r#"null"#;
    let payload = BASE64_URL_SAFE.encode(payload);

    let response = app
        .call_zome(
            cell_id.dna_hash(),
            "fixture1",
            "coordinator1",
            "get_all_1",
            Some(&payload),
        )
        .await;
    assert_eq!(response.status(), StatusCode::OK);
}
