//! Synthetic, transport-independent contract for the P01 prototypes.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use utoipa::{OpenApi, ToSchema};

pub mod storage;

#[derive(Deserialize, Serialize, ToSchema)]
pub struct EchoRequest {
    pub text: String,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct EchoResponse {
    pub text: String,
}

/// Return synthetic text unchanged, without storing it.
#[utoipa::path(
    post,
    path = "/prototype/echo",
    operation_id = "echo",
    request_body = EchoRequest,
    responses((status = 200, description = "Text returned unchanged", body = EchoResponse))
)]
pub fn echo(request: EchoRequest) -> EchoResponse {
    EchoResponse { text: request.text }
}

#[derive(OpenApi)]
#[openapi(
    info(title = "ZeDone synthetic prototype", version = "0.0.0"),
    paths(echo, storage::storage)
)]
pub struct ApiDoc;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    #[test]
    fn echo_preserves_json_text_including_unicode_and_empty_input() {
        for text in ["", "Ёж, ЕЖ, cafe\u{301}, 😀\n\"строка\"\\\t"] {
            let input = json!({"text": text});
            let request: EchoRequest = serde_json::from_value(input.clone()).unwrap();
            assert_eq!(serde_json::to_value(echo(request)).unwrap(), input);
        }
    }

    #[test]
    fn request_rejects_missing_null_and_non_string_text() {
        for input in [json!({}), json!({"text": null}), json!({"text": 42})] {
            assert!(serde_json::from_value::<EchoRequest>(input).is_err());
        }
    }

    #[test]
    fn openapi_describes_the_json_boundary() {
        let doc: Value = serde_json::to_value(ApiDoc::openapi()).unwrap();
        assert_eq!(doc["openapi"], "3.1.0");
        let operation = &doc["paths"]["/prototype/echo"]["post"];
        assert_eq!(operation["operationId"], "echo");
        assert_eq!(operation["requestBody"]["required"], true);
        assert_eq!(
            operation["requestBody"]["content"]["application/json"]["schema"]["$ref"],
            "#/components/schemas/EchoRequest"
        );
        assert_eq!(
            operation["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
            "#/components/schemas/EchoResponse"
        );
        for name in ["EchoRequest", "EchoResponse"] {
            let schema = &doc["components"]["schemas"][name];
            assert_eq!(schema["required"], json!(["text"]));
            assert_eq!(schema["properties"]["text"]["type"], "string");
        }
    }
}
