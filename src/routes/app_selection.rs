use crate::HcHttpGatewayResult;

#[tracing::instrument]
pub async fn app_selection(raw_dna_hash: String) -> HcHttpGatewayResult<()> {
    Ok(())
}
