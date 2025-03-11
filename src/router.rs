use crate::holochain::AppCall;
use crate::{
    config::Configuration,
    routes::{health_check, zome_call},
    service::AppState,
    AdminCall,
};
use axum::{http::StatusCode, routing::get, Router};
use std::sync::Arc;

pub fn hc_http_gateway_router(
    configuration: Configuration,
    admin_call: Arc<dyn AdminCall>,
    app_call: Arc<dyn AppCall>,
) -> Router {
    let state = AppState {
        configuration,
        admin_call,
        app_call,
    };

    Router::new()
        .route("/health", get(health_check))
        .route(
            "/{dna_hash}/{coordinator_identifier}/{zome_name}/{fn_name}",
            get(zome_call),
        )
        .method_not_allowed_fallback(|| async { (StatusCode::METHOD_NOT_ALLOWED, ()) })
        .with_state(state)
}

#[cfg(test)]
pub mod tests {
    use crate::{
        config::{AllowedFns, Configuration, ZomeFn},
        router::hc_http_gateway_router,
        AdminConn,
    };
    use crate::{HcHttpGatewayError, MockAppCall};
    use axum::{body::Body, http::Request, Router};
    use holochain::prelude::ExternIO;
    use holochain::sweettest::SweetConductor;
    use holochain_client::SerializedBytes;
    use holochain_serialized_bytes::prelude::*;
    use http_body_util::BodyExt;
    use reqwest::StatusCode;
    use serde::{Deserialize, Serialize};
    use std::collections::{HashMap, HashSet};
    use std::sync::Arc;
    use tower::ServiceExt;

    #[derive(Debug, Serialize, Deserialize, SerializedBytes)]
    pub struct TestZomeResponse {
        hello: String,
    }

    pub struct TestRouter(Router);

    impl TestRouter {
        /// Construct a test router with 1024 bytes payload limit.
        /// Allowed functions are restricted to coordinator "coordinator", zome name "zome_name",
        /// function name "fn_name".
        pub async fn new() -> Self {
            let mut allowed_fns = HashMap::new();
            let allowed_zome_fn = ZomeFn {
                zome_name: "zome_name".to_string(),
                fn_name: "fn_name".to_string(),
            };
            let mut allowed_zome_fns = HashSet::new();
            allowed_zome_fns.insert(allowed_zome_fn);
            let restricted_fns = AllowedFns::Restricted(allowed_zome_fns);
            allowed_fns.insert("coordinator".to_string(), restricted_fns);

            let sweet_conductor = SweetConductor::from_standard_config().await;
            let admin_port = sweet_conductor
                .get_arbitrary_admin_websocket_port()
                .unwrap();

            let config = Configuration::try_new(
                format!("ws://127.0.0.1:{admin_port}").as_str(),
                "1024",
                "",
                allowed_fns,
                "",
                "",
            )
            .unwrap();
            Self::new_with_config(config).await
        }

        pub async fn new_with_config(config: Configuration) -> Self {
            let mut app_call = MockAppCall::new();
            app_call.expect_handle_zome_call().returning(|_, _| {
                Box::pin(async {
                    let response = TestZomeResponse {
                        hello: "world".to_string(),
                    };
                    Ok(ExternIO::encode(response)
                        .map_err(|e| HcHttpGatewayError::RequestMalformed(e.to_string()))?)
                })
            });

            let admin_call = AdminConn::connect(&config.admin_ws_url).await.unwrap();

            Self(hc_http_gateway_router(
                config,
                Arc::new(admin_call),
                Arc::new(app_call),
            ))
        }

        // Send request and return status code and body of response.
        pub async fn request(self, uri: &str) -> (StatusCode, String) {
            let response = self
                .0
                .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
                .await
                .unwrap();
            let status_code = response.status();
            let body = String::from_utf8(
                response
                    .into_body()
                    .collect()
                    .await
                    .unwrap()
                    .to_bytes()
                    .to_vec(),
            )
            .unwrap();
            (status_code, body)
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn get_request_to_root_fails() {
        let router = TestRouter::new().await;
        let (status_code, _) = router.request("/").await;
        assert_eq!(status_code, StatusCode::NOT_FOUND);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn post_method_to_health_fails() {
        let router = TestRouter::new().await;
        let response = router
            .0
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn post_method_to_zome_call_fails() {
        let router = TestRouter::new().await;
        let response = router
            .0
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/dna_hash/coodinator/zome_name/fn_name")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }
}
