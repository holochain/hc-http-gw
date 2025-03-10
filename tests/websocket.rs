use crate::sweet::{install_fixture1, install_fixture2};
use holochain::sweettest::SweetConductor;
use holochain_client::AdminWebsocket;
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

#[tokio::test]
async fn connect_admin_websocket() {}
