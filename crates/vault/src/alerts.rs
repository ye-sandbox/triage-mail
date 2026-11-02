//! Input-only alert material; complete URLs and authentication are vault-only secrets.
use serde::{Deserialize, Serialize};
use zeroize::Zeroize;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WebhookSecret {
    pub url: String,
    pub bearer: Option<String>,
}
impl std::fmt::Debug for WebhookSecret {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("WebhookSecret([REDACTED])")
    }
}
impl Drop for WebhookSecret {
    fn drop(&mut self) {
        self.url.zeroize();
        self.bearer.zeroize();
    }
}
/// Serde diagnostics must never echo secret strings through a malformed IPC argument.
pub struct IncomingWebhook(pub WebhookSecret);
impl<'de> Deserialize<'de> for IncomingWebhook {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        WebhookSecret::deserialize(d)
            .map(Self)
            .map_err(|_| serde::de::Error::custom("invalid webhook material"))
    }
}
