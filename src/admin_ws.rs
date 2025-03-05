use std::{
    sync::{Arc, Mutex},
    usize,
};

use holochain_client::AdminWebsocket;

#[derive(Clone)]
pub struct ReconnectingAdminSocket {
    url: String,
    connection: Arc<Mutex<Option<AdminWebsocket>>>,
    max_retries: usize,
    retry_delay_ms: u64,
    current_retries: usize,
}
