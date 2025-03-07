use crate::{AdminCall, HcHttpGatewayResult};
use futures::future::BoxFuture;
use holochain_client::{
    AgentPubKey, AppInfo, AuthorizeSigningCredentialsPayload, SigningCredentials,
};
use holochain_conductor_api::{
    AppAuthenticationTokenIssued, AppInterfaceInfo, IssueAppAuthenticationTokenPayload,
};
use holochain_types::dna::DnaHashB64;
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

    fn list_apps(&self) -> Vec<AppInfo> {
        use holochain_types::app::{AppManifest, AppRoleDnaManifest, AppRoleManifest, AppStatus};
        use std::collections::HashMap;

        vec![
            AppInfo {
                installed_app_id: "app1".to_string(),
                cell_info: HashMap::new(),
                status: AppStatus::Running.into(),
                agent_pub_key: AgentPubKey::from_raw_39([1; 39].to_vec()).unwrap(),
                manifest: AppManifest::V1(holochain_types::app::AppManifestV1 {
                    name: Default::default(),
                    description: Default::default(),
                    roles: vec![AppRoleManifest {
                        name: Default::default(),
                        provisioning: Default::default(),
                        dna: AppRoleDnaManifest {
                            location: Default::default(),
                            modifiers: Default::default(),
                            installed_hash: Some(
                                DnaHashB64::from_b64_str(
                                    "uhC0kVKaUAQEBAP8BHVUAAAGeFP8LzP8BAQEB3__4_9EAAAD9L-hZ",
                                )
                                .unwrap(),
                            ),
                            clone_limit: Default::default(),
                        },
                    }],
                    allow_deferred_memproofs: Default::default(),
                }),
            },
            AppInfo {
                installed_app_id: "app2".to_string(),
                cell_info: HashMap::new(),
                status: AppStatus::Running.into(),
                agent_pub_key: AgentPubKey::from_raw_39([2; 39].to_vec()).unwrap(),
                manifest: AppManifest::V1(holochain_types::app::AppManifestV1 {
                    name: Default::default(),
                    description: Default::default(),
                    roles: vec![AppRoleManifest {
                        name: Default::default(),
                        provisioning: Default::default(),
                        dna: AppRoleDnaManifest {
                            location: Default::default(),
                            modifiers: Default::default(),
                            installed_hash: Some(
                                DnaHashB64::from_b64_str(
                                    "uhC0k9gEAAf90Af9kZ0OfAQH_egrtAf20YBMB_w0B6gCEo_8k8aBt",
                                )
                                .unwrap(),
                            ),
                            clone_limit: Default::default(),
                        },
                    }],
                    allow_deferred_memproofs: Default::default(),
                }),
            },
        ]
    }

    #[cfg(feature = "test-utils")]
    fn set_admin_ws(&self, _admin_ws: holochain_client::AdminWebsocket) -> BoxFuture<'static, ()> {
        todo!()
    }
}
