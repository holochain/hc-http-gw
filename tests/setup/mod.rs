#![allow(dead_code)]

use holochain::conductor::Conductor;
use holochain::prelude::DnaHash;
use holochain_http_gateway::{
    AdminConn, AllowedFns, AppConnPool, Configuration, HcHttpGatewayService, ZomeFn,
};
use reqwest::{Client, Response};
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio::task::JoinHandle;

/// Test application harness for the HTTP gateway service
pub struct TestGateway {
    pub address: String,
    pub client: Client,
    pub task_handle: JoinHandle<()>,
}

impl TestGateway {
    /// Create a new test application with default configuration.
    /// Allowed app ids contains "forum".
    /// Allowed functions contains all functions of "forum".
    pub async fn spawn(conductor: Arc<Conductor>) -> Self {
        // Create default allowed functions
        let mut allowed_fns = HashMap::new();
        allowed_fns.insert(
            "fixture1".to_string(),
            AllowedFns::Restricted(
                [
                    ZomeFn {
                        zome_name: "coordinator1".to_string(),
                        fn_name: "get_all_1".to_string(),
                    },
                    ZomeFn {
                        zome_name: "coordinator1".to_string(),
                        fn_name: "get_mine".to_string(),
                    },
                    ZomeFn {
                        zome_name: "coordinator1".to_string(),
                        fn_name: "get_limited".to_string(),
                    },
                ]
                .into_iter()
                .collect(),
            ),
        );
        allowed_fns.insert(
            "fixture2".to_string(),
            AllowedFns::Restricted(
                [ZomeFn {
                    zome_name: "coordinator2".to_string(),
                    fn_name: "get_all_2".to_string(),
                }]
                .into_iter()
                .collect(),
            ),
        );

        let admin_port = conductor.get_arbitrary_admin_websocket_port().unwrap();

        // Create configuration
        let config = Configuration::try_new(
            SocketAddr::new(Ipv4Addr::LOCALHOST.into(), admin_port),
            "1024",
            "fixture1,fixture2",
            allowed_fns,
            "",
            "",
        )
        .unwrap();

        TestGateway::spawn_with_config(config).await
    }

    /// Create a test app with custom configuration
    pub async fn spawn_with_config(config: Configuration) -> Self {
        let admin_call = Arc::new(AdminConn::new(config.admin_socket_addr));
        let app_call = Arc::new(AppConnPool::new(config.clone(), admin_call.clone()));

        let service =
            HcHttpGatewayService::new([127, 0, 0, 1], 0, config.clone(), admin_call, app_call)
                .await
                .unwrap();

        let address = service.address().unwrap().to_string();

        // Run service in the background
        let task_handle = tokio::task::spawn(async move { service.run().await.unwrap() });

        TestGateway {
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

impl Drop for TestGateway {
    fn drop(&mut self) {
        self.task_handle.abort();
    }
}
