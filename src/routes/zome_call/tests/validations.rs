use crate::test::router::TestRouter;
use crate::test::test_tracing::initialize_testing_tracing_subscriber;
use crate::{
    config::{AllowedFns, Configuration},
    routes::zome_call::MAX_IDENTIFIER_CHARS,
};
use base64::{prelude::BASE64_URL_SAFE, Engine};
use reqwest::StatusCode;
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};

// DnaHash::from_raw_32(vec![1; 32]).to_string()
const DNA_HASH: &str = "uhC0kAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQF-z86-";

#[tokio::test]
async fn valid_dna_hash_is_accepted() {
    initialize_testing_tracing_subscriber();

    let router = TestRouter::new();
    let uri = format!("/{DNA_HASH}/coordinator/zome_name/fn_name");
    let (status_code, _) = router.request(&uri).await;
    assert_eq!(status_code, StatusCode::OK);
}

#[tokio::test]
async fn invalid_dna_hash_is_rejected() {
    initialize_testing_tracing_subscriber();

    let router = TestRouter::new();
    let invalid_dna_hash = "thisaintnodnahash";
    let uri = format!("/{invalid_dna_hash}/coordinator/zome_name/fn_name");
    let (status_code, body) = router.request(&uri).await;
    assert_eq!(status_code, StatusCode::BAD_REQUEST);
    assert_eq!(
        body,
        r#"{"error":"Request is malformed: Invalid DNA hash"}"#
    );
}

#[tokio::test]
async fn coordinator_identifier_with_excess_length_is_rejected() {
    initialize_testing_tracing_subscriber();

    let router = TestRouter::new();
    let coordinator = "12345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901";
    let uri = format!("/{DNA_HASH}/{coordinator}/zome_name/fn_name");
    let (status_code, body) = router.request(&uri).await;
    assert_eq!(status_code, StatusCode::BAD_REQUEST);
    assert_eq!(
        body,
        format!(
            r#"{{"error":"Request is malformed: Identifier {coordinator} longer than {MAX_IDENTIFIER_CHARS} characters"}}"#
        )
    );
}

#[tokio::test]
async fn zome_name_with_excess_length_is_rejected() {
    initialize_testing_tracing_subscriber();

    let router = TestRouter::new();
    let zome_name = "12345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901";
    let uri = format!("/{DNA_HASH}/coordinator/{zome_name}/fn_name");
    let (status_code, body) = router.request(&uri).await;
    assert_eq!(status_code, StatusCode::BAD_REQUEST);
    assert_eq!(
        body,
        format!(
            r#"{{"error":"Request is malformed: Identifier {zome_name} longer than {MAX_IDENTIFIER_CHARS} characters"}}"#
        )
    );
}

#[tokio::test]
async fn function_name_with_excess_length_is_rejected() {
    initialize_testing_tracing_subscriber();

    let router = TestRouter::new();
    let fn_name = "12345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901";
    let uri = format!("/{DNA_HASH}/coordinator/zome_name/{fn_name}");
    let (status_code, body) = router.request(&uri).await;
    assert_eq!(status_code, StatusCode::BAD_REQUEST);
    assert_eq!(
        body,
        format!(
            r#"{{"error":"Request is malformed: Identifier {fn_name} longer than {MAX_IDENTIFIER_CHARS} characters"}}"#
        )
    );
}

#[tokio::test]
async fn unauthorized_function_name_is_rejected() {
    initialize_testing_tracing_subscriber();

    // Only one allowed function "fn_name" in test router.
    let router = TestRouter::new();
    let fn_name = "unauthorized_fn";
    let uri = format!("/{DNA_HASH}/coordinator/zome_name/{fn_name}");
    let (status_code, body) = router.request(&uri).await;
    assert_eq!(status_code, StatusCode::FORBIDDEN);
    assert_eq!(
        body,
        format!(
            r#"{{"error":"Function {fn_name} in zome zome_name in app coordinator is not allowed"}}"#
        )
    );
}

#[tokio::test]
async fn payload_with_excess_length_is_rejected() {
    initialize_testing_tracing_subscriber();

    let mut allowed_fns = HashMap::new();
    allowed_fns.insert("coordinator".to_string(), AllowedFns::All);

    let config = Configuration::try_new(
        SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 8888),
        "10",
        "",
        allowed_fns,
        "",
        "",
    )
    .unwrap();
    let router = TestRouter::new_with_config(config);
    let payload = BASE64_URL_SAFE.encode(vec![1; 11]);
    let uri = format!("/{DNA_HASH}/coordinator/zome_name/fn_name?payload={payload}");
    let (status_code, body) = router.request(&uri).await;
    assert_eq!(status_code, StatusCode::BAD_REQUEST);
    assert_eq!(
        body,
        format!(r#"{{"error":"Request is malformed: Payload exceeds 10 bytes"}}"#)
    );
}

#[tokio::test]
async fn payload_with_invalid_base64_encoding_is_rejected() {
    initialize_testing_tracing_subscriber();

    let router = TestRouter::new();
    let payload = "$%&#";
    let uri = format!("/{DNA_HASH}/coordinator/zome_name/fn_name?payload={payload}");
    let (status_code, body) = router.request(&uri).await;
    assert_eq!(status_code, StatusCode::BAD_REQUEST);
    assert_eq!(
        body,
        r#"{"error":"Request is malformed: Invalid base64 encoding"}"#
    );
}

#[tokio::test]
async fn payload_with_invalid_json_is_rejected() {
    initialize_testing_tracing_subscriber();

    let router = TestRouter::new();
    let payload = BASE64_URL_SAFE.encode("{invalid}");
    let uri = format!("/{DNA_HASH}/coordinator/zome_name/fn_name?payload={payload}");
    let (status_code, body) = router.request(&uri).await;
    assert_eq!(status_code, StatusCode::BAD_REQUEST);
    assert_eq!(
        body,
        r#"{"error":"Request is malformed: Invalid JSON value"}"#
    );
}
