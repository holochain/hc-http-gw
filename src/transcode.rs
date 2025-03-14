//! Functions to transcode call payloads and responses.
//!
//! The incoming HTTP request's payload is a base64 encoded JSON string, which has
//! to be transcoded to `ExternIO` to be passed through as zome call payload.
//!
//! On the way out, the zome call response is `ExternIO` encoded and needs to be converted
//! to a JSON string.

use crate::{HcHttpGatewayError, HcHttpGatewayResult};
use base64::{prelude::BASE64_URL_SAFE, Engine};
use holochain_client::ConductorApiError;
use holochain_types::prelude::ExternIO;

/// Function to transcode an optional base64 encoded payload to Holochain serialized bytes
/// (type `ExternIO`). If no payload is passed in, a unit value will be serialized.
pub fn base64_json_to_hsb(
    maybe_base64_encoded_payload: Option<String>,
) -> HcHttpGatewayResult<ExternIO> {
    let json_payload = if let Some(base64_encoded_payload) = maybe_base64_encoded_payload {
        let base64_decoded_payload =
            BASE64_URL_SAFE
                .decode(base64_encoded_payload)
                .map_err(|_| {
                    HcHttpGatewayError::RequestMalformed("Invalid base64 encoding".to_string())
                })?;
        serde_json::from_slice::<serde_json::Value>(&base64_decoded_payload)
            .map_err(|_| HcHttpGatewayError::RequestMalformed("Invalid JSON value".to_string()))?
    } else {
        serde_json::Value::Null
    };
    let msgpack_encoded_payload = ExternIO::encode(json_payload).map_err(|err| {
        HcHttpGatewayError::RequestMalformed(format!("Failure to serialize payload - {err}"))
    })?;
    Ok(msgpack_encoded_payload)
}

/// Function to transcode a zome call response encoded as Holochain serialized bytes (type `ExternIO`)
/// to a JSON string.
pub fn hsb_to_json(hsb_encoded_response: &ExternIO) -> HcHttpGatewayResult<String> {
    let json_value = hsb_encoded_response
        .decode::<serde_json::Value>()
        .map_err(|err| {
            HcHttpGatewayError::HolochainError(ConductorApiError::WebsocketError(err.into()))
        })?;
    Ok(json_value.to_string())
}

#[cfg(test)]
mod tests {
    use crate::{
        transcode::{base64_json_to_hsb, hsb_to_json},
        HcHttpGatewayError,
    };
    use assert2::let_assert;
    use base64::{prelude::BASE64_URL_SAFE, Engine};
    use holochain_types::dna::ActionHash;
    use holochain_types::prelude::ExternIO;
    use serde::{Deserialize, Serialize};

    #[test]
    fn happy_no_payload_encode() {
        // No payload needs to be encoded for zome call invocation too. Test that a unit value is encoded.
        let hsb_encoded_payload = base64_json_to_hsb(None).unwrap();

        // Deserializing the serialized bytes to the original struct should succeed.
        hsb_encoded_payload.decode::<()>().unwrap();
    }

    #[test]
    fn happy_base64_json_to_hsb() {
        #[derive(Debug, PartialEq, Deserialize, Serialize)]
        struct ZomeCallPayload {
            field: bool,
        }

        // Create a payload, serialize to JSON and base64 encode.
        let payload = ZomeCallPayload { field: false };
        let json_payload = serde_json::to_string(&payload).unwrap();
        let base64_encoded_payload = BASE64_URL_SAFE.encode(json_payload);

        let hsb_encoded_payload = base64_json_to_hsb(Some(base64_encoded_payload)).unwrap();

        // Deserializing the serialized bytes to the original struct should succeed.
        let decoded_payload = hsb_encoded_payload.decode::<ZomeCallPayload>().unwrap();
        assert_eq!(decoded_payload, payload);
    }

    #[test]
    fn plain_json_to_hsb_fails() {
        #[derive(Debug, PartialEq, Deserialize, Serialize)]
        struct ZomeCallPayload {
            field: bool,
        }

        let payload = ZomeCallPayload { field: false };
        let json_payload = serde_json::to_string(&payload).unwrap();

        let result = base64_json_to_hsb(Some(json_payload));
        let_assert!(HcHttpGatewayError::RequestMalformed(err) = result.unwrap_err());
        assert_eq!(err.to_string(), "Invalid base64 encoding");
    }

    #[test]
    fn invalid_json_to_hsb_fails() {
        let base64_encoded_payload = BASE64_URL_SAFE.encode("invalid");

        let result = base64_json_to_hsb(Some(base64_encoded_payload));
        let_assert!(HcHttpGatewayError::RequestMalformed(err) = result.unwrap_err());
        assert_eq!(err.to_string(), "Invalid JSON value");
    }

    #[test]
    fn happy_hsb_to_json() {
        #[derive(Clone, Debug, Deserialize, Serialize)]
        struct ZomeCallResponse {
            value: Vec<String>,
        }

        let response = ZomeCallResponse {
            value: vec!["value1".to_string(), "value2".to_string()],
        };
        let msgpack_encoded_response = ExternIO::encode(response.clone()).unwrap();

        let json_response = hsb_to_json(&msgpack_encoded_response).unwrap();

        let expected_json_response = serde_json::to_string(&response).unwrap();
        assert_eq!(json_response, expected_json_response);
    }

    // TODO requires https://github.com/serde-rs/json/pull/1247
    #[test]
    fn deserialize_binary() {
        let output = ExternIO::encode(ActionHash::from_raw_32(vec![2; 32])).unwrap();

        let json = hsb_to_json(&output).unwrap();

        assert_eq!(json, "[132,41,36,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,2,32,73,61,253]");
    }
}
