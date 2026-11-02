//! Input-only alert material; complete URLs and authentication are vault-only secrets.
//! Telegram normalizes into a fixed Bot API URL plus chat ID in the vault-only bearer slot.
//! That slot is never used as HTTP authorization for a Telegram destination.
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
        let input = AlertInput::deserialize(d)
            .map_err(|_| serde::de::Error::custom("invalid alert material"))?;
        match input {
            AlertInput::Webhook(value) => Ok(Self(value)),
            AlertInput::Telegram(value) => value
                .into_material()
                .map(Self)
                .map_err(|_| serde::de::Error::custom("invalid alert material")),
        }
    }
}

/// Telegram input is never serializable or Debug; normalized material uses the existing vault format.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TelegramSecret {
    pub token: String,
    pub chat_id: String,
}
impl Drop for TelegramSecret {
    fn drop(&mut self) {
        self.token.zeroize();
        self.chat_id.zeroize();
    }
}
impl TelegramSecret {
    pub fn into_material(self) -> Result<WebhookSecret, &'static str> {
        if !valid_bot_token(&self.token) || !valid_chat_id(&self.chat_id) {
            return Err("invalid alert material");
        }
        Ok(WebhookSecret {
            url: format!("https://api.telegram.org/bot{}/sendMessage", self.token),
            bearer: Some(self.chat_id.clone()),
        })
    }
}
pub fn valid_bot_token(token: &str) -> bool {
    let Some((id, key)) = token.split_once(':') else {
        return false;
    };
    !id.is_empty()
        && id.len() <= 20
        && id.bytes().all(|c| c.is_ascii_digit())
        && !key.is_empty()
        && key.len() <= 256
        && key
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-'))
}
pub fn valid_chat_id(chat: &str) -> bool {
    chat.parse::<i64>()
        .is_ok_and(|id| id != 0 && id.to_string() == chat)
}

#[derive(Deserialize)]
#[serde(untagged)]
enum AlertInput {
    Webhook(WebhookSecret),
    Telegram(TelegramSecret),
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn old_input_and_typed_telegram_are_redacted_and_validate_without_io() {
        let old: IncomingWebhook =
            serde_json::from_str(r#"{"url":"https://example.com/private","bearer":null}"#).unwrap();
        assert_eq!(old.0.url, "https://example.com/private");
        let telegram: IncomingWebhook =
            serde_json::from_str(r#"{"token":"123:synthetic","chat_id":"-123"}"#).unwrap();
        assert!(!format!("{:?}", telegram.0).contains("synthetic"));
        for raw in [
            r#"{"token":"123:synthetic/escape","chat_id":"-123"}"#,
            r#"{"token":"123:synthetic","chat_id":"@recipient"}"#,
            r#"{"token":"123:synthetic","chat_id":"0"}"#,
            r#"{"token":"123:synthetic","chat_id":123}"#,
        ] {
            let err = serde_json::from_str::<IncomingWebhook>(raw)
                .err()
                .unwrap()
                .to_string();
            assert!(!err.contains("synthetic"));
            assert!(!err.contains("recipient"));
        }
    }
}
