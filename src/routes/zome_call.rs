use crate::{
    service::AppState, transcode::base64_json_to_hsb, HcHttpGatewayError, HcHttpGatewayResult,
};
use axum::extract::{FromRequestParts, Path, Query, State};
use holochain_types::{dna::DnaHash, prelude::ExternIO};
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
        let Path(raw_params) = Path::<RawZomeCallParams>::from_request_parts(parts, state).await?;
        let RawZomeCallParams {
            dna_hash,
            coordinator_identifier,
            zome_name,
            fn_name,
        } = raw_params;
        // Check DNA hash validity.
        let dna_hash = DnaHash::try_from(dna_hash)?;
        // Reject identifiers longer than the maximum length.
        if coordinator_identifier.len() > MAX_IDENTIFIER_CHARS as usize {
            return Err(HcHttpGatewayError::IdentifierLengthExceeded(
                coordinator_identifier,
                MAX_IDENTIFIER_CHARS,
            ));
        }
        if zome_name.len() > MAX_IDENTIFIER_CHARS as usize {
            return Err(HcHttpGatewayError::IdentifierLengthExceeded(
                zome_name,
                MAX_IDENTIFIER_CHARS,
            ));
        }
        if fn_name.len() > MAX_IDENTIFIER_CHARS as usize {
            return Err(HcHttpGatewayError::IdentifierLengthExceeded(
                fn_name,
                MAX_IDENTIFIER_CHARS,
            ));
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
) -> HcHttpGatewayResult<()> {
    let ZomeCallParams {
        coordinator_identifier,
        zome_name,
        fn_name,
        ..
    } = params;
    // Check payload byte length does not exceed configured maximum.
    if let Some(payload) = &query.payload {
        if payload.as_bytes().len() > state.configuration.payload_limit_bytes as usize {
            return Err(HcHttpGatewayError::PayloadSizeLimitError {
                size: payload.as_bytes().len() as u32,
                limit: state.configuration.payload_limit_bytes,
            });
        }
    }
    // Check if function name is allowed.
    if !state
        .configuration
        .is_function_allowed(&coordinator_identifier, &zome_name, &fn_name)
    {
        return Err(HcHttpGatewayError::UnauthorizedFunction {
            app_id: coordinator_identifier,
            zome_name,
            fn_name,
        });
    }

    // Transcode to payload from base64 encoded JSON to ExternIO.
    let _zome_call_payload = if let Some(payload) = &query.payload {
        base64_json_to_hsb(payload)?
    } else {
        ExternIO::encode(())?
    };

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        config::{AllowedFns, Configuration},
        router::tests::TestRouter,
        routes::zome_call::MAX_IDENTIFIER_CHARS,
    };
    use base64::{prelude::BASE64_URL_SAFE, Engine};
    use holochain::sweettest::SweetConductor;
    use reqwest::StatusCode;
    use std::collections::HashMap;

    // DnaHash::from_raw_32(vec![1; 32]).to_string()
    const DNA_HASH: &str = "uhC0kAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQEBAQF-z86-";

    #[tokio::test(flavor = "multi_thread")]
    async fn valid_dna_hash_is_accepted() {
        let router = TestRouter::new().await;
        let uri = format!("/{DNA_HASH}/coordinator/zome_name/fn_name");
        let (status_code, _) = router.request(&uri).await;
        assert_eq!(status_code, StatusCode::OK);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn invalid_dna_hash_is_rejected() {
        let router = TestRouter::new().await;
        let invalid_dna_hash = "thisaintnodnahash";
        let uri = format!("/{invalid_dna_hash}/coordinator/zome_name/fn_name");
        let (status_code, body) = router.request(&uri).await;
        assert_eq!(status_code, StatusCode::BAD_REQUEST);
        assert_eq!(body, r#"{"error":"Invalid base64 DNA hash"}"#);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn coordinator_identifier_with_excess_length_is_rejected() {
        let router = TestRouter::new().await;
        let coordinator = "12345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901";
        let uri = format!("/{DNA_HASH}/{coordinator}/zome_name/fn_name");
        let (status_code, body) = router.request(&uri).await;
        assert_eq!(status_code, StatusCode::BAD_REQUEST);
        assert_eq!(
            body,
            format!(
                r#"{{"error":"Identifier {coordinator} longer than {MAX_IDENTIFIER_CHARS} characters"}}"#
            )
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn zome_name_with_excess_length_is_rejected() {
        let router = TestRouter::new().await;
        let zome_name = "12345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901";
        let uri = format!("/{DNA_HASH}/coordinator/{zome_name}/fn_name");
        let (status_code, body) = router.request(&uri).await;
        assert_eq!(status_code, StatusCode::BAD_REQUEST);
        assert_eq!(
            body,
            format!(
                r#"{{"error":"Identifier {zome_name} longer than {MAX_IDENTIFIER_CHARS} characters"}}"#
            )
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn function_name_with_excess_length_is_rejected() {
        let router = TestRouter::new().await;
        let fn_name = "12345678901234567890123456789012345678901234567890123456789012345678901234567890123456789012345678901";
        let uri = format!("/{DNA_HASH}/coordinator/zome_name/{fn_name}");
        let (status_code, body) = router.request(&uri).await;
        assert_eq!(status_code, StatusCode::BAD_REQUEST);
        assert_eq!(
            body,
            format!(
                r#"{{"error":"Identifier {fn_name} longer than {MAX_IDENTIFIER_CHARS} characters"}}"#
            )
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn unauthorized_function_name_is_rejected() {
        // Only one allowed function "fn_name" in test router.
        let router = TestRouter::new().await;
        let fn_name = "unauthorized_fn";
        let uri = format!("/{DNA_HASH}/coordinator/zome_name/{fn_name}");
        let (status_code, body) = router.request(&uri).await;
        assert_eq!(status_code, StatusCode::FORBIDDEN);
        assert_eq!(
            body,
            format!(
                r#"{{"error":"Function {fn_name} in zome zome_name in app coordinator is not allowed"}}"#
            )
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn payload_with_excess_length_is_rejected() {
        let mut allowed_fns = HashMap::new();
        allowed_fns.insert("coordinator".to_string(), AllowedFns::All);

        let sweet_conductor = SweetConductor::from_standard_config().await;
        let admin_port = sweet_conductor
            .get_arbitrary_admin_websocket_port()
            .unwrap();

        let config = Configuration::try_new(
            format!("ws://127.0.0.1:{admin_port}").as_str(),
            "10",
            "",
            allowed_fns,
            "",
            "",
        )
        .unwrap();
        let router = TestRouter::new_with_config(config).await;
        let payload = BASE64_URL_SAFE.encode(vec![1; 11]);
        let payload_length = payload.len();
        let uri = format!("/{DNA_HASH}/coordinator/zome_name/fn_name?payload={payload}");
        let (status_code, body) = router.request(&uri).await;
        assert_eq!(status_code, StatusCode::BAD_REQUEST);
        assert_eq!(
            body,
            format!(
                r#"{{"error":"Payload size ({payload_length} bytes) exceeds maximum allowed size (10 bytes)"}}"#
            )
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn payload_with_invalid_base64_encoding_is_rejected() {
        let router = TestRouter::new().await;
        let payload = "$%&#";
        let uri = format!("/{DNA_HASH}/coordinator/zome_name/fn_name?payload={payload}");
        let (status_code, body) = router.request(&uri).await;
        assert_eq!(status_code, StatusCode::BAD_REQUEST);
        assert_eq!(
            body,
            r#"{"error":"Failed to decode base64 encoded string"}"#
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn payload_with_invalid_json_is_rejected() {
        let router = TestRouter::new().await;
        let payload = BASE64_URL_SAFE.encode("{invalid}");
        let uri = format!("/{DNA_HASH}/coordinator/zome_name/fn_name?payload={payload}");
        let (status_code, body) = router.request(&uri).await;
        assert_eq!(status_code, StatusCode::BAD_REQUEST);
        assert_eq!(body, r#"{"error":"Payload contains invalid JSON"}"#);
    }
}
