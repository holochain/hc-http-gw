use holochain_http_gateway::{
    config::{AllowedFns, Configuration},
    AdminConn, AppConnPool, HcHttpGatewayService,
};

use reqwest::{Client, Response};
use std::collections::HashMap;
use std::sync::Arc;
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
    pub async fn spawn() -> Self {
        // Create default allowed functions
        let mut allowed_fns = HashMap::new();
        allowed_fns.insert("forum".to_string(), AllowedFns::All);

        // Create configuration
        let config =
            Configuration::try_new("ws://localhost:50350", "1024", "forum", allowed_fns, "", "")
                .unwrap();

        TestApp::spawn_with_config(config).await
    }

    /// Create a test app with custom configuration
    pub async fn spawn_with_config(config: Configuration) -> Self {
        let admin_call = Arc::new(AdminConn);
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
        dna_hash: &str,
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
