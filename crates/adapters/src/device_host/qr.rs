use super::*;

impl<S: SecretStore + 'static> DeviceHost<S> {
    pub(super) async fn outgoing_qr(
        &self,
        body: &[u8],
        key: &str,
    ) -> CommandResponse<serde_json::Value> {
        let result = async {
            self.authorize(key).await?;
            if body.len() > MAX_REQUEST_BYTES {
                return Err(ErrorCode::InvalidRequest);
            }
            #[derive(serde::Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Input {
                address: SocketAddr,
            }
            let input: Input =
                serde_json::from_slice(body).map_err(|_| ErrorCode::InvalidRequest)?;
            let outgoing = match self.outgoing_offer(false, key).await {
                CommandResponse::Ok(value) => value.result,
                CommandResponse::Error(value) => return Err(value.error.code),
            };
            let offer = outgoing["offer"].as_str().ok_or(ErrorCode::NotFound)?;
            let qr = crate::pairing::qr::encode(input.address, offer)?;
            Ok(serde_json::json!({
                "text": qr.text, "modules": qr.modules, "invitation": outgoing["invitation"]
            }))
        }
        .await;
        result
            .map(|result| Execution {
                result,
                replayed: false,
            })
            .map_err(|code| CommandError { code })
            .into()
    }
}
