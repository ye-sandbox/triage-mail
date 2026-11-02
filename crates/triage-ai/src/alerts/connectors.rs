//! Pure provider formatting and bounded response classification. No credentials or I/O.
use super::{DeliveryOutcome, DestinationKind, MAX_PAYLOAD_BYTES};
use serde::Deserialize;
use serde_json::{json, Value};

pub const MAX_RESPONSE_BYTES: usize = 65_536;

/// INVARIANT: One event produces one bounded message; truncation never creates extra deliveries.
pub fn message(kind: DestinationKind, payload: &str, chat: Option<&str>) -> Option<String> {
    if payload.len() > MAX_PAYLOAD_BYTES {
        return None;
    }
    let value: Value = serde_json::from_str(payload).ok()?;
    if !value.is_object() {
        return None;
    }
    let limit = match kind {
        DestinationKind::Telegram => 3500,
        DestinationKind::Discord => 1800,
        _ => return None,
    };
    let text = bounded_text(payload, limit);
    Some(
        match kind {
            DestinationKind::Telegram => json!({
                "chat_id": chat?, "text": text, "link_preview_options": {"is_disabled": true}
            }),
            // INVARIANT: Untrusted mail must never ping users/roles/everyone or generate link embeds.
            DestinationKind::Discord => json!({
                "content": text, "allowed_mentions": {"parse": []}, "flags": 4
            }),
            _ => return None,
        }
        .to_string(),
    )
}
fn bounded_text(text: &str, limit: usize) -> String {
    const SUFFIX: &str = "\n[truncated]";
    if text.encode_utf16().count() <= limit {
        return text.to_string();
    }
    let mut result = String::new();
    let mut units = 0;
    for c in text.chars() {
        units += c.len_utf16();
        if units > limit - SUFFIX.len() {
            break;
        }
        result.push(c);
    }
    result.push_str(SUFFIX);
    result
}
#[derive(Deserialize)]
struct Parameters {
    retry_after: Option<f64>,
}
#[derive(Deserialize)]
struct Envelope {
    ok: Option<bool>,
    error_code: Option<u16>,
    retry_after: Option<f64>,
    parameters: Option<Parameters>,
}
/// Only fixed outcomes and bounded delay escape this parser; descriptions/results are discarded.
pub fn response(kind: DestinationKind, status: u16, body: &[u8]) -> (DeliveryOutcome, i64) {
    if body.len() > MAX_RESPONSE_BYTES {
        return (DeliveryOutcome::Rejected, 0);
    }
    let envelope = serde_json::from_slice::<Envelope>(body).ok();
    let delay = envelope
        .as_ref()
        .and_then(|e| {
            if kind == DestinationKind::Telegram {
                e.parameters.as_ref().and_then(|p| p.retry_after)
            } else {
                e.retry_after
            }
        })
        .filter(|s| s.is_finite() && *s >= 0.0)
        .map(|s| (s.min(3600.0) * 1000.0).ceil() as i64)
        .unwrap_or(0);
    let code = if kind == DestinationKind::Telegram && (200..300).contains(&status) {
        match &envelope {
            Some(e) if e.ok == Some(true) => return (DeliveryOutcome::Accepted, 0),
            Some(e) if e.ok == Some(false) => e.error_code.unwrap_or(400),
            _ => return (DeliveryOutcome::Rejected, 0),
        }
    } else {
        status
    };
    let outcome = match code {
        200..=299 if kind == DestinationKind::Discord => DeliveryOutcome::Accepted,
        408 | 429 | 500..=599 => DeliveryOutcome::Retry,
        _ => DeliveryOutcome::Rejected,
    };
    (
        outcome,
        if outcome == DeliveryOutcome::Retry {
            delay
        } else {
            0
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unicode_bounds_and_provider_controls_preserve_selected_fields_only() {
        let payload = json!({"subject": "😀<&_*`@everyone https://example.com/".repeat(900), "truncated":false}).to_string();
        for (kind, limit) in [
            (DestinationKind::Telegram, 3500),
            (DestinationKind::Discord, 1800),
        ] {
            let v: Value =
                serde_json::from_str(&message(kind, &payload, Some("-123")).unwrap()).unwrap();
            let text = v[if kind == DestinationKind::Telegram {
                "text"
            } else {
                "content"
            }]
            .as_str()
            .unwrap();
            assert!(text.encode_utf16().count() <= limit);
            assert!(text.ends_with("[truncated]"));
            assert!(v.get("parse_mode").is_none());
            assert!(v.get("attachments").is_none());
            if kind == DestinationKind::Discord {
                assert_eq!(v["allowed_mentions"], json!({"parse":[]}));
                assert_eq!(v["flags"], 4);
            } else {
                assert_eq!(v["link_preview_options"]["is_disabled"], true);
            }
        }
        let plain = json!({"event_id":"synthetic","test":true}).to_string();
        let v: Value =
            serde_json::from_str(&message(DestinationKind::Discord, &plain, None).unwrap())
                .unwrap();
        assert_eq!(v["content"], plain);
        assert!(!v.to_string().contains("subject"));
        assert!(message(DestinationKind::Telegram, "[]", Some("1")).is_none());
    }
    #[test]
    fn provider_acceptance_errors_delays_and_malformed_responses() {
        use DestinationKind::{Discord, Telegram};
        assert_eq!(
            response(Telegram, 200, br#"{"ok":true,"result":{"text":"discard"}}"#),
            (DeliveryOutcome::Accepted, 0)
        );
        assert_eq!(
            response(
                Telegram,
                200,
                br#"{"ok":false,"error_code":429,"parameters":{"retry_after":12}}"#
            ),
            (DeliveryOutcome::Retry, 12000)
        );
        assert_eq!(
            response(Discord, 429, br#"{"retry_after":1.25,"message":"discard"}"#),
            (DeliveryOutcome::Retry, 1250)
        );
        for status in [408, 429, 500, 503] {
            assert_eq!(
                response(Telegram, status, b"invalid").0,
                DeliveryOutcome::Retry
            );
        }
        for status in [301, 400, 401, 403, 404] {
            assert_eq!(
                response(Discord, status, b"invalid").0,
                DeliveryOutcome::Rejected
            );
        }
        assert_eq!(response(Discord, 204, b"").0, DeliveryOutcome::Accepted);
        assert_eq!(response(Telegram, 200, b"{}").0, DeliveryOutcome::Rejected);
        assert_eq!(
            response(Telegram, 200, b"invalid").0,
            DeliveryOutcome::Rejected
        );
        assert_eq!(
            response(Discord, 429, br#"{"retry_after":999999999}"#).1,
            3_600_000
        );
        assert_eq!(response(Discord, 429, br#"{"retry_after":-1}"#).1, 0);
        assert_eq!(
            response(Telegram, 200, &vec![b' '; MAX_RESPONSE_BYTES + 1]).0,
            DeliveryOutcome::Rejected
        );
    }
}
