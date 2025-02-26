use std::net::IpAddr;

/// Command line arguments and environment variables for configuring the Gateway Service
#[derive(clap::Parser, Debug)]
pub struct HcHttpGatewayArgs {
    /// The address to use
    #[arg(short, long, env = "HC_GW_ADDRESS", default_value = "127.0.0.1")]
    pub address: IpAddr,

    /// The port to bind to
    #[arg(short, long, env = "HC_GW_PORT", default_value = "8090")]
    pub port: u16,
}
