use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use utoipa::{OpenApi, ToSchema};
use zedone_core::{
    ApiDoc,
    command::{Command, CommandResult},
    error::{CommandError, ErrorCode},
    replay::{Execution, StateVersion, canonical_json},
    wire::{CommandRequest, CommandResponse, decode_request},
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

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
struct WireFixture {
    title: String,
    expected: StateVersion,
}

impl Command for WireFixture {
    const NAME: &'static str = "fixture.change.v1";
    type Output = String;
}

fn wire_request() -> Value {
    json!({
        "id": vec![255; 16], "operation": WireFixture::NAME,
        "parameters": {"title": "Ёж, cafe\u{301}, 😀\n\"текст\"", "expected": vec![255; 32]}
    })
}

#[test]
fn wire_roundtrip_preserves_intention_and_original_conditions() {
    let wire = wire_request();
    let request: CommandRequest<WireFixture> =
        decode_request(&serde_json::to_vec(&wire).unwrap()).unwrap();
    assert_eq!(request.id, [255; 16]);
    assert_eq!(request.parameters.expected, StateVersion([255; 32]));
    assert_eq!(request.parameters.title, wire["parameters"]["title"]);
    assert_eq!(serde_json::to_value(&request).unwrap(), wire);
    let delivered: CommandRequest<WireFixture> =
        decode_request(&serde_json::to_vec(&request).unwrap()).unwrap();
    assert_eq!(delivered.id, request.id);
    assert_eq!(
        canonical_json(&delivered.parameters),
        canonical_json(&request.parameters)
    );
}

#[test]
fn malformed_requests_and_attempts_to_supply_trusted_context_are_rejected() {
    let wire = wire_request();
    let mut cases = vec![json!(null), json!([]), json!({})];
    for field in ["id", "operation", "parameters"] {
        let mut changed = wire.clone();
        changed.as_object_mut().unwrap().remove(field);
        cases.push(changed);
    }
    for field in ["subject", "credential", "grants", "admin", "replayed"] {
        let mut changed = wire.clone();
        changed[field] = json!(true);
        cases.push(changed);
    }
    for id in [
        json!(vec![0; 16]),
        json!(vec![1; 15]),
        json!(vec![1; 17]),
        json!(vec![256; 16]),
        json!(vec![-1; 16]),
        json!(vec![1.5; 16]),
        json!("00112233445566778899aabbccddeeff"),
        json!(null),
    ] {
        let mut changed = wire.clone();
        changed["id"] = id;
        cases.push(changed);
    }
    for version in [
        json!(vec![0; 31]),
        json!(vec![0; 33]),
        json!(vec![256; 32]),
        json!(vec![-1; 32]),
        json!(null),
    ] {
        let mut changed = wire.clone();
        changed["parameters"]["expected"] = version;
        cases.push(changed);
    }
    for operation in [json!(""), json!("other.operation"), json!(42), json!(null)] {
        let mut changed = wire.clone();
        changed["operation"] = operation;
        cases.push(changed);
    }
    let mut changed = wire.clone();
    changed["parameters"]["admin"] = json!(true);
    cases.push(changed);
    for input in cases {
        assert_eq!(
            decode_request::<WireFixture>(&serde_json::to_vec(&input).unwrap())
                .unwrap_err()
                .code,
            ErrorCode::InvalidRequest,
            "{input}"
        );
    }
    let raw = wire.to_string();
    for input in [
        format!("{raw} trailing"),
        format!("{raw}{raw}"),
        raw.replacen("{", "{\"operation\":\"fixture.change.v1\",", 1),
        raw.replace("\"title\":", "\"title\":\"duplicate\",\"title\":"),
        "{".into(),
    ] {
        assert_eq!(
            decode_request::<WireFixture>(input.as_bytes())
                .unwrap_err()
                .code,
            ErrorCode::InvalidRequest
        );
    }
}

#[test]
fn wire_responses_distinguish_original_completion_replay_and_unknown_outcome() {
    for replayed in [false, true] {
        let response = CommandResponse::from(Ok(Execution {
            result: json!(null),
            replayed,
        }));
        let wire = json!({"status": "ok", "result": null, "replayed": replayed});
        assert_eq!(serde_json::to_value(&response).unwrap(), wire);
        assert_eq!(
            serde_json::from_value::<CommandResponse<Value>>(wire).unwrap(),
            response
        );
    }
    for (code, name) in ERRORS {
        let response = CommandResponse::<Value>::from(Err(CommandError { code }));
        let wire = json!({"status": "error", "error": {"code": name}});
        assert_eq!(serde_json::to_value(&response).unwrap(), wire);
        assert_eq!(
            serde_json::from_value::<CommandResponse<Value>>(wire).unwrap(),
            response
        );
    }
    for wire in [
        json!({"Ok": "old rust representation"}),
        json!({"status": "ok", "replayed": false}),
        json!({"status": "ok", "result": null}),
        json!({"status": "ok", "result": null, "replayed": false, "error": {"code":"outcome_unknown"}}),
        json!({"status": "error", "error": {"code": "unavailable"}, "result": null}),
        json!({"status": "error", "error": {"code": "unavailable", "details": "secret"}}),
    ] {
        assert!(
            serde_json::from_value::<CommandResponse<Value>>(wire.clone()).is_err(),
            "{wire}"
        );
    }
}

#[test]
fn wire_schemas_match_fixed_byte_arrays_and_strict_response_variants() {
    let doc = serde_json::to_value(ApiDoc::openapi()).unwrap();
    let schemas = &doc["components"]["schemas"];
    for (schema, length) in [
        (&schemas["CommandRequest_Value"]["properties"]["id"], 16),
        (&schemas["StateVersion"], 32),
    ] {
        assert_eq!(schema["minItems"], length);
        assert_eq!(schema["maxItems"], length);
        assert_eq!(schema["items"]["type"], "integer");
        assert_eq!(schema["items"]["minimum"], 0);
        assert_eq!(schema["items"]["maximum"], 255);
    }
    for name in [
        "CommandRequest_Value",
        "CommandSuccess_Value",
        "CommandFailure",
    ] {
        assert_eq!(schemas[name]["additionalProperties"], false, "{name}");
    }
    assert_eq!(doc["paths"], json!({}));
}
