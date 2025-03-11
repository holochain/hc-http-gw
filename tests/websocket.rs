use crate::sweet::{init_zome, install_fixture1, install_fixture2, TestType};
use futures::future::BoxFuture;
use holochain::sweettest::SweetConductor;
use holochain_client::{
    AdminWebsocket, AuthorizeSigningCredentialsPayload, CellInfo, ConductorApiError, ExternIO,
    SigningCredentials, ZomeCallTarget,
};
use holochain_conductor_api::{
    AppAuthenticationTokenIssued, AppInterfaceInfo, IssueAppAuthenticationTokenPayload,
};
use holochain_http_gateway::config::{AllowedFns, Configuration, ZomeFn};
use holochain_http_gateway::tracing::initialize_tracing_subscriber;
use holochain_http_gateway::{
    AdminCall, AdminConn, AppConnPool, HcHttpGatewayError, HcHttpGatewayResult, HTTP_GW_ORIGIN,
};
use holochain_types::app::DisabledAppReason;
use holochain_types::websocket::AllowedOrigins;
use std::net::Ipv4Addr;
use std::sync::Arc;
use tokio::sync::Mutex;
use url2::url2;

mod setup;
mod sweet;

#[tokio::test(flavor = "multi_thread")]
async fn connect_app_websocket() {
    initialize_tracing_subscriber();

    let sweet_conductor = SweetConductor::from_standard_config().await;

    let app_1 = install_fixture1(sweet_conductor.clone(), None)
        .await
        .unwrap();
    init_zome(sweet_conductor.clone(), &app_1, "coordinator1".to_string())
        .await
        .unwrap();
    install_fixture2(sweet_conductor.clone(), None)
        .await
        .unwrap();
    init_zome(sweet_conductor.clone(), &app_1, "coordinator2".to_string())
        .await
        .unwrap();

    let admin_port = sweet_conductor
        .get_arbitrary_admin_websocket_port()
        .unwrap();
    let admin_ws = AdminWebsocket::connect((Ipv4Addr::LOCALHOST, admin_port))
        .await
        .unwrap();

    let apps = admin_ws.list_apps(None).await.unwrap();
    assert_eq!(apps.len(), 2);

    let admin_call = Arc::new(AdminWrapper::new(admin_ws));
    let pool = AppConnPool::new(create_test_configuration(admin_port), admin_call.clone());

    let app_client_1 = pool
        .get_or_connect_app_client("fixture1".to_string())
        .await
        .unwrap();
    assert_eq!(
        "fixture1".to_string(),
        app_client_1.cached_app_info().installed_app_id
    );

    let app_client_2 = pool
        .get_or_connect_app_client("fixture2".to_string())
        .await
        .unwrap();
    assert_eq!(
        "fixture2".to_string(),
        app_client_2.cached_app_info().installed_app_id
    );

    // Should have provisioned exactly 1 app interface for the HTTP gateway
    //
    // Note that this check would also pass if the conductor was exposing an app interface with
    // allowed origins *.
    let app_interfaces = sweet_conductor.list_app_interfaces().await.unwrap();
    let matched_app_interfaces = app_interfaces
        .iter()
        .filter(|interface| interface.allowed_origins.is_allowed(HTTP_GW_ORIGIN))
        .collect::<Vec<_>>();
    assert_eq!(matched_app_interfaces.len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn reuse_connection() {
    initialize_tracing_subscriber();

    let sweet_conductor = SweetConductor::from_standard_config().await;

    let app = install_fixture1(sweet_conductor.clone(), None)
        .await
        .unwrap();
    init_zome(sweet_conductor.clone(), &app, "coordinator1".to_string())
        .await
        .unwrap();

    let admin_port = sweet_conductor
        .get_arbitrary_admin_websocket_port()
        .unwrap();
    let admin_ws = AdminWebsocket::connect((Ipv4Addr::LOCALHOST, admin_port))
        .await
        .unwrap();

    let admin_call = Arc::new(AdminWrapper::new(admin_ws));
    let pool = AppConnPool::new(create_test_configuration(admin_port), admin_call.clone());

    let app_client_1 = pool
        .get_or_connect_app_client("fixture1".to_string())
        .await
        .unwrap();
    assert_eq!(
        "fixture1".to_string(),
        app_client_1.cached_app_info().installed_app_id
    );

    // Take out a read lock so that the pool cannot create a new connection
    let inner_pool = pool.get_inner_pool();
    let _read_lock = inner_pool.read().await;

    let app_client_1_handle = tokio::time::timeout(std::time::Duration::from_millis(100), {
        let pool = pool.clone();
        async move { pool.get_or_connect_app_client("fixture1".to_string()).await }
    })
    .await
    .unwrap()
    .unwrap();

    // Check that the client is usable
    assert_eq!(
        "fixture1".to_string(),
        app_client_1_handle
            .app_info()
            .await
            .unwrap()
            .unwrap()
            .installed_app_id
    );

    // Demonstrate that the pool was prevented from writing by the read lock held above.
    assert!(inner_pool.try_write().is_err());
}

/// When making calls using the app connection pool, we need to reconnect websockets that are
/// closed or otherwise in a problem state. However, we don't want to reconnect for other errors.
/// In this test, we connect an app websocket and then disable the target app. We then prevent the
/// pool from opening new connections and try to make a call. The call should fail with an error
/// immediately, without trying to reconnect.
/// If the code did try to reconnect, this test will fail with a timeout instead.
#[tokio::test(flavor = "multi_thread")]
async fn does_not_reconnect_on_non_websocket_error() {
    initialize_tracing_subscriber();

    let sweet_conductor = SweetConductor::from_standard_config().await;

    let app = install_fixture1(sweet_conductor.clone(), None)
        .await
        .unwrap();
    init_zome(sweet_conductor.clone(), &app, "coordinator1".to_string())
        .await
        .unwrap();

    let admin_port = sweet_conductor
        .get_arbitrary_admin_websocket_port()
        .unwrap();
    let admin_ws = AdminWebsocket::connect((Ipv4Addr::LOCALHOST, admin_port))
        .await
        .unwrap();

    let admin_call = Arc::new(AdminWrapper::new(admin_ws));
    let pool = AppConnPool::new(create_test_configuration(admin_port), admin_call.clone());

    // Connect while the app is running
    let app_client = pool
        .get_or_connect_app_client("fixture1".to_string())
        .await
        .unwrap();
    assert_eq!(
        "fixture1".to_string(),
        app_client.cached_app_info().installed_app_id
    );

    // Disable the app
    sweet_conductor
        .disable_app("fixture1".to_string(), DisabledAppReason::User)
        .await
        .unwrap();

    // Take out a write lock so that the pool cannot create a new connection
    let inner_pool = pool.get_inner_pool();
    let _read_lock = inner_pool.read().await;

    let cells = app_client
        .cached_app_info()
        .cell_info
        .values()
        .flat_map(|app_info| {
            app_info.iter().filter_map(|cell_info| match cell_info {
                CellInfo::Provisioned(provisioned) => Some(provisioned.clone()),
                _ => None,
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(cells.len(), 1);

    let cell_id = cells[0].cell_id.clone();

    let err = tokio::time::timeout(std::time::Duration::from_secs(30), async move {
        pool.call::<ExternIO>("fixture1".to_string(), |app_ws| {
            Box::pin({
                let cell_id = cell_id.clone();
                async move {
                    let response = app_ws
                        .call_zome(
                            ZomeCallTarget::CellId(cell_id),
                            "coordinator1".into(),
                            "get_all_1".into(),
                            ExternIO::encode(()).unwrap(),
                        )
                        .await?;

                    Ok(response)
                }
            })
        })
        .await
    })
    .await
    .expect("Timeout waiting for call to error")
    .unwrap_err();

    assert!(matches!(
        err,
        HcHttpGatewayError::HolochainError(ConductorApiError::ExternalApiWireError(_))
    ))
}

#[tokio::test(flavor = "multi_thread")]
async fn reconnect_on_failed_websocket() {
    initialize_tracing_subscriber();

    let mut sweet_conductor = SweetConductor::from_standard_config().await;

    let app = install_fixture1(sweet_conductor.clone(), None)
        .await
        .unwrap();
    init_zome(sweet_conductor.clone(), &app, "coordinator1".to_string())
        .await
        .unwrap();

    let admin_port = sweet_conductor
        .get_arbitrary_admin_websocket_port()
        .unwrap();
    let admin_ws = AdminWebsocket::connect((Ipv4Addr::LOCALHOST, admin_port))
        .await
        .unwrap();

    let admin_call = Arc::new(AdminWrapper::new(admin_ws));
    let mut pool = AppConnPool::new(create_test_configuration(admin_port), admin_call.clone());

    // Connect while the app is running
    let app_client = pool
        .get_or_connect_app_client("fixture1".to_string())
        .await
        .unwrap();
    assert_eq!(
        "fixture1".to_string(),
        app_client.cached_app_info().installed_app_id
    );

    // Stop the conductor
    sweet_conductor.shutdown().await;

    // Verify that the app client is not usable.
    app_client.app_info().await.unwrap_err();

    // Restart the conductor
    sweet_conductor.startup().await;

    // Reconnect the admin client
    let admin_port = sweet_conductor
        .get_arbitrary_admin_websocket_port()
        .unwrap();
    let admin_ws = AdminWebsocket::connect((Ipv4Addr::LOCALHOST, admin_port))
        .await
        .unwrap();

    pool.set_admin_ws(admin_ws).await;

    let cells = app_client
        .cached_app_info()
        .cell_info
        .values()
        .flat_map(|app_info| {
            app_info.iter().filter_map(|cell_info| match cell_info {
                CellInfo::Provisioned(provisioned) => Some(provisioned.clone()),
                _ => None,
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(cells.len(), 1);

    let cell_id = cells[0].cell_id.clone();

    // Now try to make a call, which should reconnect and succeed
    let response = pool
        .call::<ExternIO>("fixture1".to_string(), |app_ws| {
            Box::pin({
                let cell_id = cell_id.clone();
                async move {
                    let response = app_ws
                        .call_zome(
                            ZomeCallTarget::CellId(cell_id),
                            "coordinator1".into(),
                            "get_all_1".into(),
                            ExternIO::encode(()).unwrap(),
                        )
                        .await?;

                    Ok(response)
                }
            })
        })
        .await
        .unwrap();

    assert!(response.decode::<Vec<TestType>>().unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn reconnect_gives_up() {
    initialize_tracing_subscriber();

    let mut sweet_conductor = SweetConductor::from_standard_config().await;

    let app = install_fixture1(sweet_conductor.clone(), None)
        .await
        .unwrap();
    init_zome(sweet_conductor.clone(), &app, "coordinator1".to_string())
        .await
        .unwrap();

    let admin_port = sweet_conductor
        .get_arbitrary_admin_websocket_port()
        .unwrap();
    let admin_ws = AdminWebsocket::connect((Ipv4Addr::LOCALHOST, admin_port))
        .await
        .unwrap();

    let admin_call = Arc::new(AdminWrapper::new(admin_ws));
    let pool = AppConnPool::new(create_test_configuration(admin_port), admin_call.clone());

    // Connect while the app is running
    let app_client = pool
        .get_or_connect_app_client("fixture1".to_string())
        .await
        .unwrap();
    assert_eq!(
        "fixture1".to_string(),
        app_client.cached_app_info().installed_app_id
    );

    // Stop the conductor
    sweet_conductor.shutdown().await;

    let cells = app_client
        .cached_app_info()
        .cell_info
        .values()
        .flat_map(|app_info| {
            app_info.iter().filter_map(|cell_info| match cell_info {
                CellInfo::Provisioned(provisioned) => Some(provisioned.clone()),
                _ => None,
            })
        })
        .collect::<Vec<_>>();
    assert_eq!(cells.len(), 1);

    let cell_id = cells[0].cell_id.clone();

    // Now try to make a call, which won't be able to reconnect
    let err = pool
        .call::<ExternIO>("fixture1".to_string(), |app_ws| {
            Box::pin({
                let cell_id = cell_id.clone();
                async move {
                    let response = app_ws
                        .call_zome(
                            ZomeCallTarget::CellId(cell_id),
                            "coordinator1".into(),
                            "get_all_1".into(),
                            ExternIO::encode(()).unwrap(),
                        )
                        .await?;

                    Ok(response)
                }
            })
        })
        .await
        .unwrap_err();

    assert!(
        matches!(
            err,
            HcHttpGatewayError::HolochainError(ConductorApiError::WebsocketError(_))
        ),
        "Expected Holochain websocket error, got {:?}",
        err
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn close_old_connections_on_limit() {
    initialize_tracing_subscriber();

    let sweet_conductor = SweetConductor::from_standard_config().await;

    let app_1 = install_fixture1(sweet_conductor.clone(), Some("app_1".to_string()))
        .await
        .unwrap();
    init_zome(sweet_conductor.clone(), &app_1, "coordinator1".to_string())
        .await
        .unwrap();
    install_fixture1(sweet_conductor.clone(), Some("app_2".to_string()))
        .await
        .unwrap();
    install_fixture1(sweet_conductor.clone(), Some("app_3".to_string()))
        .await
        .unwrap();

    let admin_port = sweet_conductor
        .get_arbitrary_admin_websocket_port()
        .unwrap();
    let admin_ws = AdminWebsocket::connect((Ipv4Addr::LOCALHOST, admin_port))
        .await
        .unwrap();

    let configuration = Configuration::try_new(
        &format!("ws://127.0.0.1:{}", admin_port),
        "",
        "app_1,app_2,app_3",
        [
            (
                "app_1".to_string(),
                AllowedFns::Restricted(
                    [ZomeFn {
                        zome_name: "coordinator1".to_string(),
                        fn_name: "get_all_1".to_string(),
                    }]
                    .into_iter()
                    .collect(),
                ),
            ),
            (
                "app_2".to_string(),
                AllowedFns::Restricted(
                    [ZomeFn {
                        zome_name: "coordinator1".to_string(),
                        fn_name: "get_all_1".to_string(),
                    }]
                    .into_iter()
                    .collect(),
                ),
            ),
            (
                "app_3".to_string(),
                AllowedFns::Restricted(
                    [ZomeFn {
                        zome_name: "coordinator1".to_string(),
                        fn_name: "get_all_1".to_string(),
                    }]
                    .into_iter()
                    .collect(),
                ),
            ),
        ]
        .into_iter()
        .collect(),
        "2",
        "",
    )
    .unwrap();

    let admin_call = Arc::new(AdminWrapper::new(admin_ws));
    let pool = AppConnPool::new(configuration, admin_call.clone());

    // Take out connections to all 3 apps
    let _app_client_2 = pool
        .get_or_connect_app_client("app_2".to_string())
        .await
        .unwrap();

    let _app_client_1 = pool
        .get_or_connect_app_client("app_1".to_string())
        .await
        .unwrap();

    let _app_client_3 = pool
        .get_or_connect_app_client("app_3".to_string())
        .await
        .unwrap();

    let inner_pool = pool.get_inner_pool();

    let mut ws_for_apps = inner_pool
        .read()
        .await
        .values()
        .map(|state| state.app_ws.cached_app_info().installed_app_id.clone())
        .collect::<Vec<_>>();
    ws_for_apps.sort();

    // We should have open websockets for app_1 and app_3, the connection for app_2 should have
    // been removed from state because we're only allowing 2 connections at once.
    assert_eq!(ws_for_apps, vec!["app_1", "app_3"]);
}

fn create_test_configuration(admin_port: u16) -> Configuration {
    Configuration::try_new(
        &format!("ws://127.0.0.1:{}", admin_port),
        "",
        "fixture1,fixture2",
        [
            (
                "fixture1".to_string(),
                AllowedFns::Restricted(
                    [ZomeFn {
                        zome_name: "coordinator1".to_string(),
                        fn_name: "get_all_1".to_string(),
                    }]
                    .into_iter()
                    .collect(),
                ),
            ),
            (
                "fixture2".to_string(),
                AllowedFns::Restricted(
                    [ZomeFn {
                        zome_name: "coordinator2".to_string(),
                        fn_name: "get_all_2".to_string(),
                    }]
                    .into_iter()
                    .collect(),
                ),
            ),
        ]
        .into_iter()
        .collect(),
        "",
        "",
    )
    .unwrap()
}

#[tokio::test(flavor = "multi_thread")]
async fn connect_admin_websocket() {
    let mut sweet_conductor = SweetConductor::from_standard_config().await;

    let admin_port = sweet_conductor
        .get_arbitrary_admin_websocket_port()
        .unwrap();
    let url = url2!("ws://localhost:{admin_port}");

    let mut admin_ws = AdminConn::connect(&url).await.unwrap();

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

impl AdminCall for AdminWrapper {
    fn list_app_interfaces(
        &self,
    ) -> BoxFuture<'static, HcHttpGatewayResult<Vec<AppInterfaceInfo>>> {
        let inner = self.inner.clone();
        Box::pin(async move { Ok(inner.lock().await.list_app_interfaces().await?) })
    }

    fn issue_app_auth_token(
        &self,
        payload: IssueAppAuthenticationTokenPayload,
    ) -> BoxFuture<'static, HcHttpGatewayResult<AppAuthenticationTokenIssued>> {
        let inner = self.inner.clone();
        Box::pin(async move { Ok(inner.lock().await.issue_app_auth_token(payload).await?) })
    }

    fn authorize_signing_credentials(
        &self,
        payload: AuthorizeSigningCredentialsPayload,
    ) -> BoxFuture<'static, HcHttpGatewayResult<SigningCredentials>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            Ok(inner
                .lock()
                .await
                .authorize_signing_credentials(payload)
                .await?)
        })
    }

    fn attach_app_interface(
        &self,
        port: u16,
        allowed_origins: AllowedOrigins,
        installed_app_id: Option<String>,
    ) -> BoxFuture<'static, HcHttpGatewayResult<u16>> {
        let inner = self.inner.clone();
        Box::pin(async move {
            Ok(inner
                .lock()
                .await
                .attach_app_interface(port, allowed_origins, installed_app_id)
                .await?)
        })
    }

    #[cfg(feature = "test-utils")]
    fn set_admin_ws(&self, admin_ws: AdminWebsocket) -> BoxFuture<'static, ()> {
        let inner = self.inner.clone();
        Box::pin(async move {
            *inner.lock().await = admin_ws;
        })
    }
}

#[derive(Debug)]
pub struct AdminWrapper {
    inner: Arc<Mutex<AdminWebsocket>>,
}

impl AdminWrapper {
    pub fn new(admin_ws: AdminWebsocket) -> Self {
        Self {
            inner: Arc::new(Mutex::new(admin_ws)),
        }
    }
}
