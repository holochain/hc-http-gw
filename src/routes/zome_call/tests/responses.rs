use crate::{
    app_selection::tests::new_fake_app_info,
    config::{AllowedFns, Configuration},
    router::tests::TestRouter,
};
use crate::{MockAdminCall, MockAppCall};
use holochain_client::ExternIO;
use holochain_types::prelude::DnaHash;
use reqwest::StatusCode;
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;

#[tokio::test(flavor = "multi_thread")]
async fn happy_zome_call() {
    let app_id = "tapp";
    let dna_hash = DnaHash::from_raw_32(vec![1; 32]);

    let mut allowed_fns = HashMap::new();
    allowed_fns.insert(app_id.into(), AllowedFns::All);
    let config = Configuration::try_new(
        SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 8888),
        "1024",
        app_id,
        allowed_fns,
        "",
        "",
    )
    .unwrap();

    let mut admin_call = MockAdminCall::new();
    let dna_hash2 = dna_hash.clone();
    admin_call.expect_list_apps().returning(move |_| {
        let dna_hash = dna_hash2.clone();
        Box::pin(async move {
            let app_info = new_fake_app_info(app_id, dna_hash);
            Ok(vec![app_info])
        })
    });
    let admin_call = Arc::new(admin_call);
    let mut app_call = MockAppCall::new();
    app_call.expect_handle_zome_call().returning(|_, _, _, _| {
        Box::pin(async move { Ok(ExternIO::encode("return_value").unwrap()) })
    });
    let app_call = Arc::new(app_call);
    let router = TestRouter::new_with_config_and_interfaces(config, admin_call, app_call);
    let (status_code, body) = router
        .request(&format!("/{dna_hash}/{app_id}/coordinator/fn_name"))
        .await;
    assert_eq!(status_code, StatusCode::OK);
    assert_eq!(body, r#""return_value""#);
}
