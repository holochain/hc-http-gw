use std::net::IpAddr;
use std::str::FromStr;

use crate::HcHttpGatewayService;

const DEFAULT_PORT: u16 = 8090;
const DEFAULT_ADDRESS: &str = "127.0.0.1";

/// Command line arguments and environment variables for configuring the Gateway Service
#[derive(clap::Parser, Debug)]
pub struct HcHttpGatewayArgs {
    /// The address to use (e.g., 127.0.0.1).
    #[arg(short, long, env = "HC_GW_ADDRESS", default_value = "127.0.0.1")]
    pub address: Option<String>,

    /// The port to bind to. (e.g 8000)
    #[arg(short, long, env = "HC_GW_PORT", default_value = "8090")]
    pub port: Option<u16>,
}

impl TryInto<HcHttpGatewayService> for HcHttpGatewayArgs {
    type Error = anyhow::Error;

    fn try_into(self) -> Result<HcHttpGatewayService, Self::Error> {
        let address =
            IpAddr::from_str(&self.address.unwrap_or_else(|| DEFAULT_ADDRESS.to_string()))?;
        let port = self.port.unwrap_or(DEFAULT_PORT);
        Ok(HcHttpGatewayService::new(address, port))
    }
}
