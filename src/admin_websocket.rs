use holochain_client::AppInfo;

#[cfg(not(test))]
pub use real_wrapper::AdminWebsocketWrapper;

#[cfg(not(test))]
mod real_wrapper {
    use std::sync::Arc;

    use holochain_client::AdminWebsocket;
    use tokio::sync::Mutex;

    use super::*;

    /// Fake AdminWebsocket until https://github.com/holochain/hc-http-gw/issues/11 is done.
    #[derive(Clone)]
    pub struct AdminWebsocketWrapper {
        inner: Arc<Mutex<AdminWebsocket>>,
    }

    impl AdminWebsocketWrapper {
        /// Connect to an AdminWebsocket at the passed address
        pub async fn connect(socket_addr: &str) -> Self {
            Self {
                inner: Arc::new(Mutex::new(
                    AdminWebsocket::connect(socket_addr).await.unwrap(),
                )),
            }
        }

        /// Get a list of all installed apps
        pub async fn list_apps(&self) -> Vec<AppInfo> {
            self.inner.lock().await.list_apps(None).await.unwrap()
        }
    }
}

#[cfg(test)]
pub use MockAdminWebsocketWrapper as AdminWebsocketWrapper;

#[cfg(test)]
mockall::mock! {
    #[derive(Debug)]
    pub AdminWebsocketWrapper {
        pub async fn connect(socket_addr: &str) -> Self;
        pub async fn list_apps(&self) -> Vec<AppInfo>;
    }
    impl Clone for AdminWebsocketWrapper {
        fn clone(&self) -> Self;
    }
}
