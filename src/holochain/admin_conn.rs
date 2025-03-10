use crate::{AdminCall, HcHttpGatewayResult};
use futures::future::BoxFuture;
use holochain_client::{AdminWebsocket, AuthorizeSigningCredentialsPayload, SigningCredentials};
use holochain_conductor_api::{
    AppAuthenticationTokenIssued, AppInterfaceInfo, IssueAppAuthenticationTokenPayload,
};
use holochain_types::websocket::AllowedOrigins;

/// Placeholder for the admin connection.
#[derive(Debug)]
pub struct AdminConn;

impl AdminCall for AdminConn {
    fn list_app_interfaces(
        &self,
    ) -> BoxFuture<'static, HcHttpGatewayResult<Vec<AppInterfaceInfo>>> {
        todo!()
    }

    fn issue_app_auth_token(
        &self,
        _payload: IssueAppAuthenticationTokenPayload,
    ) -> BoxFuture<'static, HcHttpGatewayResult<AppAuthenticationTokenIssued>> {
        todo!()
    }

    fn authorize_signing_credentials(
        &self,
        _payload: AuthorizeSigningCredentialsPayload,
    ) -> BoxFuture<'static, HcHttpGatewayResult<SigningCredentials>> {
        todo!()
    }

    fn attach_app_interface(
        &self,
        _port: u16,
        _allowed_origins: AllowedOrigins,
        _installed_app_id: Option<String>,
    ) -> BoxFuture<'static, HcHttpGatewayResult<u16>> {
        todo!()
    }

    #[cfg(feature = "test-utils")]
    fn set_admin_ws(&self, _admin_ws: AdminWebsocket) -> BoxFuture<'static, ()> {
        todo!()
    }
}
