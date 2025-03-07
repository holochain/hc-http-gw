use axum::extract::{Path, State};
use holochain_types::dna::DnaHashB64;

use crate::{
    app_selection::check_app_valid, service::AppState, HcHttpGatewayError, HcHttpGatewayResult,
};

#[tracing::instrument(skip(state))]
pub async fn app_selection(
    Path(raw_dna_hash): Path<String>,
    State(mut state): State<AppState>,
) -> HcHttpGatewayResult<()> {
    let dna_hash = DnaHashB64::from_b64_str(&raw_dna_hash)
        .map_err(|_| HcHttpGatewayError::RequestMalformed("Invalid DNA hash".to_string()))?;
    check_app_valid(
        dna_hash,
        &mut state.installed_apps,
        &state.configuration.allowed_app_ids,
        state.admin_call,
    )
    .map_err(Into::into)
}
