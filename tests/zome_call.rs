mod setup;

use base64::prelude::*;
use fixt::fixt;
use holochain_http_gateway::{
    config::{AllowedFns, Configuration},
    tracing::initialize_tracing_subscriber,
};
use holochain_types::dna::fixt::DnaHashFixturator;
use reqwest::StatusCode;

use setup::TestApp;

#[tokio::test]
async fn zome_call_uses_correct_route_parameters() {
    initialize_tracing_subscriber();

    let app = TestApp::spawn().await;

    let dna_hash = fixt!(DnaHash);
    let coordinator = "coord98765";
    let zome = "custom_zome";
    let function = "special_function";

    let payload = r#"{"limit": 100, "offset": 10}"#;
    let encoded_payload = BASE64_URL_SAFE.encode(payload);

    let response = app
        .client
        .get(format!(
            "http://{}/{dna_hash}/{coordinator}/{zome}/{function}?payload={encoded_payload}",
            app.address,
        ))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn zome_call_with_payload_exceeding_limit_fails() {
    initialize_tracing_subscriber();

    let mut allowed_fns = std::collections::HashMap::new();
    allowed_fns.insert("forum".to_string(), AllowedFns::All);

    // Custom configuration with a very small payload limit
    let config =
        Configuration::try_new("ws://localhost:50350", "10", "forum", allowed_fns).unwrap();

    let app = TestApp::spawn_with_config(config).await;

    let large_payload = r#"{"limit":100,"offset":0,"filters":{"author":"user123","tags":["important","featured","latest"],"content_contains":"search term","date_range":{"from":"2023-01-01","to":"2023-12-31"}}"#;
    let encoded_payload = BASE64_URL_SAFE.encode(large_payload);

    let dna_hash = fixt!(DnaHash);
    let coordinator = "coord98765";
    let zome = "custom_zome";
    let function = "special_function";

    let response = app
        .client
        .get(format!(
            "http://{}/{dna_hash}/{coordinator}/{zome}/{function}?payload={encoded_payload}",
            app.address,
        ))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn zome_call_with_invalid_json_payload() {
    initialize_tracing_subscriber();

    let app = TestApp::spawn().await;

    // Invalid JSON payload with a comma
    let small_payload = r#"{"limit":10,}"#;
    let encoded_payload = BASE64_URL_SAFE.encode(small_payload);

    let dna_hash = fixt!(DnaHash);
    let coordinator = "coord98765";
    let zome = "custom_zome";
    let function = "special_function";

    let response = app
        .client
        .get(format!(
            "http://{}/{dna_hash}/{coordinator}/{zome}/{function}?payload={encoded_payload}",
            app.address
        ))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn zome_call_with_invalid_dna_hash() {
    initialize_tracing_subscriber();

    let app = TestApp::spawn().await;

    let payload = r#"{"limit":10}"#;
    let encoded_payload = BASE64_URL_SAFE.encode(payload);

    // Invalid DNA hash
    let dna_hash = "not-a-dna-hash";
    let coordinator = "coord98765";
    let zome = "custom_zome";
    let function = "special_function";

    let response = app
        .client
        .get(format!(
            "http://{}/{dna_hash}/{coordinator}/{zome}/{function}?payload={encoded_payload}",
            app.address,
        ))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
