use super::{repository, MonitoringState, Record, Status};
use crate::{app_message::AppMessage, telegram::AuthState};
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Manager};

#[tauri::command]
pub(crate) async fn configure_monitoring_fixture(
    app: AppHandle,
    now: i64,
    reset: bool,
) -> Result<(), AppMessage> {
    let search = app.state::<crate::telegram::search::SearchState>();
    let _search = search.cancel_and_wait().await;
    let state = app.state::<MonitoringState>();
    let _operation = state.operation.lock().await;
    let notifications = app.state::<crate::notification::NotificationState>();
    let _notifications = notifications.cancel_and_wait().await;
    let account = app.state::<AuthState>().account_id(&app).await?;

    if reset {
        repository(&app)?.save(account, &Record::default()).await?;
    }
    *state.clock.lock().unwrap() = Some(now);
    state.is_suspended.store(false, Ordering::SeqCst);
    *state.failure.lock().await = None;

    Ok(())
}

#[tauri::command]
pub(crate) fn wake_monitoring_fixture(app: AppHandle) {
    app.state::<MonitoringState>().wake.notify_one();
}

#[tauri::command]
pub(crate) async fn tick_monitoring_fixture(app: AppHandle) -> Result<Status, AppMessage> {
    crate::telegram::search::execution::automatic(&app).await?;
    let account = app.state::<AuthState>().account_id(&app).await?;
    super::status(&app, account).await
}
