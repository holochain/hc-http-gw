#[tracing::instrument]
pub async fn healthz() -> &'static str {
    "Ok"
}
