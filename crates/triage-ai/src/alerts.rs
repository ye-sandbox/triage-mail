//! Headless alert contracts and explicit export-field selection. No delivery side effects.
use crate::rules::MessageFacts;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub mod connectors;
pub mod schedule;

pub const MAX_PAYLOAD_BYTES: usize = 65_536;
pub const MAX_AGE_MS: i64 = 86_400_000;
pub fn valid_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DestinationKind {
    Local,
    Webhook,
    Telegram,
    Discord,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExportField {
    Account,
    Sender,
    Subject,
    Excerpt,
    ExtractedFacts,
    Attachments,
    Body,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Destination {
    pub id: String,
    pub revision: u32,
    pub name: String,
    pub kind: DestinationKind,
    pub enabled: bool,
    pub fields: Vec<ExportField>,
    pub public_origin: Option<String>,
    pub has_credentials: bool,
}
impl Destination {
    pub fn validate(&self) -> bool {
        valid_id(&self.id)
            && !self.name.trim().is_empty()
            && self.name.len() <= 120
            && !self.name.chars().any(char::is_control)
            && self.revision < i32::MAX as u32
            && self.fields.len() <= 7
            && (self.kind != DestinationKind::Local || self.fields.is_empty())
    }
}
/// Only fixed outcomes enter durable history; never carry HTTP/error response text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeliveryOutcome {
    Accepted,
    Retry,
    Rejected,
    Unavailable,
}
/// An adapter may acknowledge acceptance, never exactly-once delivery or human visibility.
pub trait DeliveryAdapter: Send + Sync {
    fn deliver<'a>(
        &'a self,
        payload: &'a str,
        event_id: &'a str,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = (DeliveryOutcome, i64)> + Send + 'a>>;
}
fn clip(value: &str, limit: usize, truncated: &mut bool) -> String {
    let mut end = value.len().min(limit);
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    *truncated |= end < value.len();
    value[..end].to_string()
}
/// INVARIANT: Only selected fields leave SQLCipher. Never export raw badges (OTP/Pix), HTML or headers.
pub fn payload(event: &str, now: i64, fields: &[ExportField], facts: &MessageFacts) -> String {
    let mut result = json!({"schema_version":1,"event_id":event,"timestamp_epoch_ms":now});
    let mut truncated = false;
    for field in fields {
        let (key, value) = match field {
            ExportField::Account => (
                "account_id",
                json!(clip(&facts.account_id, 128, &mut truncated)),
            ),
            ExportField::Sender => (
                "sender",
                json!(facts
                    .sender_address
                    .as_ref()
                    .map(|s| clip(s, 512, &mut truncated))),
            ),
            ExportField::Subject => ("subject", json!(clip(&facts.subject, 1024, &mut truncated))),
            ExportField::Excerpt => (
                "excerpt",
                json!(facts.body.as_ref().map(|s| clip(s, 512, &mut truncated))),
            ),
            ExportField::Body => (
                "body",
                json!(facts.body.as_ref().map(|s| clip(s, 6000, &mut truncated))),
            ),
            ExportField::ExtractedFacts => {
                truncated |= facts.amounts_cents.len() > 16 || facts.due_dates.len() > 16;
                (
                    "extracted_facts",
                    json!({"amounts_cents":facts.amounts_cents.iter().take(16).collect::<Vec<_>>(),"due_dates":facts.due_dates.iter().take(16).map(|d|clip(d,10,&mut truncated)).collect::<Vec<_>>()}),
                )
            }
            ExportField::Attachments => {
                truncated |= facts.attachments.len() > 16;
                ("attachments", Value::Array(facts.attachments.iter().take(16).map(|(n,m)|json!({"name":clip(n,128,&mut truncated),"mime_type":clip(m,128,&mut truncated)})).collect()))
            }
        };
        result[key] = value;
    }
    result["truncated"] = json!(truncated);
    // JSON control-character escaping can expand bytes sixfold; limits above include that cost.
    let serialized = result.to_string();
    if serialized.len() <= MAX_PAYLOAD_BYTES {
        serialized
    } else {
        json!({"schema_version":1,"event_id":event,"timestamp_epoch_ms":now,"truncated":true})
            .to_string()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exports_only_selected_fields_and_bounds_escaped_unicode() {
        let f = MessageFacts {
            subject: "private".into(),
            body: Some("\u{0}😀".repeat(100_000)),
            ..Default::default()
        };
        let empty = payload("e", 0, &[], &f);
        assert!(!empty.contains("private"));
        assert!(!empty.contains("body"));
        let body = payload("e", 0, &[ExportField::Body], &f);
        assert!(body.len() <= MAX_PAYLOAD_BYTES);
        assert!(body.contains("\"truncated\":true"));
        assert!(!body.contains("subject"));
    }
}
