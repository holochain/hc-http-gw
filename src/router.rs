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
    use crate::app_selection::tests::new_fake_app_info;
    use crate::MockAppCall;
    use crate::{
        config::{AllowedFns, Configuration, ZomeFn},
        router::hc_http_gateway_router,
        AdminCall, AppCall, MockAdminCall,
    };
    use axum::{body::Body, http::Request, Router};
    use holochain::core::DnaHash;
    use holochain_client::ExternIO;
    use http_body_util::BodyExt;
    use reqwest::StatusCode;
    use std::collections::{HashMap, HashSet};
    use std::net::{Ipv4Addr, SocketAddr};
    use std::sync::Arc;
    use tower::ServiceExt;

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
                    let app_info =
                        new_fake_app_info("coordinator", DnaHash::from_raw_32(vec![1; 32]));
                    Ok(vec![app_info])
                })
            });
            let admin_call = Arc::new(admin_call);
            let mut app_call = MockAppCall::new();
            app_call
                .expect_handle_zome_call()
                .returning(|_, _, _, _, _| {
                    Box::pin(async move { Ok(ExternIO::encode(()).unwrap()) })
                });
            let app_call = Arc::new(app_call);
            Self::new_with_config_and_interfaces(config, admin_call, app_call)
        }

        pub fn new_with_config_and_interfaces(
            config: Configuration,
            admin_call: Arc<dyn AdminCall>,
            app_call: Arc<dyn AppCall>,
        ) -> Self {
            Self(hc_http_gateway_router(config, admin_call, app_call))
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
