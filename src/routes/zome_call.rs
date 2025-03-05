use axum::extract::{FromRequestParts, Path, Query};
use base64::{prelude::BASE64_URL_SAFE, Engine};
use holochain_types::dna::DnaHash;
use serde::Deserialize;

use crate::{HcHttpGatewayError, HcHttpGatewayResult};

#[derive(Debug, Deserialize)]
#[allow(unused, reason = "Temporarily unused fields")]
pub struct ZomeCallParams {
    dna_hash: DnaHash,
    coordinator_identifier: String,
    zome_name: String,
    function_name: String,
}

#[derive(Debug, Deserialize)]
struct RawZomeCallParams {
    dna_hash: String,
    coordinator_identifier: String,
    zome_name: String,
    function_name: String,
}

impl<S> FromRequestParts<S> for ZomeCallParams
where
    S: Send + Sync,
{
    type Rejection = HcHttpGatewayError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let Path(raw_params) = Path::<RawZomeCallParams>::from_request_parts(parts, state).await?;
        let dna_hash = DnaHash::try_from(raw_params.dna_hash)?;

        Ok(ZomeCallParams {
            dna_hash,
            coordinator_identifier: raw_params.coordinator_identifier,
            zome_name: raw_params.zome_name,
            function_name: raw_params.function_name,
        })
    }
}

#[derive(Debug, Deserialize)]
pub struct PayloadQuery {
    pub payload: Option<String>,
}

#[tracing::instrument]
pub async fn zome_call(
    params: ZomeCallParams,
    Query(query): Query<PayloadQuery>,
) -> HcHttpGatewayResult<()> {
    let _decoded_payload = if let Some(payload) = query.payload {
        let decoded = BASE64_URL_SAFE.decode(payload)?;
        let json = serde_json::from_slice::<serde_json::Value>(&decoded)?;
        Some(json)
    } else {
        None
    };
    Ok(())
}
