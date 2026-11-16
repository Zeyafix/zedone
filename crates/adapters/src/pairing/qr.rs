use std::net::SocketAddr;

use serde::{Deserialize, Serialize};
use zedone_core::error::ErrorCode;

use super::Offer;

const FORMAT: &str = "zedone-pairing-v1";

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Payload {
    format: String,
    address: SocketAddr,
    offer: String,
}

#[derive(Serialize)]
pub struct PairingInput {
    pub address: SocketAddr,
    pub offer: String,
}

#[derive(Serialize)]
pub struct QrPresentation {
    pub text: String,
    pub modules: Vec<Vec<bool>>,
}

fn validate(address: SocketAddr, offer: &str) -> Result<(), ErrorCode> {
    if address.port() == 0 || address.ip().is_unspecified() {
        return Err(ErrorCode::InvalidRequest);
    }
    Offer::decode(offer.as_bytes())?;
    Ok(())
}

pub fn encode(address: SocketAddr, offer: &str) -> Result<QrPresentation, ErrorCode> {
    validate(address, offer)?;
    let text = serde_json::to_string(&Payload {
        format: FORMAT.to_owned(),
        address,
        offer: offer.to_owned(),
    })
    .map_err(|_| ErrorCode::InvalidRequest)?;
    let modules = zedone_core::qr::encode(&text)?;
    Ok(QrPresentation { text, modules })
}

pub fn decode(text: &str) -> Result<PairingInput, ErrorCode> {
    if text.len() > 1024 {
        return Err(ErrorCode::InvalidRequest);
    }
    let input: Payload = serde_json::from_str(text).map_err(|_| ErrorCode::InvalidRequest)?;
    if input.format != FORMAT {
        return Err(ErrorCode::InvalidRequest);
    }
    validate(input.address, &input.offer)?;
    Ok(PairingInput {
        address: input.address,
        offer: input.offer,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::channel::Fingerprint;

    #[test]
    fn qr_preserves_offer_and_addresses_without_accepting_foreign_or_partial_data() {
        let secret = Offer::new("a".repeat(32), Fingerprint([9; 32]), &[7; 32])
            .unwrap()
            .encode();
        let offer = std::str::from_utf8(&secret).unwrap();
        for address in ["100.127.255.254:65535", "[2001:db8::1]:45821"] {
            let address = address.parse().unwrap();
            let qr = encode(address, offer).unwrap();
            let input = decode(&qr.text).unwrap();
            assert_eq!(input.address, address);
            assert_eq!(input.offer, offer);
            let mut payload: serde_json::Value = serde_json::from_str(&qr.text).unwrap();
            let mut wrong_type = payload.clone();
            wrong_type["format"] = serde_json::json!({"zedone-pairing-v1":null});
            assert!(decode(&wrong_type.to_string()).is_err());
            for (field, value) in [
                ("format", "zedone-pairing-v2"),
                ("address", "0.0.0.0:123"),
                ("address", "[::]:123"),
                ("address", "127.0.0.1:0"),
                ("address", "https://example.com:123/path"),
                ("offer", "zd1:incomplete"),
                ("offer", ""),
                ("extra", "ignored?"),
            ] {
                let mut invalid = payload.clone();
                invalid[field] = value.into();
                assert!(matches!(
                    decode(&invalid.to_string()),
                    Err(ErrorCode::InvalidRequest)
                ));
            }
            payload.as_object_mut().unwrap().remove("format");
            assert!(decode(&payload.to_string()).is_err());
            let duplicate = qr
                .text
                .replacen('{', "{\"format\":\"zedone-pairing-v1\",", 1);
            assert!(decode(&duplicate).is_err());
        }
        for text in ["".to_owned(), "{}".into(), "null".into(), "x".repeat(1025)] {
            assert!(matches!(decode(&text), Err(ErrorCode::InvalidRequest)));
        }
        assert!(encode("0.0.0.0:123".parse().unwrap(), offer).is_err());
        assert!(encode("127.0.0.1:0".parse().unwrap(), offer).is_err());
    }
}
