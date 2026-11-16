use qrcodegen::{QrCode, QrCodeEcc, QrSegment};

use crate::error::ErrorCode;

pub fn encode(text: &str) -> Result<Vec<Vec<bool>>, ErrorCode> {
    if text.len() > 2330 {
        return Err(ErrorCode::InvalidRequest);
    }
    let segments = [
        QrSegment::make_eci(26),
        QrSegment::make_bytes(text.as_bytes()),
    ];
    let code = QrCode::encode_segments(&segments, QrCodeEcc::Medium)
        .map_err(|_| ErrorCode::InvalidRequest)?;
    Ok((0..code.size())
        .map(|y| (0..code.size()).map(|x| code.get_module(x, y)).collect())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capacity_is_checked_in_utf8_bytes_without_truncation() {
        for text in ["x".repeat(2330), "Ё".repeat(1165)] {
            let modules = encode(&text).unwrap();
            assert_eq!(modules.len(), 177);
            assert!(modules.iter().all(|row| row.len() == 177));
        }
        for text in ["x".repeat(2331), "Ё".repeat(1166), "x".repeat(100_000)] {
            assert_eq!(encode(&text), Err(ErrorCode::InvalidRequest));
        }
    }
}
