use axum::{
    extract::{Query, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};

use crate::{http::HcGwErrorResponse, routes::PayloadQuery, service::AppState};

#[tracing::instrument(skip(state, request, next))]
pub async fn validate_zome_call_payload_size(
    State(state): State<AppState>,
    Query(query): Query<PayloadQuery>,
    request: Request,
    next: Next,
) -> Response {
    if let Some(encoded_payload) = query.payload {
        let estimated_decoded_size = calculate_base64_decoded_size(&encoded_payload);

        if estimated_decoded_size > state.configuration.payload_limit_bytes {
            return (
                StatusCode::BAD_REQUEST,
                Json(HcGwErrorResponse {
                    error: format!(
                        "Payload size ({} bytes) exceeds maximum allowed size ({} bytes)",
                        estimated_decoded_size, state.configuration.payload_limit_bytes
                    ),
                }),
            )
                .into_response();
        }
    }
    next.run(request).await
}

/// Calculate the approximate decoded size without actually decoding
/// Base64 encoding: every 4 chars in base64 represent 3 bytes of original data
/// Need to account for padding characters too ('='), which don't represent data
fn calculate_base64_decoded_size(encoded_payload: &str) -> usize {
    let encoded_len = encoded_payload.len();
    let padding_count = encoded_payload
        .chars()
        .rev()
        .take_while(|c| *c == '=')
        .count();

    // Adjust the encoded length by removing padding characters
    let effective_encoded_len = encoded_len - padding_count;

    // Formula: decoded_size = (effective_encoded_len * 3) / 4
    (effective_encoded_len * 3) / 4
}
