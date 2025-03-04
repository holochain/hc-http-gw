use axum::extract::{Path, Query};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ZomeCallParams {
    dna_hash: String,
    coordinator_identifier: String,
    zome_name: String,
    function_name: String,
}

#[derive(Debug, Deserialize)]
pub struct PayloadQuery {
    payload: Option<String>,
}

// http://<host>/<dna-hash>/<coordinator-identifier>/<zome-name>/<function-name>?payload=<payload>
#[tracing::instrument]
pub async fn zome_call(
    Path(params): Path<ZomeCallParams>,
    Query(query): Query<PayloadQuery>,
) -> &'static str {
    todo!("zome call");
}
