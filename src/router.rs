use crate::{
    config::Configuration,
    routes::{health_check, zome_call},
    service::AppState,
};
use axum::{routing::get, Router};

pub fn hc_http_gateway_router(configuration: Configuration) -> Router {
    let state = AppState { configuration };
    Router::new()
        .route(
            "/{dna_hash}/{coordinator_identifier}/{zome_name}/{function_name}",
            get(zome_call),
        )
        .route("/health", get(health_check))
        .with_state(state)
}

#[cfg(test)]
pub mod tests {
    use crate::{config::Configuration, router::hc_http_gateway_router};
    use axum::{body::Body, http::Request, Router};
    use http_body_util::BodyExt;
    use reqwest::StatusCode;
    use std::collections::HashMap;
    use tower::ServiceExt;

    pub struct TestRouter(Router);

    impl TestRouter {
        /// Construct a test router with 1024 bytes payload limit.
        pub fn new() -> Self {
            let config =
                Configuration::try_new("ws://127.0.0.1:1", "1024", "", HashMap::new()).unwrap();
            Self::new_with_config(config)
        }

        pub fn new_with_config(config: Configuration) -> Self {
            Self(hc_http_gateway_router(config))
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
}
