use axum::extract::{FromRequestParts, Path, Query, State};
use base64::{prelude::BASE64_URL_SAFE, Engine};
use holochain_types::dna::DnaHash;
use serde::Deserialize;

use crate::{service::AppState, HcHttpGatewayError, HcHttpGatewayResult};

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

#[tracing::instrument(skip(state))]
pub async fn zome_call(
    params: ZomeCallParams,
    State(state): State<AppState>,
    Query(query): Query<PayloadQuery>,
) -> HcHttpGatewayResult<()> {
    check_payload_size(
        query.payload.as_ref(),
        state.configuration.payload_limit_bytes,
    )?;

    let _decoded_payload = if let Some(payload) = query.payload {
        let decoded = BASE64_URL_SAFE.decode(payload)?;
        let json = serde_json::from_slice::<serde_json::Value>(&decoded)?;
        Some(json)
    } else {
        None
    };
    Ok(())
}

fn check_payload_size(
    payload: Option<&String>,
    payload_limit_bytes: u32,
) -> HcHttpGatewayResult<()> {
    if let Some(encoded_payload) = payload {
        let estimated_decoded_size = calculate_base64_decoded_size(&encoded_payload);

        if estimated_decoded_size > payload_limit_bytes {
            return Err(HcHttpGatewayError::PayloadSizeLimitError {
                size: estimated_decoded_size,
                limit: payload_limit_bytes,
            });
        }
    }

    Ok(())
}

/// Calculate the approximate decoded size without actually decoding
/// Base64 encoding: every 4 chars in base64 represent 3 bytes of original data
/// Need to account for padding characters too ('='), which don't represent data
fn calculate_base64_decoded_size(encoded_payload: &str) -> u32 {
    let encoded_len = encoded_payload.len();
    let padding_count = encoded_payload
        .chars()
        .rev()
        .take_while(|c| *c == '=')
        .count();

    // Adjust the encoded length by removing padding characters
    let effective_encoded_len = encoded_len - padding_count;

    // Formula: decoded_size = (effective_encoded_len * 3) / 4
    ((effective_encoded_len * 3) / 4) as u32
}
