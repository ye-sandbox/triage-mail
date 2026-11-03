//! Civil scheduling is headless and clock-injected. No OS or network access.
use serde::{Deserialize, Serialize};

pub const MINUTE_MS: i64 = 60_000;
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScheduleZone {
    #[default]
    SystemLocal,
    Utc,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cadence {
    #[default]
    Immediate,
    Hourly {
        minute: u16,
    },
    Daily {
        minute: u16,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuietHours {
    pub start: u16,
    pub end: u16,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchedulePolicy {
    pub destination_id: String,
    pub destination_revision: u32,
    pub revision: u32,
    pub cadence: Cadence,
    pub zone: ScheduleZone,
    pub quiet: Option<QuietHours>,
    pub urgent_bypass: bool,
}
impl SchedulePolicy {
    pub fn immediate(id: &str, revision: u32) -> Self {
        Self {
            destination_id: id.into(),
            destination_revision: revision,
            revision: 0,
            cadence: Cadence::Immediate,
            zone: ScheduleZone::SystemLocal,
            quiet: None,
            urgent_bypass: false,
        }
    }
    pub fn validate(&self) -> bool {
        super::valid_id(&self.destination_id)
            && self.revision < i32::MAX as u32
            && match self.cadence {
                Cadence::Immediate => true,
                Cadence::Hourly { minute } => minute < 60,
                Cadence::Daily { minute } => minute < 1440,
            }
            && self
                .quiet
                .is_none_or(|q| q.start < 1440 && q.end < 1440 && q.start != q.end)
    }
}
/// Civil minute = days since Unix epoch * 1440 + minute of day.
/// INVARIANT: Resolution chooses the earlier overlap occurrence and advances gaps to
/// the first valid minute. Implementations must fail closed outside supported dates.
pub trait ScheduleCalendar {
    fn civil_minute(&self, utc_ms: i64, zone: ScheduleZone) -> Option<i64>;
    fn resolve(&self, civil_minute: i64, zone: ScheduleZone) -> Option<i64>;
}
pub struct UtcCalendar;
impl ScheduleCalendar for UtcCalendar {
    fn civil_minute(&self, utc_ms: i64, zone: ScheduleZone) -> Option<i64> {
        (zone == ScheduleZone::Utc).then(|| utc_ms.div_euclid(MINUTE_MS))
    }
    fn resolve(&self, civil: i64, zone: ScheduleZone) -> Option<i64> {
        if zone != ScheduleZone::Utc {
            return None;
        }
        civil.checked_mul(MINUTE_MS)
    }
}
/// The quiet interval is [start,end), with wrapping at civil midnight.
pub fn quiet_end(
    policy: &SchedulePolicy,
    now: i64,
    calendar: &dyn ScheduleCalendar,
) -> Option<i64> {
    let Some(q) = policy.quiet else {
        return Some(now);
    };
    let civil = calendar.civil_minute(now, policy.zone)?;
    let minute = civil.rem_euclid(1440);
    let (start, end) = (i64::from(q.start), i64::from(q.end));
    let inside = if start < end {
        minute >= start && minute < end
    } else {
        minute >= start || minute < end
    };
    if !inside {
        return Some(now);
    }
    let day = civil.div_euclid(1440) + i64::from(start > end && minute >= start);
    let resolved = calendar.resolve(day.checked_mul(1440)?.checked_add(end)?, policy.zone)?;
    if resolved > now {
        return Some(resolved);
    }
    // INVARIANT: In the second occurrence of a repeated hour the earlier quiet-end
    // boundary may be in the past. Stay quiet until wall time actually exits it.
    for step in 1..=2880 {
        let candidate = now
            .div_euclid(MINUTE_MS)
            .checked_add(step)?
            .checked_mul(MINUTE_MS)?;
        let minute = calendar
            .civil_minute(candidate, policy.zone)?
            .rem_euclid(1440);
        let inside = if start < end {
            minute >= start && minute < end
        } else {
            minute >= start || minute < end
        };
        if !inside {
            return Some(candidate);
        }
    }
    None
}
/// Returns an item's next slot and its stable civil identity. Due times can be
/// recomputed after timezone changes until the summary is sealed.
pub fn next_slot(
    policy: &SchedulePolicy,
    created: i64,
    calendar: &dyn ScheduleCalendar,
) -> Option<(i64, i64)> {
    let civil = calendar.civil_minute(created, policy.zone)?;
    if policy.cadence == Cadence::Immediate {
        let due = quiet_end(policy, created, calendar)?;
        return Some((due, calendar.civil_minute(due, policy.zone)?));
    }
    let (period, offset) = match policy.cadence {
        Cadence::Hourly { minute } => (60, i64::from(minute)),
        Cadence::Daily { minute } => (1440, i64::from(minute)),
        Cadence::Immediate => unreachable!(),
    };
    let first = civil
        .div_euclid(period)
        .checked_mul(period)?
        .checked_add(offset)?;
    for step in 0..4 {
        let slot = first.checked_add(step * period)?;
        let resolved = calendar.resolve(slot, policy.zone)?;
        if resolved > created {
            return Some((quiet_end(policy, resolved, calendar)?, slot));
        }
    }
    None
}
/// A bounded digest never enriches existing, consent-filtered snapshots.
pub fn digest(event: &str, now: i64, items: &[impl AsRef<str>]) -> String {
    let mut entries = Vec::new();
    for item in items.iter().take(50) {
        let Ok(value) = serde_json::from_str::<serde_json::Value>(item.as_ref()) else {
            continue;
        };
        entries.push(value);
        if serde_json::to_vec(&entries).map_or(true, |v| v.len() > super::MAX_PAYLOAD_BYTES - 1024)
        {
            entries.pop();
            break;
        }
    }
    serde_json::json!({"schema_version":1,"event_id":event,"timestamp_epoch_ms":now,
        "digest":true,"total":items.len(),"omitted":items.len()-entries.len(),
        "truncated":items.len()>entries.len(),"entries":entries})
    .to_string()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn overnight_boundaries_and_cadences() {
        let mut p = SchedulePolicy::immediate("d", 1);
        p.zone = ScheduleZone::Utc;
        p.quiet = Some(QuietHours {
            start: 22 * 60,
            end: 7 * 60,
        });
        assert_eq!(
            quiet_end(&p, 22 * 60 * MINUTE_MS, &UtcCalendar),
            Some((1440 + 420) * MINUTE_MS)
        );
        assert_eq!(
            quiet_end(&p, 420 * MINUTE_MS, &UtcCalendar),
            Some(420 * MINUTE_MS)
        );
        p.cadence = Cadence::Daily { minute: 23 * 60 };
        assert_eq!(
            next_slot(&p, 20 * 60 * MINUTE_MS, &UtcCalendar),
            Some(((1440 + 420) * MINUTE_MS, 1380))
        );
        p.quiet = Some(QuietHours { start: 1, end: 1 });
        assert!(!p.validate());
    }
    #[test]
    fn digest_bounds_and_no_new_fields() {
        let input = vec![serde_json::json!({"subject":"x".repeat(6000)}).to_string(); 1000];
        let result = digest("stable", 0, &input);
        assert!(result.len() <= super::super::MAX_PAYLOAD_BYTES);
        let value: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(value["total"], 1000);
        assert_eq!(value["truncated"], true);
        assert!(value["entries"].as_array().unwrap().len() <= 50);
        assert!(!result.contains("body"));
    }
}

#[cfg(test)]
mod calendar_tests {
    use super::*;
    // Synthetic transitions occur at UTC minute 120. The inverse resolves a gap
    // to its end, and an overlap to the first occurrence, as required by the trait.
    struct Transition {
        delta: i64,
    }
    impl ScheduleCalendar for Transition {
        fn civil_minute(&self, ms: i64, _: ScheduleZone) -> Option<i64> {
            let utc = ms.div_euclid(MINUTE_MS);
            Some(utc + if utc >= 120 { self.delta } else { 0 })
        }
        fn resolve(&self, civil: i64, _: ScheduleZone) -> Option<i64> {
            let utc = if self.delta > 0 {
                if civil < 120 {
                    civil
                } else {
                    (civil - self.delta).max(120)
                }
            } else if civil < 120 {
                civil
            } else {
                civil - self.delta
            };
            Some(utc * MINUTE_MS)
        }
    }
    #[test]
    fn gaps_overlaps_and_repeated_quiet_end_fail_closed() {
        let mut p = SchedulePolicy::immediate("d", 1);
        p.cadence = Cadence::Daily { minute: 150 };
        assert_eq!(
            next_slot(&p, 100 * MINUTE_MS, &Transition { delta: 60 }),
            Some((120 * MINUTE_MS, 150))
        );
        p.cadence = Cadence::Daily { minute: 90 };
        assert_eq!(
            next_slot(&p, 50 * MINUTE_MS, &Transition { delta: -60 }),
            Some((90 * MINUTE_MS, 90))
        );
        // Second 01:10 cannot schedule another 01:30 delivery on the same day.
        assert_eq!(
            next_slot(&p, 130 * MINUTE_MS, &Transition { delta: -60 }),
            Some((1590 * MINUTE_MS, 1530))
        );
        p.quiet = Some(QuietHours { start: 0, end: 90 });
        assert_eq!(
            quiet_end(&p, 130 * MINUTE_MS, &Transition { delta: -60 }),
            Some(150 * MINUTE_MS)
        );
    }
    struct Offset(i64);
    impl ScheduleCalendar for Offset {
        fn civil_minute(&self, ms: i64, _: ScheduleZone) -> Option<i64> {
            Some(ms.div_euclid(MINUTE_MS) + self.0)
        }
        fn resolve(&self, civil: i64, _: ScheduleZone) -> Option<i64> {
            Some((civil - self.0) * MINUTE_MS)
        }
    }
    #[test]
    fn non_whole_hour_offsets_and_pre_epoch_dates() {
        let mut p = SchedulePolicy::immediate("d", 1);
        p.cadence = Cadence::Daily { minute: 540 };
        assert_eq!(next_slot(&p, 0, &Offset(345)), Some((195 * MINUTE_MS, 540)));
        assert_eq!(
            next_slot(&p, -1440 * MINUTE_MS, &Offset(-180)),
            Some((-720 * MINUTE_MS, -900))
        );
    }
}
