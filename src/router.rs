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
        app_info_cache: Default::default(),
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
    };
    use crate::{HcHttpGatewayError, MockAdminCall, MockAppCall};
    use axum::{body::Body, http::Request, Router};
    use holochain::core::{AgentPubKey, DnaHash};
    use holochain::prelude::ExternIO;
    use holochain::prelude::{CellId, DnaModifiersBuilder};
    use holochain_client::SerializedBytes;
    use holochain_conductor_api::{AppInfo, AppInfoStatus, CellInfo};
    use holochain_serialized_bytes::prelude::*;
    use holochain_types::prelude::{AppManifest, AppManifestV1};
    use http_body_util::BodyExt;
    use reqwest::StatusCode;
    use serde::{Deserialize, Serialize};
    use std::collections::{HashMap, HashSet};
    use std::net::{Ipv4Addr, SocketAddr};
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
        pub fn new() -> Self {
            let mut allowed_fns = HashMap::new();
            let allowed_zome_fn = ZomeFn {
                zome_name: "zome_name".to_string(),
                fn_name: "fn_name".to_string(),
            };
            let mut allowed_zome_fns = HashSet::new();
            allowed_zome_fns.insert(allowed_zome_fn);
            let restricted_fns = AllowedFns::Restricted(allowed_zome_fns);
            allowed_fns.insert("coordinator".to_string(), restricted_fns);

            let config = Configuration::try_new(
                SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 8888),
                "1024",
                "coordinator",
                allowed_fns,
                "",
                "",
            )
            .unwrap();
            Self::new_with_config(config)
        }

        pub fn new_with_config(config: Configuration) -> Self {
            let mut admin_call = MockAdminCall::new();
            admin_call.expect_list_apps().returning(|_| {
                Box::pin(async {
                    let agent_pub_key = AgentPubKey::from_raw_32(vec![17; 32]);
                    Ok(vec![AppInfo {
                        installed_app_id: "coordinator".to_string(),
                        cell_info: [(
                            "test-role".to_string(),
                            vec![CellInfo::new_provisioned(
                                CellId::new(
                                    DnaHash::from_raw_32(vec![1; 32]),
                                    agent_pub_key.clone(),
                                ),
                                DnaModifiersBuilder::default()
                                    .network_seed("".to_string())
                                    .build()
                                    .unwrap(),
                                "test-dna".to_string(),
                            )],
                        )]
                        .into_iter()
                        .collect(),
                        status: AppInfoStatus::Running,
                        agent_pub_key,
                        manifest: AppManifest::V1(AppManifestV1 {
                            name: "coordinator".to_string(),
                            roles: Vec::with_capacity(0),
                            description: None,
                            allow_deferred_memproofs: false,
                        }),
                    }])
                })
            });

            let mut app_call = MockAppCall::new();
            app_call.expect_handle_zome_call().returning(|_, _| {
                Box::pin(async {
                    let response = TestZomeResponse {
                        hello: "world".to_string(),
                    };
                    ExternIO::encode(response)
                        .map_err(|e| HcHttpGatewayError::RequestMalformed(e.to_string()))
                })
            });

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

    #[tokio::test]
    async fn get_request_to_root_fails() {
        let router = TestRouter::new();
        let (status_code, _) = router.request("/").await;
        assert_eq!(status_code, StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn post_method_to_health_fails() {
        let router = TestRouter::new();
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

    #[tokio::test]
    async fn post_method_to_zome_call_fails() {
        let router = TestRouter::new();
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
