use axum::extract::{Path, Query};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[allow(unused, reason = "Temporarily unused fields")]
pub struct ZomeCallParams {
    dna_hash: String,
    coordinator_identifier: String,
    zome_name: String,
    function_name: String,
}

#[derive(Debug, Deserialize)]
pub struct PayloadQuery {
    pub payload: Option<String>,
}

#[tracing::instrument]
pub async fn zome_call(
    Path(params): Path<ZomeCallParams>,
    Query(query): Query<PayloadQuery>,
) -> &'static str {
    "Ok"
}
