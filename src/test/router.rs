//! A test router that can be used to test router handlers with mocked state.

use crate::router::hc_http_gateway_router;
use crate::test::data::new_test_app_info;
use crate::{AdminCall, AllowedFns, AppCall, Configuration, MockAdminCall, MockAppCall, ZomeFn};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use holochain_client::ExternIO;
use holochain_types::prelude::DnaHash;
use http_body_util::BodyExt;
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
                let app_info = new_test_app_info("coordinator", DnaHash::from_raw_32(vec![1; 32]));
                Ok(vec![app_info])
            })
        });
        let admin_call = Arc::new(admin_call);
        let mut app_call = MockAppCall::new();
        app_call
            .expect_handle_zome_call()
            .returning(|_, _, _, _, _| Box::pin(async move { Ok(ExternIO::encode(()).unwrap()) }));
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

impl Default for TestRouter {
    fn default() -> Self {
        Self::new()
    }
}

impl std::ops::Deref for TestRouter {
    type Target = Router;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
