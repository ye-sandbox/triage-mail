//! RFC 2087 IMAP `QUOTA` parsing and deterministic subscription-tier inference.
//!
//! INVARIANT: Plan badges are inferred exclusively from the server-reported storage limit.
//! Message bodies are never inspected, so marketing or forwarded mail cannot cause false positives.

use serde::{Deserialize, Serialize};

use crate::provider::ProviderKind;

const KIB: u64 = 1024;
const GIB: u64 = 1024 * 1024 * 1024;
const TIB: u64 = 1024 * GIB;

/// Storage usage reported by `QUOTA` in bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountStorageQuota {
    pub used_bytes: u64,
    pub limit_bytes: u64,
}

/// Closed set of subscription plan badges.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AccountPlanBadge {
    Ultra,
    Pro,
    One,
    Workspace,
    Plus,
}

/// Manual per-account badge override; always serialized as `{ mode, value? }`.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "mode", content = "value")]
pub enum AccountBadgeOverride {
    #[default]
    Auto,
    Hidden,
    Preset(AccountPlanBadge),
    Custom(String),
}

/// Maximum custom badge label length in characters.
pub const MAX_CUSTOM_BADGE_CHARS: usize = 12;

impl AccountBadgeOverride {
    /// Trims and validates a custom label; returns `None` when empty, too long or non-printable.
    #[must_use]
    pub fn normalized(self) -> Option<Self> {
        match self {
            Self::Custom(label) => {
                let label = label.trim().to_string();
                let valid = !label.is_empty()
                    && label.chars().count() <= MAX_CUSTOM_BADGE_CHARS
                    && label.chars().all(|c| !c.is_control());
                valid.then_some(Self::Custom(label))
            }
            other => Some(other),
        }
    }
}

/// Parses one untagged `* QUOTA <root> (STORAGE <used> <limit> ...)` line.
///
/// RFC 2087 STORAGE values are 1024-octet blocks; conversion uses checked arithmetic.
#[must_use]
pub fn parse_quota_line(line: &str) -> Option<AccountStorageQuota> {
    let rest = line.trim().strip_prefix("* QUOTA ")?;
    let open = rest.find('(')?;
    let close = rest.rfind(')')?;
    let mut tokens = rest.get(open + 1..close)?.split_whitespace();
    while let Some(resource) = tokens.next() {
        let used = tokens.next()?;
        let limit = tokens.next()?;
        if resource.eq_ignore_ascii_case("STORAGE") {
            return Some(AccountStorageQuota {
                used_bytes: used.parse::<u64>().ok()?.checked_mul(KIB)?,
                limit_bytes: limit.parse::<u64>().ok()?.checked_mul(KIB)?,
            });
        }
    }
    None
}

/// Deterministically maps a quota limit to a plan badge for Google accounts only.
///
/// INVARIANT: Non-Google servers report arbitrary admin-set limits, so no badge is inferred.
#[must_use]
pub fn infer_subscription_badge(
    provider: ProviderKind,
    host: &str,
    quota: Option<AccountStorageQuota>,
) -> Option<AccountPlanBadge> {
    let host = host.trim().to_ascii_lowercase();
    let is_google = provider == ProviderKind::Gmail
        || host.ends_with("gmail.com")
        || host.ends_with("googlemail.com");
    if !is_google {
        return None;
    }
    let limit = quota?.limit_bytes;
    if limit >= 5 * TIB {
        Some(AccountPlanBadge::Ultra)
    } else if limit >= 500 * GIB {
        Some(AccountPlanBadge::Pro)
    } else if limit > 20 * GIB {
        Some(AccountPlanBadge::One)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn q(limit_gib: u64) -> Option<AccountStorageQuota> {
        Some(AccountStorageQuota {
            used_bytes: 1,
            limit_bytes: limit_gib * GIB,
        })
    }

    #[test]
    fn parses_storage_quota_and_converts_kib() {
        let parsed = parse_quota_line("* QUOTA \"\" (STORAGE 10 512)").unwrap();
        assert_eq!(parsed.used_bytes, 10 * 1024);
        assert_eq!(parsed.limit_bytes, 512 * 1024);
        let multi = parse_quota_line("* QUOTA \"root\" (MESSAGE 5 100 STORAGE 7 9)").unwrap();
        assert_eq!(multi.limit_bytes, 9 * 1024);
    }

    #[test]
    fn rejects_malformed_or_overflowing_quota() {
        assert!(parse_quota_line("* QUOTA \"\" (MESSAGE 1 2)").is_none());
        assert!(parse_quota_line("* QUOTA \"\" (STORAGE x 2)").is_none());
        assert!(parse_quota_line("* QUOTAROOT INBOX \"\"").is_none());
        let overflow = format!("* QUOTA \"\" (STORAGE 1 {})", u64::MAX);
        assert!(parse_quota_line(&overflow).is_none());
    }

    #[test]
    fn infers_google_tiers_only() {
        let g = ProviderKind::Gmail;
        assert_eq!(infer_subscription_badge(g, "imap.gmail.com", q(15)), None);
        assert_eq!(
            infer_subscription_badge(g, "imap.gmail.com", q(100)),
            Some(AccountPlanBadge::One)
        );
        assert_eq!(
            infer_subscription_badge(g, "imap.gmail.com", q(2048)),
            Some(AccountPlanBadge::Pro)
        );
        assert_eq!(
            infer_subscription_badge(g, "imap.gmail.com", q(5120)),
            Some(AccountPlanBadge::Ultra)
        );
        assert_eq!(infer_subscription_badge(g, "imap.gmail.com", None), None);
        let generic = ProviderKind::GenericRfc;
        assert_eq!(
            infer_subscription_badge(generic, "imap.example.com", q(2048)),
            None
        );
    }

    #[test]
    fn override_serializes_tagged_and_validates_custom() {
        let json = serde_json::to_string(&AccountBadgeOverride::Auto).unwrap();
        assert_eq!(json, r#"{"mode":"Auto"}"#);
        let json =
            serde_json::to_string(&AccountBadgeOverride::Preset(AccountPlanBadge::Pro)).unwrap();
        assert_eq!(json, r#"{"mode":"Preset","value":"Pro"}"#);
        assert!(AccountBadgeOverride::Custom("  ".into())
            .normalized()
            .is_none());
        assert!(AccountBadgeOverride::Custom("x".repeat(13))
            .normalized()
            .is_none());
        assert_eq!(
            AccountBadgeOverride::Custom(" Work ".into()).normalized(),
            Some(AccountBadgeOverride::Custom("Work".into()))
        );
    }
}
