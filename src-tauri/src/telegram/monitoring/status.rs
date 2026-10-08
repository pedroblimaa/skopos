use super::{repository, schedule, MonitoringState, Status};
use crate::app_message::AppMessage;
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager};

pub(crate) async fn status(app: &AppHandle, account: i64) -> Result<Status, AppMessage> {
    let record = repository(app)?.load(account).await?;
    let state = app.state::<MonitoringState>();
    let next_due = schedule::next_due(&record, schedule::now(app));
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
