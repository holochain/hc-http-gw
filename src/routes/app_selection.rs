use holochain_types::dna::DnaHashB64;

use crate::HcHttpGatewayResult;

#[tracing::instrument]
pub async fn app_selection(raw_dna_hash: String) -> HcHttpGatewayResult<()> {
    let _dna_hash = DnaHashB64::from_b64_str(&raw_dna_hash)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::HcHttpGatewayError;

    use super::*;
    use assert2::assert;
    use holochain::core::DnaHash;

    #[tokio::test]
    async fn returns_hash_error_if_invalid_hash() {
        let result = app_selection("invalid-hash".to_string()).await;

        assert!(let Err(HcHttpGatewayError::HoloHashError(_)) = result);
    }

    #[tokio::test]
    async fn returns_ok_if_valid_hash() {
        let hash: DnaHashB64 = DnaHash::from_raw_32([1; 32].to_vec()).into();
        let result = app_selection(hash.to_string()).await;

        assert!(let Ok(()) = result);
    }
}
