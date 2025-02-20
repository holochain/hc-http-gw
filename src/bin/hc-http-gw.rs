use clap::Parser;
use holochain_http_gateway::{HcHttpGatewayArgs, HcHttpGatewayService};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = HcHttpGatewayArgs::parse();
    let service: HcHttpGatewayService = args.try_into()?;

    service.run().await?;

    Ok(())
}
