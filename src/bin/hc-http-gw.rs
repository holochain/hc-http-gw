use clap::Parser;
use holochain_http_gateway::{
    tracing::initialize_tracing_subscriber, HcHttpGatewayArgs, HcHttpGatewayService,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    initialize_tracing_subscriber("info");

    let args = HcHttpGatewayArgs::parse();
    let service: HcHttpGatewayService = args.try_into()?;

    service.run().await?;

    Ok(())
}
