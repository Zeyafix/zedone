use serde::Deserialize;
use tauri::{
    Manager, Runtime,
    plugin::{PluginHandle, TauriPlugin},
};
use zedone_adapters::secrets::{MAX_SECRET_BYTES, Secret, SecretError, SecretStore, Slot};

pub struct AndroidSecrets<R: Runtime> {
    handle: PluginHandle<R>,
    namespace: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Reply {
    status: String,
    bytes: Option<Vec<u8>>,
}
impl<R: Runtime> AndroidSecrets<R> {
    pub fn new(app: &tauri::AppHandle<R>, namespace: [u8; 32]) -> Result<Self, SecretError> {
        Ok(Self {
            handle: app
                .try_state::<PluginHandle<R>>()
                .ok_or(SecretError::Unavailable)?
                .inner()
                .clone(),
            namespace: namespace.iter().map(|b| format!("{b:02x}")).collect(),
        })
    }
    async fn access(
        &self,
        operation: &str,
        slot: Slot,
        bytes: Option<&[u8]>,
    ) -> Result<Reply, SecretError> {
        let reply: Reply = self.handle.run_mobile_plugin_async("access", serde_json::json!({
            "operation": operation, "namespace": self.namespace, "slot": slot.name(), "bytes": bytes,
        })).await.map_err(|_| SecretError::Unavailable)?;
        match reply.status.as_str() {
            "ok" | "missing" => Ok(reply),
            "already_exists" => Err(SecretError::AlreadyExists),
            "invalid_secret" => Err(SecretError::InvalidSecret),
            _ => Err(SecretError::Unavailable),
        }
    }
}
impl<R: Runtime> SecretStore for AndroidSecrets<R> {
    async fn load(&self, slot: Slot) -> Result<Option<Secret>, SecretError> {
        let reply = self.access("load", slot, None).await?;
        if reply.status == "missing" && reply.bytes.is_none() {
            return Ok(None);
        }
        let bytes = Secret::new(reply.bytes.ok_or(SecretError::Unavailable)?);
        if reply.status != "ok" || bytes.is_empty() || bytes.len() > MAX_SECRET_BYTES {
            return Err(SecretError::InvalidSecret);
        }
        Ok(Some(bytes))
    }
    async fn create(&self, slot: Slot, bytes: &[u8]) -> Result<(), SecretError> {
        if bytes.is_empty() || bytes.len() > MAX_SECRET_BYTES {
            return Err(SecretError::InvalidSecret);
        }
        let reply = self.access("create", slot, Some(bytes)).await?;
        if reply.status != "ok" {
            return Err(SecretError::Unavailable);
        }
        Ok(())
    }
    async fn remove(&self, slot: Slot) -> Result<(), SecretError> {
        let reply = self.access("remove", slot, None).await?;
        if reply.status != "ok" {
            return Err(SecretError::Unavailable);
        }
        Ok(())
    }
}

pub fn plugin<R: Runtime>() -> TauriPlugin<R> {
    tauri::plugin::Builder::new("host-secrets")
        .setup(|app, api| {
            app.manage(api.register_android_plugin("app.zedone.local", "SecretsPlugin")?);
            Ok(())
        })
        .invoke_handler(|invoke| {
            invoke.resolver.reject("host operation unavailable");
            true
        })
        .build()
}
