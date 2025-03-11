#[tracing::instrument]
pub async fn health_check() -> &'static str {
    "Ok"
}

#[cfg(test)]
mod tests {
    use crate::router::tests::TestRouter;
    use reqwest::StatusCode;

    #[tokio::test(flavor = "multi_thread")]
    async fn get_request_health_check_succeeds() {
        let router = TestRouter::new().await;
        let (status_code, body) = router.request("/health").await;
        assert_eq!(status_code, StatusCode::OK);
        assert_eq!(body, "Ok");
    }
}
