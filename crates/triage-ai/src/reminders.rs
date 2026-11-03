//! Pure due-date dashboard models, scheduled reminder triggers, and civil date arithmetic.
use crate::rules::valid_date;
use serde::{Deserialize, Serialize};

pub const MAX_REMINDER_CONFIGS: usize = 50;

/// Extracted due date item presented on the due-date dashboard.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DueDateItem {
    pub message_id: String,
    pub account_id: String,
    pub subject: String,
    pub sender: String,
    pub received_at_epoch_ms: i64,
    pub is_read: bool,
    pub digitable_line: String,
    pub amount_cents: Option<u64>,
    pub due_date_iso: String,
    pub is_dismissed: bool,
}

/// Categorized due-date status relative to a reference calendar date.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DueDateStatus {
    Overdue,
    DueToday,
    Upcoming,
    Dismissed,
}

impl DueDateItem {
    pub fn status(&self, today_iso: &str) -> DueDateStatus {
        if self.is_dismissed {
            DueDateStatus::Dismissed
        } else if self.due_date_iso.as_str() < today_iso {
            DueDateStatus::Overdue
        } else if self.due_date_iso.as_str() == today_iso {
            DueDateStatus::DueToday
        } else {
            DueDateStatus::Upcoming
        }
    }
}

/// Trigger definition for a scheduled reminder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReminderTrigger {
    /// Scheduled relative to the extracted due date.
    DueDate {
        days_before: u32,
        target_hour: u8,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        min_amount_cents: Option<u64>,
    },
    /// Scheduled if a matching message remains unread for a specified duration.
    UnreadInterval {
        interval_hours: u32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        min_amount_cents: Option<u64>,
    },
}

/// Persisted configuration for an automated scheduled reminder rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScheduledReminderConfig {
    pub version: u32,
    pub id: String,
    pub revision: u32,
    pub name: String,
    pub enabled: bool,
    pub trigger: ReminderTrigger,
    pub destination_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
}

fn bounded(s: &str, max: usize) -> bool {
    !s.trim().is_empty() && s.len() <= max && !s.chars().any(char::is_control)
}

impl ScheduledReminderConfig {
    pub fn validate(&self) -> bool {
        if self.version != 1
            || !bounded(&self.id, 100)
            || !bounded(&self.name, 120)
            || self.destination_ids.is_empty()
            || self.destination_ids.len() > 32
            || !self.destination_ids.iter().all(|d| bounded(d, 100))
            || self.account_id.as_ref().is_some_and(|a| !bounded(a, 100))
        {
            return false;
        }

        match &self.trigger {
            ReminderTrigger::DueDate {
                days_before,
                target_hour,
                ..
            } => *days_before <= 365 && *target_hour <= 23,
            ReminderTrigger::UnreadInterval { interval_hours, .. } => {
                *interval_hours >= 1 && *interval_hours <= 8760
            }
        }
    }
}

/// Days from civil epoch (0000-03-01) using Howard Hinnant's algorithm.
pub fn day_number(date: &str) -> Option<i64> {
    if !valid_date(date) {
        return None;
    }
    let mut parts = date.split('-').map(str::parse::<i64>);
    let (mut y, m, d) = (
        parts.next()?.ok()?,
        parts.next()?.ok()?,
        parts.next()?.ok()?,
    );
    y -= i64::from(m <= 2);
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = m + if m > 2 { -3 } else { 9 };
    Some(era * 146097 + yoe * 365 + yoe / 4 - yoe / 100 + (153 * mp + 2) / 5 + d - 1)
}

/// Converts a civil day number back to `YYYY-MM-DD`.
pub fn date_from_day_number(d_ord: i64) -> String {
    let era = d_ord.div_euclid(146097);
    let doe = d_ord - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let mut y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    y += i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Subtracts `days` from an ISO `YYYY-MM-DD` date.
pub fn sub_days_iso(date: &str, days: u32) -> Option<String> {
    let d = day_number(date)?;
    Some(date_from_day_number(d.checked_sub(i64::from(days))?))
}

/// Adds `days` to an ISO `YYYY-MM-DD` date.
pub fn add_days_iso(date: &str, days: u32) -> Option<String> {
    let d = day_number(date)?;
    Some(date_from_day_number(d.checked_add(i64::from(days))?))
}

const UNIX_EPOCH_DAY_ORDINAL: i64 = 719468;

/// Calculates the exact target UTC epoch milliseconds for a due-date reminder.
///
/// Parameters:
/// - `due_date_iso`: "YYYY-MM-DD"
/// - `days_before`: number of days prior to the due date
/// - `target_hour`: hour of day (0..=23)
/// - `tz_offset_seconds`: offset from UTC in seconds (e.g. -10800 for UTC-3 Brasília)
pub fn due_date_trigger_epoch_ms(
    due_date_iso: &str,
    days_before: u32,
    target_hour: u8,
    tz_offset_seconds: i32,
) -> Option<i64> {
    if target_hour > 23 {
        return None;
    }
    let target_date = sub_days_iso(due_date_iso, days_before)?;
    let d_ord = day_number(&target_date)?;
    let days_since_unix = d_ord.checked_sub(UNIX_EPOCH_DAY_ORDINAL)?;
    let midnight_utc_secs = days_since_unix.checked_mul(86400)?;
    let target_hour_secs = i64::from(target_hour) * 3600;
    // INVARIANT: Local target hour minus tz_offset_seconds yields exact UTC seconds
    let utc_secs = midnight_utc_secs
        .checked_add(target_hour_secs)?
        .checked_sub(i64::from(tz_offset_seconds))?;
    utc_secs.checked_mul(1000)
}

/// Calculates the target UTC epoch milliseconds for an unread-interval reminder.
pub fn unread_interval_trigger_epoch_ms(
    received_at_epoch_ms: i64,
    interval_hours: u32,
) -> Option<i64> {
    let millis = i64::from(interval_hours).checked_mul(3600 * 1000)?;
    received_at_epoch_ms.checked_add(millis)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_day_number_roundtrip() {
        let dates = [
            "1970-01-01",
            "2000-02-29",
            "2024-02-29",
            "2025-01-01",
            "2026-11-02",
            "2030-12-31",
        ];
        for d in dates {
            let num = day_number(d).expect("valid day number");
            let roundtrip = date_from_day_number(num);
            assert_eq!(d, roundtrip, "roundtrip match for {d}");
        }
    }

    #[test]
    fn test_civil_date_arithmetic() {
        // Leap year rollover
        assert_eq!(sub_days_iso("2024-03-01", 1), Some("2024-02-29".into()));
        assert_eq!(sub_days_iso("2025-03-01", 1), Some("2025-02-28".into()));
        assert_eq!(add_days_iso("2024-02-28", 1), Some("2024-02-29".into()));
        assert_eq!(add_days_iso("2024-02-28", 2), Some("2024-03-01".into()));

        // Year boundary
        assert_eq!(sub_days_iso("2025-01-01", 1), Some("2024-12-31".into()));
        assert_eq!(add_days_iso("2024-12-31", 1), Some("2025-01-01".into()));
    }

    #[test]
    fn test_due_date_trigger_epoch_ms_utc() {
        // 1970-01-02 at 09:00 UTC, 1 day before -> target date 1970-01-01 09:00 UTC
        let ms = due_date_trigger_epoch_ms("1970-01-02", 1, 9, 0).expect("valid epoch");
        assert_eq!(ms, 9 * 3600 * 1000);
    }

    #[test]
    fn test_due_date_trigger_epoch_ms_timezone_offset() {
        // Brazil UTC-3 (-10800 seconds)
        // Target: 2026-11-05, 0 days before, 09:00 local (UTC-3)
        // 09:00 UTC-3 is 12:00 UTC
        let ms = due_date_trigger_epoch_ms("2026-11-05", 0, 9, -10800).expect("valid epoch");
        let utc_noon = due_date_trigger_epoch_ms("2026-11-05", 0, 12, 0).expect("valid epoch");
        assert_eq!(ms, utc_noon);
    }

    #[test]
    fn test_unread_interval_trigger() {
        let now = 1_700_000_000_000i64;
        let due = unread_interval_trigger_epoch_ms(now, 24).expect("valid");
        assert_eq!(due, now + 24 * 3600 * 1000);
    }

    #[test]
    fn test_due_date_status() {
        let item = DueDateItem {
            message_id: "m1".into(),
            account_id: "a1".into(),
            subject: "Boleto".into(),
            sender: "billing@example.org".into(),
            received_at_epoch_ms: 1000,
            is_read: false,
            digitable_line: "34191.79001...".into(),
            amount_cents: Some(15000),
            due_date_iso: "2026-11-05".into(),
            is_dismissed: false,
        };

        assert_eq!(item.status("2026-11-04"), DueDateStatus::Upcoming);
        assert_eq!(item.status("2026-11-05"), DueDateStatus::DueToday);
        assert_eq!(item.status("2026-11-06"), DueDateStatus::Overdue);

        let mut dismissed = item.clone();
        dismissed.is_dismissed = true;
        assert_eq!(dismissed.status("2026-11-06"), DueDateStatus::Dismissed);
    }

    #[test]
    fn test_reminder_config_validation() {
        let valid = ScheduledReminderConfig {
            version: 1,
            id: "rem-1".into(),
            revision: 0,
            name: "Due date 1d before".into(),
            enabled: true,
            trigger: ReminderTrigger::DueDate {
                days_before: 1,
                target_hour: 9,
                min_amount_cents: Some(1000),
            },
            destination_ids: vec!["dest-local".into()],
            account_id: None,
        };
        assert!(valid.validate());

        let mut invalid = valid.clone();
        invalid.destination_ids.clear();
        assert!(!invalid.validate());

        let mut invalid_hour = valid.clone();
        invalid_hour.trigger = ReminderTrigger::DueDate {
            days_before: 1,
            target_hour: 24,
            min_amount_cents: None,
        };
        assert!(!invalid_hour.validate());

        let mut invalid_interval = valid.clone();
        invalid_interval.trigger = ReminderTrigger::UnreadInterval {
            interval_hours: 0,
            min_amount_cents: None,
        };
        assert!(!invalid_interval.validate());
    }
}
