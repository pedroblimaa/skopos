#[cfg(feature = "e2e")]
use super::MonitoringState;
use super::Record;
use chrono::{DateTime, Local, Timelike};
use tauri::AppHandle;
#[cfg(feature = "e2e")]
use tauri::Manager;

impl Record {
    pub(crate) fn is_due(&self, day: &str, hour: u32) -> bool {
        if !self.enabled {
            return false;
        }

        self.days
            .get(day)
            .is_none_or(|slots| !slots.launch || (hour >= 18 && !slots.evening))
    }

    pub(crate) fn claim(&mut self, day: String, hour: u32, now: i64) {
        let slots = self.days.entry(day).or_default();
        slots.launch = true;

        if hour >= 18 {
            slots.evening = true;
        }

        self.last_attempt = Some(now);
        self.failure = None;
    }
}

pub(super) fn next_due(record: &Record, now: DateTime<Local>) -> Option<i64> {
    if !record.enabled {
        return None;
    }

    let day = now.format("%Y-%m-%d").to_string();

    if record.is_due(&day, now.hour()) {
        return Some(now.timestamp());
    }

    let slots = record.days.get(&day);
    let target = if slots.is_some_and(|slots| !slots.evening) {
        now.date_naive().and_hms_opt(18, 0, 0)
    } else {
        now.date_naive()
            .succ_opt()
            .and_then(|date| date.and_hms_opt(0, 0, 0))
    };

    target
        .and_then(|date| date.and_local_timezone(Local).earliest())
        .map(|date| date.timestamp())
}

pub(crate) fn now(app: &AppHandle) -> DateTime<Local> {
    #[cfg(feature = "e2e")]
    if let Some(timestamp) = *app.state::<MonitoringState>().clock.lock().unwrap() {
        if let Some(date) = DateTime::from_timestamp(timestamp, 0) {
            return date.with_timezone(&Local);
        }
    }

    let _ = app;
    Local::now()
}
