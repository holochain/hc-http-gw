#[tracing::instrument]
pub async fn health_check() -> &'static str {
    "Ok"
}
