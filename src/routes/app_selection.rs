use axum::{
    extract::{Path, State},
    Json,
};
use holochain_types::dna::DnaHash;

use crate::{
    app_selection::try_get_valid_app, service::AppState, HcHttpGatewayError, HcHttpGatewayResult,
};

#[tracing::instrument(skip(state))]
pub async fn app_selection(
    Path(raw_dna_hash): Path<String>,
    State(mut state): State<AppState>,
) -> HcHttpGatewayResult<Json<String>> {
    let dna_hash = DnaHash::try_from(raw_dna_hash)
        .map_err(|_| HcHttpGatewayError::RequestMalformed("Invalid DNA hash".to_string()))?;
    let app_info = try_get_valid_app(
        dna_hash,
        &mut state.installed_apps,
        &state.configuration.allowed_app_ids,
        state.admin_call,
    )
    .await?;

    Ok(Json(app_info.installed_app_id))
}
