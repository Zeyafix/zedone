//! Stateless encoding only; synthetic text grants no pairing or access rights.

use qrcodegen::{QrCode, QrCodeEcc, QrSegment};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct QrRequest {
    pub text: String,
}

#[derive(Deserialize, Serialize, ToSchema)]
pub struct QrResponse {
    pub modules: Vec<Vec<bool>>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum QrError {
    TooLong,
}

/// Encode every UTF-8 byte, without normalization, truncation or network access.
#[utoipa::path(
    post,
    path = "/prototype/qr",
    operation_id = "qr",
    request_body = QrRequest,
    responses(
        (status = 200, description = "QR modules without quiet zone", body = QrResponse),
        (status = 422, description = "Entire text cannot fit in a QR", body = QrError)
    )
)]
pub fn qr(request: QrRequest) -> Result<QrResponse, QrError> {
    // Even version 40 at the lowest ECC cannot hold more than 2953 byte-mode
    // bytes. Reject larger input before allocating segment bits; the library
    // checks the actual capacity at Medium ECC, including the UTF-8 ECI header.
    if request.text.len() > 2953 {
        return Err(QrError::TooLong);
    }
    let segments = [
        QrSegment::make_eci(26),
        QrSegment::make_bytes(request.text.as_bytes()),
    ];
    let code =
        QrCode::encode_segments(&segments, QrCodeEcc::Medium).map_err(|_| QrError::TooLong)?;
    Ok(QrResponse {
        modules: (0..code.size())
            .map(|y| (0..code.size()).map(|x| code.get_module(x, y)).collect())
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capacity_is_checked_in_bytes_without_truncation() {
        for text in ["x".repeat(2330), "Ё".repeat(1165)] {
            let code = qr(QrRequest { text }).unwrap();
            assert_eq!(code.modules.len(), 177);
            assert!(code.modules.iter().all(|row| row.len() == 177));
        }
        for text in ["x".repeat(2331), "Ё".repeat(1166), "x".repeat(100_000)] {
            assert!(matches!(qr(QrRequest { text }), Err(QrError::TooLong)));
        }
    }
}
