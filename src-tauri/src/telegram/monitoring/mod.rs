pub(crate) mod commands;
#[cfg(feature = "e2e")]
pub(crate) mod fixture;
mod repository;
#[cfg(test)]
mod tests;

use crate::{app_message::AppMessage, promotion::ProductMatch};
use chrono::{Local, Timelike};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
};
use tauri::{AppHandle, Emitter, Listener, Manager};
use tokio::sync::{Mutex, Notify};

#[derive(Clone, Deserialize, Serialize)]
pub(crate) struct Checkpoint {
    pub message_id: i32,
    pub checked_at: i64,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct Record {
    pub enabled: bool,
    pub days: HashMap<String, Slots>,
    pub checkpoints: HashMap<String, Checkpoint>,
    pub pending: Vec<ProductMatch>,
    pub pending_day: String,
    pub last_attempt: Option<i64>,
    pub failure: Option<AppMessage>,
}

impl Default for Record {
    fn default() -> Self {
        Self {
            enabled: true,
            days: HashMap::new(),
            checkpoints: HashMap::new(),
            pending: Vec::new(),
            pending_day: String::new(),
            last_attempt: None,
            failure: None,
        }
    }
}

#[derive(Clone, Default, Deserialize, Serialize)]
pub(crate) struct Slots {
    launch: bool,
    evening: bool,
}

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

#[derive(Default)]
pub(crate) struct MonitoringState {
    #[cfg(feature = "e2e")]
    pub(crate) clock: std::sync::Mutex<Option<i64>>,
    pub(crate) operation: Mutex<()>,
    pub(crate) wake: Notify,
    pub(crate) is_running: AtomicBool,
    pub(crate) is_suspended: AtomicBool,
    pub(crate) is_stopping: AtomicBool,
    pub(crate) failure: Mutex<Option<AppMessage>>,
}

impl MonitoringState {
    pub(crate) fn can_run(&self) -> bool {
        !self.is_suspended.load(Ordering::SeqCst) && !self.is_stopping.load(Ordering::SeqCst)
    }
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Status {
    pub account_id: i64,
    pub enabled: bool,
    pub is_running: bool,
    pub last_attempt: Option<i64>,
    pub next_due: Option<i64>,
    pub failure: Option<AppMessage>,
}

pub(crate) fn repository(app: &AppHandle) -> Result<repository::Repository, AppMessage> {
    Ok(repository::Repository(
        directory(app)?.join("results.sqlite"),
    ))
}

pub(crate) fn directory(app: &AppHandle) -> Result<PathBuf, AppMessage> {
    #[cfg(feature = "e2e")]
    let directory = std::env::temp_dir().join(format!("skopos-e2e-{}", std::process::id()));
    #[cfg(feature = "e2e")]
    let _ = app;
    #[cfg(not(feature = "e2e"))]
    let directory = app
        .path()
        .app_local_data_dir()
        .map_err(|_| AppMessage::MonitoringStorage)?;
    std::fs::create_dir_all(&directory).map_err(|_| AppMessage::MonitoringStorage)?;

    Ok(directory)
}

pub(crate) async fn status(app: &AppHandle, account: i64) -> Result<Status, AppMessage> {
    let record = repository(app)?.load(account).await?;
    let state = app.state::<MonitoringState>();
    let now = now(app);
    let day = now.format("%Y-%m-%d").to_string();
    let next_due = if !record.enabled {
        None
    } else if record.is_due(&day, now.hour()) {
        Some(now.timestamp())
    } else {
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
    };
    let failure = state.failure.lock().await.clone().or(record.failure);

    Ok(Status {
        account_id: account,
        enabled: record.enabled,
        is_running: state.is_running.load(Ordering::SeqCst),
        last_attempt: record.last_attempt,
        next_due,
        failure,
    })
}

pub(crate) async fn publish(app: &AppHandle, account: i64) {
    if let Ok(status) = status(app, account).await {
        // The persisted state is authoritative when a window is hidden or misses an event.
        let _ = app.emit("monitoring:status", status);
    }
}

pub(crate) fn start(app: &AppHandle) {
    let handle = app.clone();
    app.listen("telegram:auth-changed", move |event| {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(event.payload()) {
            let state = handle.state::<MonitoringState>();
            state
                .is_suspended
                .store(value["authorized"] != true, Ordering::SeqCst);

            if let Ok(mut failure) = state.failure.try_lock() {
                *failure = None;
            }

            state.wake.notify_one();
        }
    });
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = handle.state::<MonitoringState>();

        loop {
            if state.is_stopping.load(Ordering::SeqCst) {
                break;
            }

            poll(&handle, &state).await;
            tokio::select! {
                _ = tokio::time::sleep(std::time::Duration::from_secs(60)) => {},
                _ = state.wake.notified() => {},
            }
        }
    });
}

async fn poll(app: &AppHandle, state: &MonitoringState) {
    #[cfg(feature = "e2e")]
    if state.clock.lock().unwrap().is_none() {
        return;
    }

    if state.is_suspended.load(Ordering::SeqCst) {
        return;
    }

    let result = super::search::execution::automatic(app).await;
    *state.failure.lock().await = result.err();

    if state.is_suspended.load(Ordering::SeqCst) || state.is_stopping.load(Ordering::SeqCst) {
        return;
    }

    let auth = app.state::<super::AuthState>();
    let Ok(context) = auth.client(app).await else {
        return;
    };
    let Ok(account) = context.local_account_id().await else {
        return;
    };
    publish(app, account).await;
}

pub(crate) fn now(app: &AppHandle) -> chrono::DateTime<Local> {
    #[cfg(feature = "e2e")]
    if let Some(timestamp) = *app.state::<MonitoringState>().clock.lock().unwrap() {
        if let Some(date) = chrono::DateTime::from_timestamp(timestamp, 0) {
            return date.with_timezone(&Local);
        }
    }

    let _ = app;
    Local::now()
}
