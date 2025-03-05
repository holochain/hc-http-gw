mod setup;

use reqwest::StatusCode;

use setup::TestApp;

#[tokio::test]
async fn health_check_works() {
    let app = TestApp::spawn().await;

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
