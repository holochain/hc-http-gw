use axum::extract::{Path, State};
use holochain_types::dna::DnaHashB64;

use crate::{app_selection::check_app_valid, service::AppState, HcHttpGatewayResult};

#[tracing::instrument(skip(state))]
pub async fn app_selection(
    Path(raw_dna_hash): Path<String>,
    State(mut state): State<AppState>,
) -> HcHttpGatewayResult<()> {
    let dna_hash = DnaHashB64::from_b64_str(&raw_dna_hash)?;
    check_app_valid(
        dna_hash,
        &mut state.installed_apps,
        &state.configuration.allowed_app_ids,
        &state.admin_websocket,
    )
    .map_err(Into::into)
}
