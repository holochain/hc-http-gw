use crate::app_selection::try_get_valid_app;
use crate::{
    service::AppState,
    transcode::{base64_json_to_hsb, hsb_to_json},
    HcHttpGatewayError, HcHttpGatewayResult,
};
use axum::extract::{FromRequestParts, Path, Query, State};
use holochain_types::dna::DnaHash;
use serde::Deserialize;

const MAX_IDENTIFIER_CHARS: u8 = 100;

#[derive(Debug, Deserialize)]
#[allow(unused, reason = "Temporarily unused fields")]
pub struct ZomeCallParams {
    dna_hash: DnaHash,
    coordinator_identifier: String,
    zome_name: String,
    fn_name: String,
}

#[derive(Debug, Deserialize)]
struct RawZomeCallParams {
    dna_hash: String,
    coordinator_identifier: String,
    zome_name: String,
    fn_name: String,
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
        let Path(raw_params) = Path::<RawZomeCallParams>::from_request_parts(parts, state)
            .await
            .map_err(|err| HcHttpGatewayError::RequestMalformed(err.to_string()))?;
        let RawZomeCallParams {
            dna_hash,
            coordinator_identifier,
            zome_name,
            fn_name,
        } = raw_params;
        // Check DNA hash validity.
        let dna_hash = DnaHash::try_from(dna_hash)
            .map_err(|_| HcHttpGatewayError::RequestMalformed("Invalid DNA hash".to_string()))?;
        // Reject identifiers longer than the maximum length.
        if coordinator_identifier.chars().count() > MAX_IDENTIFIER_CHARS as usize {
            return Err(HcHttpGatewayError::RequestMalformed(format!(
                "Identifier {coordinator_identifier} longer than {MAX_IDENTIFIER_CHARS} characters"
            )));
        }
        if zome_name.chars().count() > MAX_IDENTIFIER_CHARS as usize {
            return Err(HcHttpGatewayError::RequestMalformed(format!(
                "Identifier {zome_name} longer than {MAX_IDENTIFIER_CHARS} characters"
            )));
        }
        if fn_name.chars().count() > MAX_IDENTIFIER_CHARS as usize {
            return Err(HcHttpGatewayError::RequestMalformed(format!(
                "Identifier {fn_name} longer than {MAX_IDENTIFIER_CHARS} characters"
            )));
        }

        Ok(ZomeCallParams {
            dna_hash,
            coordinator_identifier,
            zome_name,
            fn_name,
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
) -> HcHttpGatewayResult<String> {
    let ZomeCallParams {
        dna_hash,
        coordinator_identifier,
        zome_name,
        fn_name,
    } = params;
    // Check payload byte length does not exceed configured maximum.
    if let Some(payload) = &query.payload {
        // `len()` of a string is not the number of characters, but the number of bytes.
        if payload.len() > state.configuration.payload_limit_bytes as usize {
            return Err(HcHttpGatewayError::RequestMalformed(format!(
                "Payload exceeds {} bytes",
                state.configuration.payload_limit_bytes
            )));
        }
    }

    let app_info = try_get_valid_app(
        dna_hash,
        coordinator_identifier.clone(),
        state.app_info_cache.clone(),
        &state.configuration.allowed_app_ids,
        state.admin_call.clone(),
    )
    .await?;

    // Check if function name is allowed.
    if !state
        .configuration
        .is_function_allowed(&app_info.installed_app_id, &zome_name, &fn_name)
    {
        return Err(HcHttpGatewayError::UnauthorizedFunction {
            app_id: app_info.installed_app_id,
            zome_name,
            fn_name,
        });
    }

    // Transcode payload from base64 encoded JSON to ExternIO.
    let zome_call_payload = base64_json_to_hsb(query.payload)?;

    let serialized_response = state
        .app_call
        .handle_zome_call(
            coordinator_identifier.into(),
            zome_name,
            fn_name,
            zome_call_payload,
        )
        .await?;

    // Transcode ExternIO response to JSON.
    hsb_to_json(&serialized_response)
}

#[cfg(test)]
mod tests;
