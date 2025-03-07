use axum::{
    extract::{Path, State},
    Json,
};
use holochain_types::dna::DnaHashB64;

use crate::{app_selection::try_get_valid_app, service::AppState, HcHttpGatewayResult};

#[tracing::instrument(skip(state))]
pub async fn app_selection(
    Path(raw_dna_hash): Path<String>,
    State(mut state): State<AppState>,
) -> HcHttpGatewayResult<Json<String>> {
    let dna_hash = DnaHashB64::from_b64_str(&raw_dna_hash)?;
    let app_info = try_get_valid_app(
        dna_hash,
        &mut state.installed_apps,
        &state.configuration.allowed_app_ids,
        &state.admin_websocket,
    )?;

    Ok(Json(app_info.installed_app_id))
}
