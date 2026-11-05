use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use utoipa::{OpenApi, ToSchema};
use zedone_core::{
    ApiDoc,
    command::{Command, CommandResult},
    error::{CommandError, ErrorCode},
};

const ERRORS: [(ErrorCode, &str); 7] = [
    (ErrorCode::InvalidRequest, "invalid_request"),
    (ErrorCode::AccessDenied, "access_denied"),
    (ErrorCode::NotFound, "not_found"),
    (ErrorCode::StaleState, "stale_state"),
    (ErrorCode::CommandMismatch, "command_mismatch"),
    (ErrorCode::Unavailable, "unavailable"),
    (ErrorCode::OutcomeUnknown, "outcome_unknown"),
];

#[test]
fn errors_have_distinct_strict_machine_readable_codes() {
    for (error, code) in ERRORS {
        let error = CommandError { code: error };
        let wire = json!({"code": code});
        assert_eq!(serde_json::to_value(error).unwrap(), wire);
        assert_eq!(serde_json::from_value::<CommandError>(wire).unwrap(), error);
        assert!(
            serde_json::from_value::<CommandError>(json!({
                "code": code, "details": "private storage path or credentials"
            }))
            .is_err()
        );
    }
    for wire in [
        json!({}),
        json!({"code": "future_error"}),
        json!({"code": null}),
    ] {
        assert!(serde_json::from_value::<CommandError>(wire).is_err());
    }
}

#[test]
fn openapi_matches_serialized_errors_and_exposes_no_unimplemented_operations() {
    let document = serde_json::to_value(ApiDoc::openapi()).unwrap();
    assert_eq!(document["openapi"], "3.1.0");
    assert_eq!(document["paths"], json!({}));
    let schema = &document["components"]["schemas"]["CommandError"];
    assert_eq!(schema["required"], json!(["code"]));
    assert_eq!(schema["additionalProperties"], false);
    assert_eq!(
        schema["properties"]["code"]["$ref"],
        "#/components/schemas/ErrorCode"
    );
    let mut codes: Vec<_> = document["components"]["schemas"]["ErrorCode"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|value| value.as_str().unwrap())
        .collect();
    let mut expected: Vec<_> = ERRORS.iter().map(|(_, code)| *code).collect();
    codes.sort_unstable();
    expected.sort_unstable();
    assert_eq!(codes, expected);
}

// Test-only command: no synthetic operation is added to the product API.
#[derive(Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
struct ReadFixture {
    title: String,
}

impl Command for ReadFixture {
    const NAME: &'static str = "read_fixture";
    type Output = String;
}

#[test]
fn typed_command_contract_preserves_text_and_its_associated_output() {
    fn handle(request: ReadFixture) -> CommandResult<<ReadFixture as Command>::Output> {
        Ok(request.title)
    }
    let wire = json!({"title": "Ёж, cafe\u{301}, 😀\n\"текст\""});
    let request: ReadFixture = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(ReadFixture::NAME, "read_fixture");
    let output: String = handle(request).unwrap();
    assert_eq!(Value::String(output), wire["title"]);
    assert!(serde_json::from_value::<ReadFixture>(json!({"title": "x", "admin": true})).is_err());
}
