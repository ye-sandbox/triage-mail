//! Native timezone adapter; scheduling and persistence stay headless.
use crate::{DesktopService, IpcError};
use chrono::{Local, LocalResult, NaiveDateTime, TimeZone, Utc};
use serde::Serialize;
use std::sync::Arc;
use triage_ai::alerts::schedule::{next_slot, ScheduleCalendar, SchedulePolicy, ScheduleZone};

pub struct NativeCalendar;
impl ScheduleCalendar for NativeCalendar {
    fn civil_minute(&self, ms: i64, zone: ScheduleZone) -> Option<i64> {
        let utc = chrono::DateTime::<Utc>::from_timestamp_millis(ms)?;
        Some(match zone {
            ScheduleZone::Utc => utc.timestamp().div_euclid(60),
            ScheduleZone::SystemLocal => utc
                .with_timezone(&Local)
                .naive_local()
                .and_utc()
                .timestamp()
                .div_euclid(60),
        })
    }
    fn resolve(&self, civil: i64, zone: ScheduleZone) -> Option<i64> {
        let naive: NaiveDateTime =
            chrono::DateTime::<Utc>::from_timestamp(civil.checked_mul(60)?, 0)?.naive_utc();
        if zone == ScheduleZone::Utc {
            return Some(naive.and_utc().timestamp_millis());
        }
        // INVARIANT: Gaps advance to the first valid civil minute, overlaps choose earlier.
        // A full skipped day is bounded as well (historical timezone realignments).
        for minute in 0..=1440 {
            let value = naive.checked_add_signed(chrono::Duration::minutes(minute))?;
            match Local.from_local_datetime(&value) {
                LocalResult::Single(t) => return Some(t.timestamp_millis()),
                LocalResult::Ambiguous(a, b) => return Some(a.min(b).timestamp_millis()),
                LocalResult::None => (),
            }
        }
        None
    }
}
#[derive(Serialize)]
pub struct SchedulePreview {
    next_epoch_ms: i64,
    timezone: String,
}
impl DesktopService {
    pub async fn alert_schedule(
        self: &Arc<Self>,
        id: String,
    ) -> Result<storage::alerts::schedule::ScheduleState, IpcError> {
        self.blocking(move |s| {
            s.store
                .alert_schedule(&id)
                .map_err(|_| IpcError::InvalidRequest)
        })
        .await
    }
    pub async fn save_alert_schedule(
        self: &Arc<Self>,
        policy: SchedulePolicy,
        consent: bool,
    ) -> Result<SchedulePolicy, IpcError> {
        let _gate = self.alert_gate.lock().await;
        self.blocking(move |s| {
            s.store
                .save_alert_schedule(policy, consent, storage::now_epoch_ms())
                .map_err(|_| IpcError::InvalidRequest)
        })
        .await
    }
    pub fn preview_alert_schedule(policy: SchedulePolicy) -> Result<SchedulePreview, IpcError> {
        if !policy.validate() {
            return Err(IpcError::InvalidRequest);
        }
        let (next_epoch_ms, _) = next_slot(&policy, storage::now_epoch_ms(), &NativeCalendar)
            .ok_or(IpcError::InvalidRequest)?;
        let timezone = match policy.zone {
            ScheduleZone::Utc => "UTC".into(),
            ScheduleZone::SystemLocal => {
                format!("System Local ({})", Local::now().format("%Z %:z"))
            }
        };
        Ok(SchedulePreview {
            next_epoch_ms,
            timezone,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_timezone_child() {
        let Ok(zone) = std::env::var("TRIAGE_SCHEDULE_ZONE_TEST") else {
            return;
        };
        let calendar = NativeCalendar;
        let civil = |y, m, d, h, min| {
            Utc.with_ymd_and_hms(y, m, d, h, min, 0)
                .unwrap()
                .timestamp()
                .div_euclid(60)
        };
        if zone == "America/New_York" {
            assert_eq!(
                calendar.resolve(civil(2026, 3, 8, 2, 30), ScheduleZone::SystemLocal),
                Some(
                    Utc.with_ymd_and_hms(2026, 3, 8, 7, 0, 0)
                        .unwrap()
                        .timestamp_millis()
                )
            );
            assert_eq!(
                calendar.resolve(civil(2026, 11, 1, 1, 30), ScheduleZone::SystemLocal),
                Some(
                    Utc.with_ymd_and_hms(2026, 11, 1, 5, 30, 0)
                        .unwrap()
                        .timestamp_millis()
                )
            );
        } else {
            let utc = Utc
                .with_ymd_and_hms(2026, 1, 1, 0, 0, 0)
                .unwrap()
                .timestamp_millis();
            let local = utc.div_euclid(60_000) + 345;
            assert_eq!(
                calendar.civil_minute(utc, ScheduleZone::SystemLocal),
                Some(local)
            );
            assert_eq!(
                calendar.resolve(local, ScheduleZone::SystemLocal),
                Some(utc)
            );
        }
        assert_eq!(calendar.resolve(0, ScheduleZone::Utc), Some(0));
    }
    #[test]
    fn native_calendar_uses_os_dst_and_non_hour_offsets() {
        // INVARIANT: Isolated child environments avoid mutating process-global TZ
        // or racing chrono's timezone cache in the parallel test suite.
        for zone in ["America/New_York", "Asia/Kathmandu"] {
            let status = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "alert_schedule::tests::native_timezone_child"])
                .env("TZ", zone)
                .env("TRIAGE_SCHEDULE_ZONE_TEST", zone)
                .status()
                .unwrap();
            assert!(status.success());
        }
    }
}
