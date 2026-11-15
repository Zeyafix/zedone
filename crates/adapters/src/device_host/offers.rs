//! Outgoing manual code: persisted before publication, never in a receipt.
use super::*;
use crate::secrets::Slot;
use zedone_core::invitations::{InvitationSnapshot, InvitationStatus};

impl<S: SecretStore + 'static> DeviceHost<S> {
    pub async fn outgoing_offer(
        &self,
        create: bool,
        key: &str,
    ) -> CommandResponse<serde_json::Value> {
        let result = async {
            let _guard = self.operations.lock().await;
            self.authorize(key).await?;
            let saved = self.store.load(Slot::OutgoingInvitation).await.map_err(|_| ErrorCode::Unavailable)?;
            if let Some(bytes) = saved {
                let previous = Offer::decode(&bytes)?;
                let identity = channel::load_identity(&self.store).await.map_err(|_| ErrorCode::Unavailable)?.ok_or(ErrorCode::Unavailable)?;
                if identity.fingerprint() != previous.fingerprint { return Err(ErrorCode::Unavailable); }
                // Re-publish the exact original create. A crash before its commit
                // must not turn an unpublished code into an abandoned OS record.
                self.publish_offer(&previous, key).await?;
                let snapshot = self.invitation_snapshot(&previous.invitation, key).await?;
                if snapshot.status == InvitationStatus::Open || !create {
                    return Ok(serde_json::json!({"offer": if snapshot.status == InvitationStatus::Open {
                        Some(String::from_utf8(previous.encode().to_vec()).map_err(|_| ErrorCode::Unavailable)?)
                    } else { None }, "invitation": snapshot}));
                }
                self.store.remove(Slot::OutgoingInvitation).await.map_err(|_| ErrorCode::Unavailable)?;
            }
            if !create { return Ok(serde_json::json!({"offer":null,"invitation":null})); }
            let identity = channel::load_identity(&self.store).await.map_err(|_| ErrorCode::Unavailable)?.ok_or(ErrorCode::Unavailable)?;
            let mut id = [0;16];
            let mut code = Secret::new(vec![0;32]);
            getrandom::fill(&mut id).map_err(|_| ErrorCode::Unavailable)?;
            getrandom::fill(&mut code).map_err(|_| ErrorCode::Unavailable)?;
            let invitation = id.iter().map(|b|format!("{b:02x}")).collect();
            let next = Offer::new(invitation, identity.fingerprint(), &code)?;
            let bytes = next.encode();
            self.store.create(Slot::OutgoingInvitation, &bytes).await.map_err(|_| ErrorCode::Unavailable)?;
            let acknowledged = self.store.load(Slot::OutgoingInvitation).await.map_err(|_| ErrorCode::Unavailable)?.ok_or(ErrorCode::Unavailable)?;
            if *acknowledged != *bytes { return Err(ErrorCode::Unavailable); }
            self.publish_offer(&next, key).await?;
            let snapshot = self.invitation_snapshot(&next.invitation, key).await?;
            Ok(serde_json::json!({"offer":String::from_utf8(bytes.to_vec()).map_err(|_| ErrorCode::Unavailable)?,"invitation":snapshot}))
        }.await;
        match result {
            Ok(result) => Ok(Execution {
                result,
                replayed: false,
            })
            .into(),
            Err(code) => failure(code),
        }
    }
    async fn publish_offer(&self, offer: &Offer, key: &str) -> Result<bool, ErrorCode> {
        let id: Vec<u8> = (0..32)
            .step_by(2)
            .map(|i| {
                u8::from_str_radix(&offer.invitation[i..i + 2], 16)
                    .map_err(|_| ErrorCode::Unavailable)
            })
            .collect::<Result<_, _>>()?;
        let body = serde_json::json!({"id":id,"operation":"invitation.create","parameters":{"invitation":offer.invitation,"verifier":offer.verifier()}});
        match self
            .service
            .dispatch("invitation.create", body.to_string().as_bytes(), key)
            .await
        {
            CommandResponse::Ok(value) => Ok(value.replayed),
            CommandResponse::Error(value) => Err(value.error.code),
        }
    }
    async fn invitation_snapshot(
        &self,
        invitation: &str,
        key: &str,
    ) -> Result<InvitationSnapshot, ErrorCode> {
        let mut id = [0; 16];
        getrandom::fill(&mut id).map_err(|_| ErrorCode::Unavailable)?;
        let body = serde_json::json!({"id":id,"operation":"invitation.inspect","parameters":{"invitation":invitation}});
        match self
            .service
            .dispatch("invitation.inspect", body.to_string().as_bytes(), key)
            .await
        {
            CommandResponse::Ok(value) => {
                serde_json::from_value(value.result).map_err(|_| ErrorCode::Unavailable)
            }
            CommandResponse::Error(value) => Err(value.error.code),
        }
    }
}
