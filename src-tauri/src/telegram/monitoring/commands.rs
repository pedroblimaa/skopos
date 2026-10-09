use super::{repository, status, MonitoringState, Status};
use crate::{app_message::AppMessage, telegram::AuthState};
use tauri::{AppHandle, State};

#[tauri::command]
pub(crate) async fn monitoring_status(
    app: AppHandle,
    auth: State<'_, AuthState>,
) -> Result<Status, AppMessage> {
    let context = auth.client(&app).await.map_err(|error| error.message())?;
    let account = context
        .local_account_id()
        .await
        .map_err(|error| error.message())?;
    status(&app, account).await
}

#[tauri::command]
pub(crate) async fn save_monitoring_settings(
    app: AppHandle,
    auth: State<'_, AuthState>,
    state: State<'_, MonitoringState>,
    enabled: bool,
) -> Result<Status, AppMessage> {
    let account = auth.account_id(&app).await?;
    let operation = state.operation.lock().await;
    let repository = repository(&app)?;
    let mut record = repository.load(account).await?;
    record.enabled = enabled;
    repository.save(account, &record).await?;
    drop(operation);
    state.wake.notify_one();
    super::publish(&app, account).await;
    status(&app, account).await
}
