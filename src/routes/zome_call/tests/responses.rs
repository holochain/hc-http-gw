use super::DNA_HASH;
use crate::config::{AllowedFns, Configuration};
use crate::test::data::new_test_app_info;
use crate::test::router::TestRouter;
use crate::{MockAdminCall, MockAppCall};
use holochain::holochain_wasmer_host::prelude::WasmErrorInner;
use holochain_client::{ConductorApiError, ExternIO};
use holochain_conductor_api::ExternalApiWireError;
use holochain_types::prelude::DnaHash;
use reqwest::StatusCode;
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;

const APP_ID: &str = "tapp";

fn create_test_router(app_call: MockAppCall) -> TestRouter {
    let mut allowed_fns = HashMap::new();
    allowed_fns.insert(APP_ID.into(), AllowedFns::All);
    let config = Configuration::try_new(
        SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 8888),
        "1024",
        APP_ID,
        allowed_fns,
        "",
        "",
    )
    .unwrap();

    let mut admin_call = MockAdminCall::new();
    admin_call.expect_list_apps().returning(move |_| {
        Box::pin(async move {
            let app_info = new_test_app_info(APP_ID, DnaHash::from_raw_32(vec![1; 32]));
            Ok(vec![app_info])
        })
    });
    let admin_call = Arc::new(admin_call);
    let app_call = Arc::new(app_call);
    TestRouter::new_with_config_and_interfaces(config, admin_call, app_call)
}

#[tokio::test]
async fn happy_zome_call() {
    let mut app_call = MockAppCall::new();
    app_call
        .expect_handle_zome_call()
        .returning(|_, _, _, _, _| {
            Box::pin(async move { Ok(ExternIO::encode("return_value").unwrap()) })
        });
    let router = create_test_router(app_call);
    let (status_code, body) = router
        .request(&format!("/{DNA_HASH}/{APP_ID}/coordinator/fn_name"))
        .await;
    assert_eq!(status_code, StatusCode::OK);
    assert_eq!(body, r#""return_value""#);
}

#[tokio::test]
async fn ribosome_errors_are_returned() {
    let mut app_call = MockAppCall::new();
    app_call
        .expect_handle_zome_call()
        .returning(|_, _, _, _, _| {
            Box::pin(async move {
                // A bit contrived this error, but close enough to reality.
                Err(crate::HcHttpGatewayError::HolochainError(
                    ConductorApiError::ExternalApiWireError(ExternalApiWireError::RibosomeError(
                        format!(
                            "{:?}",
                            WasmErrorInner::Guest("could not find record xyz".to_string())
                        ),
                    )),
                ))
            })
        });
    let router = create_test_router(app_call);
    let (status_code, body) = router
        .request(&format!("/{DNA_HASH}/{APP_ID}/coordinator/fn_name"))
        .await;
    assert_eq!(status_code, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(body, r#"{"error":"Guest(\"could not find record xyz\")"}"#);
}

#[tokio::test]
async fn app_not_found() {
    let mut app_call = MockAppCall::new();
    app_call
        .expect_handle_zome_call()
        .returning(|_, _, _, _, _| {
            Box::pin(async move {
                Err(crate::HcHttpGatewayError::HolochainError(
                    ConductorApiError::AppNotFound,
                ))
            })
        });
    let router = create_test_router(app_call);
    let (status_code, body) = router
        .request(&format!("/{DNA_HASH}/{APP_ID}/coordinator/fn_name"))
        .await;
    // The app must have been found earlier when looking it up for the call,
    // so this must have been an internal error of some kind.
    assert_eq!(status_code, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(body, r#"{"error":"Something went wrong"}"#);
}

#[tokio::test]
async fn cell_not_found() {
    let mut app_call = MockAppCall::new();
    app_call
        .expect_handle_zome_call()
        .returning(|_, _, _, _, _| {
            Box::pin(async move {
                Err(crate::HcHttpGatewayError::HolochainError(
                    ConductorApiError::CellNotFound,
                ))
            })
        });
    let router = create_test_router(app_call);
    let (status_code, body) = router
        .request(&format!("/{DNA_HASH}/{APP_ID}/coordinator/fn_name"))
        .await;
    assert_eq!(status_code, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(body, r#"{"error":"Something went wrong"}"#);
}

#[tokio::test]
async fn other_external_api_wire_error() {
    let mut app_call = MockAppCall::new();
    app_call
        .expect_handle_zome_call()
        .returning(|_, _, _, _, _| {
            Box::pin(async move {
                Err(crate::HcHttpGatewayError::HolochainError(
                    ConductorApiError::ExternalApiWireError(
                        ExternalApiWireError::ZomeCallUnauthorized("unauthorized".to_string()),
                    ),
                ))
            })
        });
    let router = create_test_router(app_call);
    let (status_code, body) = router
        .request(&format!("/{DNA_HASH}/{APP_ID}/coordinator/fn_name"))
        .await;
    assert_eq!(status_code, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(body, r#"{"error":"Something went wrong"}"#);
}

#[tokio::test]
async fn fresh_nonce_error() {
    let mut app_call = MockAppCall::new();
    app_call
        .expect_handle_zome_call()
        .returning(|_, _, _, _, _| {
            Box::pin(async move {
                Err(crate::HcHttpGatewayError::HolochainError(
                    ConductorApiError::FreshNonceError("nonce_kaputt".into()),
                ))
            })
        });
    let router = create_test_router(app_call);
    let (status_code, body) = router
        .request(&format!("/{DNA_HASH}/{APP_ID}/coordinator/fn_name"))
        .await;
    assert_eq!(status_code, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(body, r#"{"error":"Something went wrong"}"#);
}

#[tokio::test]
async fn io_error() {
    let mut app_call = MockAppCall::new();
    app_call
        .expect_handle_zome_call()
        .returning(|_, _, _, _, _| {
            Box::pin(async move {
                Err(crate::HcHttpGatewayError::HolochainError(
                    ConductorApiError::IoError(std::io::Error::other("ssd not found")),
                ))
            })
        });
    let router = create_test_router(app_call);
    let (status_code, body) = router
        .request(&format!("/{DNA_HASH}/{APP_ID}/coordinator/fn_name"))
        .await;
    assert_eq!(status_code, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(body, r#"{"error":"Something went wrong"}"#);
}

#[tokio::test]
async fn sign_zome_call_error() {
    let mut app_call = MockAppCall::new();
    app_call
        .expect_handle_zome_call()
        .returning(|_, _, _, _, _| {
            Box::pin(async move {
                Err(crate::HcHttpGatewayError::HolochainError(
                    ConductorApiError::SignZomeCallError("unsigned".to_string()),
                ))
            })
        });
    let router = create_test_router(app_call);
    let (status_code, body) = router
        .request(&format!("/{DNA_HASH}/{APP_ID}/coordinator/fn_name"))
        .await;
    assert_eq!(status_code, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(body, r#"{"error":"Something went wrong"}"#);
}

#[tokio::test]
async fn websocket_error() {
    let mut app_call = MockAppCall::new();
    app_call
        .expect_handle_zome_call()
        .returning(|_, _, _, _, _| {
            Box::pin(async move {
                Err(crate::HcHttpGatewayError::HolochainError(
                    ConductorApiError::WebsocketError(
                        // WebsocketError is not exposed.
                        std::io::Error::other("websocket closed").into(),
                    ),
                ))
            })
        });
    let router = create_test_router(app_call);
    let (status_code, body) = router
        .request(&format!("/{DNA_HASH}/{APP_ID}/coordinator/fn_name"))
        .await;
    assert_eq!(status_code, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(body, r#"{"error":"Something went wrong"}"#);
}
