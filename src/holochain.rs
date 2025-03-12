use crate::HcHttpGatewayResult;
use futures::future::BoxFuture;
use holochain_client::{
    AppInfo, AuthorizeSigningCredentialsPayload, CellId, ExternIO, SigningCredentials,
};
use holochain_conductor_api::{
    AppAuthenticationTokenIssued, AppInterfaceInfo, AppStatusFilter,
    IssueAppAuthenticationTokenPayload,
};
use holochain_types::app::InstalledAppId;
use holochain_types::websocket::AllowedOrigins;

mod admin_conn;
pub use admin_conn::AdminConn;

mod app_conn_pool;
pub use app_conn_pool::{AppConnPool, AppWebsocketWithState, HTTP_GW_ORIGIN};

/// A trait for making admin calls with an admin connection.
#[cfg_attr(test, mockall::automock)]
pub trait AdminCall: std::fmt::Debug + Send + Sync {
    /// Call [`AdminWebsocket::list_app_interfaces`](holochain_client::AdminWebsocket::list_app_interfaces).
    fn list_app_interfaces(&self)
        -> BoxFuture<'static, HcHttpGatewayResult<Vec<AppInterfaceInfo>>>;

    /// Call [`AdminWebsocket::issue_app_auth_token`](holochain_client::AdminWebsocket::issue_app_auth_token)
    /// with the given payload.
    fn issue_app_auth_token(
        &self,
        payload: IssueAppAuthenticationTokenPayload,
    ) -> BoxFuture<'static, HcHttpGatewayResult<AppAuthenticationTokenIssued>>;

    /// Call [`AdminWebsocket::authorize_signing_credentials`](holochain_client::AdminWebsocket::authorize_signing_credentials)
    /// with the given payload.
    fn authorize_signing_credentials(
        &self,
        payload: AuthorizeSigningCredentialsPayload,
    ) -> BoxFuture<'static, HcHttpGatewayResult<SigningCredentials>>;

    /// Call [`AdminWebsocket::attach_app_interface`](holochain_client::AdminWebsocket::attach_app_interface) with the given parameters.
    fn attach_app_interface(
        &self,
        port: u16,
        allowed_origins: AllowedOrigins,
        installed_app_id: Option<String>,
    ) -> BoxFuture<'static, HcHttpGatewayResult<u16>>;

    /// Call [`AdminWebsocket::list_apps`](holochain_client::AdminWebsocket::list_apps) and return
    /// the result.
    fn list_apps(
        &self,
        status_filter: Option<AppStatusFilter>,
    ) -> BoxFuture<'static, HcHttpGatewayResult<Vec<AppInfo>>>;
}

/// A trait for making zome calls with an app connection.
///
/// Primarily used to allow the [`AppConnPool`] to be mocked in tests.
#[cfg_attr(test, mockall::automock)]
pub trait AppCall: std::fmt::Debug + Send + Sync {
    /// Make a zome call by executing the provided function with an app websocket connection.
    fn handle_zome_call(
        &self,
        installed_app_id: InstalledAppId,
        cell_id: CellId,
        zome_name: String,
        fn_name: String,
        payload: ExternIO,
    ) -> BoxFuture<'static, HcHttpGatewayResult<ExternIO>>;
}
