use holochain::sweettest::SweetConductor;
use holochain_http_gateway::test::test_tracing::initialize_testing_tracing_subscriber;
use reqwest::StatusCode;
use setup::TestApp;

mod setup;

#[tokio::test(flavor = "multi_thread")]
async fn health_check_works() {
    initialize_testing_tracing_subscriber();

    let sweet_conductor = SweetConductor::from_standard_config().await;

    let app = TestApp::spawn(sweet_conductor.clone()).await;

    let response = app
        .client
        .get(format!("http://{}/health", app.address))
        .send()
        .await
        .expect("Failed to execute request");

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.text().await.expect("Failed to read response body");
    assert_eq!(body, "Ok");
}
