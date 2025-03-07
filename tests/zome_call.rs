pub mod setup;

use base64::{prelude::BASE64_URL_SAFE, Engine};
use holochain::core::DnaHash;
use holochain_http_gateway::{
    config::{AllowedFns, Configuration},
    tracing::initialize_tracing_subscriber,
};
use reqwest::StatusCode;

use setup::TestApp;

#[tokio::test]
async fn zome_call_with_valid_params() {
    initialize_tracing_subscriber();

    let app = TestApp::spawn().await;

    let dna_hash = DnaHash::from_raw_32(vec![1; 32]).to_string();
    let payload = r#"{"limit": 100, "offset": 10}"#;
    let payload = BASE64_URL_SAFE.encode(payload);

    let response = app
        .call_zome(
            &dna_hash,
            "forum",
            "custom_zome",
            "special_function",
            Some(&payload),
        )
        .await;
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn zome_call_with_valid_params_but_no_payload() {
    initialize_tracing_subscriber();

    let app = TestApp::spawn().await;

    let dna_hash = DnaHash::from_raw_32(vec![1; 32]).to_string();

    let response = app
        .call_zome(&dna_hash, "forum", "custom_zome", "special_function", None)
        .await;

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn zome_call_with_payload_exceeding_limit_fails() {
    initialize_tracing_subscriber();

    let mut allowed_fns = std::collections::HashMap::new();
    allowed_fns.insert("forum".to_string(), AllowedFns::All);

    // Custom configuration with a very small payload limit
    let config =
        Configuration::try_new("ws://localhost:50350", "10", "forum", allowed_fns, "").unwrap();

    let app = TestApp::spawn_with_config(config).await;

    let dna_hash = DnaHash::from_raw_32(vec![1; 32]).to_string();
    let large_payload = r#"{"limit":100,"offset":0,"filters":{"author":"user123","tags":["important","featured","latest"],"content_contains":"search term","date_range":{"from":"2023-01-01","to":"2023-12-31"}}"#;
    let large_payload = BASE64_URL_SAFE.encode(large_payload);

    let response = app
        .call_zome(
            &dna_hash,
            "forum",
            "custom_zome",
            "special_function",
            Some(&large_payload),
        )
        .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn zome_call_with_invalid_json_payload_fails() {
    initialize_tracing_subscriber();

    let app = TestApp::spawn().await;

    // Invalid JSON payload
    let dna_hash = DnaHash::from_raw_32(vec![1; 32]).to_string();
    let invalid_payload = r#"{"limit":10, offset: 0,}"#;
    let invalid_payload = BASE64_URL_SAFE.encode(invalid_payload);

    let response = app
        .call_zome(
            &dna_hash,
            "forum",
            "custom_zome",
            "special_function",
            Some(&invalid_payload),
        )
        .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn zome_call_with_invalid_dna_hash_fails() {
    initialize_tracing_subscriber();

    let app = TestApp::spawn().await;

    // Invalid DNA hash
    let dna_hash = "not-a-dna-hash";
    let payload = r#"{"limit":10}"#;
    let payload = BASE64_URL_SAFE.encode(payload);

    let response = app
        .call_zome(
            dna_hash,
            "forum",
            "custom_zome",
            "special_function",
            Some(&payload),
        )
        .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn zome_call_with_non_base64_encoded_payload_fails() {
    initialize_tracing_subscriber();

    let app = TestApp::spawn().await;

    let dna_hash = DnaHash::from_raw_32(vec![1; 32]).to_string();
    // Sending a raw JSON string without base64 encoding
    let payload = r#"{"limit":10}"#;

    let response = app
        .call_zome(
            &dna_hash,
            "forum",
            "custom_zome",
            "special_function",
            Some(payload),
        )
        .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
