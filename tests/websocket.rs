use crate::sweet::{install_fixture1, install_fixture2};
use holochain::sweettest::SweetConductor;
use holochain_client::AdminWebsocket;
use holochain_http_gateway::{HcHttpGatewayError, ReconnectingAdminWebsocket};
use std::net::Ipv4Addr;

mod setup;
mod sweet;

#[tokio::test(flavor = "multi_thread")]
async fn connect_app_websocket() {
    let sweet_conductor = SweetConductor::from_standard_config().await;

    install_fixture1(sweet_conductor.clone()).await;
    install_fixture2(sweet_conductor.clone()).await;

    let admin_port = sweet_conductor
        .get_arbitrary_admin_websocket_port()
        .unwrap();
    let admin_ws = AdminWebsocket::connect((Ipv4Addr::LOCALHOST, admin_port))
        .await
        .unwrap();

    let apps = admin_ws.list_apps(None).await.unwrap();

    assert_eq!(apps.len(), 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn connect_admin_websocket() {
    let mut sweet_conductor = SweetConductor::from_standard_config().await;

    let admin_port = sweet_conductor
        .get_arbitrary_admin_websocket_port()
        .unwrap();
    let url = format!("localhost:{admin_port}");

    let mut admin_ws = ReconnectingAdminWebsocket::new(&url);

    // First call should succeed
    let apps = admin_ws
        .call(|ws| async move { ws.list_apps(None).await })
        .await
        .unwrap();

    assert_eq!(apps.len(), 0);
    assert_eq!(admin_ws.get_reconnection_retries(), 0);

    // Shutdown the conductor to force a connection error
    sweet_conductor.shutdown().await;

    let apps = admin_ws
        .call(|ws| async move { ws.list_apps(None).await })
        .await;

    if let Err(err) = apps {
        assert!(matches!(err, HcHttpGatewayError::UpstreamUnavailable));
    } else {
        panic!("Expected UpstreamUnavailable error, found: {apps:?}");
    }
}
