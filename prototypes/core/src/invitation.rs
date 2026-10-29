//! Synthetic input inspection only: no connection, address lookup or credentials.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct Invitation {
    pub address: String,
    pub code: String,
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(tag = "source", rename_all = "snake_case", deny_unknown_fields)]
pub enum InvitationRequest {
    Qr { text: String },
    Manual { address: String, code: String },
}

#[derive(Debug, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum InvitationError {
    InvalidData,
}

/// The same inspection for camera content, manual input and direct agent calls.
/// These fields deliberately do not specify the future pairing protocol.
#[utoipa::path(
    post, path = "/prototype/invitation", operation_id = "invitation",
    request_body = InvitationRequest,
    responses(
        (status = 200, description = "Synthetic fields, unchanged; no connection", body = Invitation),
        (status = 422, description = "Expected nonempty synthetic address/code", body = InvitationError)
    )
)]
pub fn invitation(request: InvitationRequest) -> Result<Invitation, InvitationError> {
    let data = match request {
        InvitationRequest::Qr { text } => {
            serde_json::from_str(&text).map_err(|_| InvitationError::InvalidData)?
        }
        InvitationRequest::Manual { address, code } => Invitation { address, code },
    };
    if data.address.trim().is_empty() || data.code.trim().is_empty() {
        return Err(InvitationError::InvalidData);
    }
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_inputs_preserve_exact_fields_and_reject_incomplete_data() {
        let address = " http://192.0.2.1:4317/ёж ";
        let code = " café cafe\u{301} 😀 ";
        let manual = invitation(InvitationRequest::Manual {
            address: address.into(),
            code: code.into(),
        })
        .unwrap();
        let qr = invitation(InvitationRequest::Qr {
            text: serde_json::to_string(&manual).unwrap(),
        })
        .unwrap();
        assert_eq!(qr.address, address);
        assert_eq!(qr.code, code);
        for text in [
            "",
            "{}",
            "null",
            r#"{"address":"x","code":""}"#,
            r#"{"address":"x","code":"y","extra":true}"#,
        ] {
            assert!(invitation(InvitationRequest::Qr { text: text.into() }).is_err());
        }
        assert!(
            invitation(InvitationRequest::Manual {
                address: "  ".into(),
                code: "x".into()
            })
            .is_err()
        );
    }
}
