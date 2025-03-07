use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct HcGwErrorResponse {
    pub error: String,
}
