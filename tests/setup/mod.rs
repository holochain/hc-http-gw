#![allow(dead_code)]

use futures::future::BoxFuture;
use holochain::conductor::Conductor;
use holochain::prelude::DnaHash;
use holochain_client::{AdminWebsocket, AuthorizeSigningCredentialsPayload, SigningCredentials};
use holochain_conductor_api::{
    AppAuthenticationTokenIssued, AppInfo, AppInterfaceInfo, IssueAppAuthenticationTokenPayload,
};
use holochain_http_gateway::config::{AllowedFns, Configuration};
use holochain_http_gateway::{AdminCall, AppConnPool, HcHttpGatewayResult, HcHttpGatewayService};
use holochain_types::websocket::AllowedOrigins;
use reqwest::{Client, Response};
use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

/// Test application harness for the HTTP gateway service
pub struct TestApp {
    pub address: String,
    pub client: Client,
    pub task_handle: JoinHandle<()>,
}

impl TestApp {
    /// Create a new test application with default configuration.
    /// Allowed app ids contains "forum".
    /// Allowed functions contains all functions of "forum".
    pub async fn spawn(conductor: Arc<Conductor>) -> Self {
        // Create default allowed functions
        let mut allowed_fns = HashMap::new();
        allowed_fns.insert("fixture1".to_string(), AllowedFns::All);
        allowed_fns.insert("fixture2".to_string(), AllowedFns::All);

        let admin_port = conductor.get_arbitrary_admin_websocket_port().unwrap();

        // Create configuration
        let config = Configuration::try_new(
            &format!("ws://localhost:{admin_port}"),
            "1024",
            "fixture1,fixture2",
            allowed_fns,
            "",
            "",
        )
        .unwrap();

        TestApp::spawn_with_config(config).await
    }

    /// Create a test app with custom configuration
    pub async fn spawn_with_config(config: Configuration) -> Self {
        let admin_ws =
            AdminWebsocket::connect((Ipv4Addr::LOCALHOST, config.admin_ws_url.port().unwrap()))
                .await
                .unwrap();

        let admin_call = Arc::new(AdminWrapper::new(admin_ws));
        let app_call = Arc::new(AppConnPool::new(config.clone(), admin_call.clone()));

        let service =
            HcHttpGatewayService::new([127, 0, 0, 1], 0, config.clone(), admin_call, app_call)
                .await
                .unwrap();

        let address = service.address().unwrap().to_string();

        // Run service in the background
        let task_handle = tokio::task::spawn(async move { service.run().await.unwrap() });

        TestApp {
            address,
            client: Client::new(),
            task_handle,
        }
    }

    /// Util to make a request to the zome call GET endpoint
    pub async fn call_zome(
        &self,
        dna_hash: &DnaHash,
        coordinator_identifier: &str,
        zome: &str,
        zome_fn: &str,
        payload: Option<&str>,
    ) -> Response {
        let url = {
            let mut url = format!(
                "http://{}/{dna_hash}/{coordinator_identifier}/{zome}/{zome_fn}",
                self.address
            );
            if let Some(payload) = payload {
                url.push_str(&format!("?payload={}", payload));
            }
            url
        };

        self.client
            .get(url)
            .send()
            .await
            .expect("Failed to execute request")
    }
}

impl Drop for TestApp {
    fn drop(&mut self) {
        self.task_handle.abort();
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

    fn list_apps(&self) -> BoxFuture<'static, HcHttpGatewayResult<Vec<AppInfo>>> {
        let inner = self.inner.clone();
        Box::pin(async move { Ok(inner.lock().await.list_apps(None).await?) })
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
