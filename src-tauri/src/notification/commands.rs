use super::{delivery, repository::Repository, DeliveryStatus, NotificationState, Settings};
use crate::{app_message::AppMessage, telegram::AuthState};
use std::path::PathBuf;
#[cfg(not(feature = "e2e"))]
use tauri::Manager;
use tauri::{AppHandle, State};

#[tauri::command]
pub(crate) async fn notification_settings(
    app: AppHandle,
    auth: State<'_, AuthState>,
    state: State<'_, NotificationState>,
) -> Result<Settings, AppMessage> {
    let account = auth.account_id(&app).await?;
    let _operation = state.operation.lock().await;
    Ok(repository(&app)?.load(account).await?.settings)
}

#[tauri::command]
pub(crate) async fn save_notification_settings(
    app: AppHandle,
    auth: State<'_, AuthState>,
    state: State<'_, NotificationState>,
    settings: Settings,
) -> Result<Settings, AppMessage> {
    let account = auth.account_id(&app).await?;
    let _operation = state.cancel_and_wait().await;
    let repository = repository(&app)?;
    let mut record = repository.load(account).await?;
    record.settings.telegram_enabled = settings.telegram_enabled;
    record.settings.desktop_enabled = settings.desktop_enabled;
    record.settings.language = if settings.language == "en" {
        "en"
    } else {
        "pt-BR"
    }
    .into();
    repository.save(account, &record).await?;

    Ok(record.settings)
}

#[tauri::command]
pub(crate) async fn notification_status(
    app: AppHandle,
    auth: State<'_, AuthState>,
    state: State<'_, NotificationState>,
) -> Result<DeliveryStatus, AppMessage> {
    let account = auth.account_id(&app).await?;
    let _operation = state.operation.lock().await;
    Ok(repository(&app)?.load(account).await?.status(None))
}

#[tauri::command]
pub(crate) async fn retry_uncertain_notifications(
    app: AppHandle,
    auth: State<'_, AuthState>,
    state: State<'_, NotificationState>,
    notification_day: String,
) -> Result<(), AppMessage> {
    let account = auth.account_id(&app).await?;
    let operation = state.cancel_and_wait().await;
    let repository = repository(&app)?;
    let mut record = repository.load(account).await?;
    record.delivery_day = Some(notification_day);
    if let Some(separator) = &mut record.separator {
        if separator.telegram == "uncertain" {
            separator.telegram = "pending".into();
        }
    }
    for item in &mut record.items {
        if item.telegram == "uncertain" {
            item.telegram = "pending".into();
        }
    }
    repository.save(account, &record).await?;
    drop(operation);
    delivery::start(app, account);
    Ok(())
}

pub(super) fn repository(app: &AppHandle) -> Result<Repository, AppMessage> {
    #[cfg(feature = "e2e")]
    let directory: PathBuf =
        std::env::temp_dir().join(format!("skopos-e2e-{}", std::process::id()));
    #[cfg(not(feature = "e2e"))]
    let directory: PathBuf = app
        .path()
        .app_local_data_dir()
        .map_err(|_| AppMessage::NotificationStorage)?;
    #[cfg(feature = "e2e")]
    let _ = app;
    std::fs::create_dir_all(&directory).map_err(|_| AppMessage::NotificationStorage)?;

    Ok(Repository(directory.join("notifications.sqlite")))
}
