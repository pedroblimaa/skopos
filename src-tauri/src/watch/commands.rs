use super::repository::{CreateWatch, Watch, WatchRepository};
use crate::app_message::AppMessage;
use tauri::AppHandle;
use tauri::Manager;

#[tauri::command]
pub async fn create_watch(app: AppHandle, input: CreateWatch) -> Result<Watch, AppMessage> {
    let repository = repository(&app)?;
    let watch = repository
        .create(input)
        .await
        .map_err(|error| error.message())?;
    app.state::<crate::telegram::monitoring::MonitoringState>()
        .wake
        .notify_one();

    Ok(watch)
}

#[tauri::command]
pub async fn list_watches(app: AppHandle) -> Result<Vec<Watch>, AppMessage> {
    let repository = repository(&app)?;
    repository.list().await.map_err(|error| error.message())
}

#[tauri::command]
pub async fn update_watch(
    app: AppHandle,
    id: i64,
    input: CreateWatch,
) -> Result<Watch, AppMessage> {
    let watch = repository(&app)?
        .update(id, input)
        .await
        .map_err(|error| error.message())?;
    app.state::<crate::telegram::monitoring::MonitoringState>()
        .wake
        .notify_one();

    Ok(watch)
}

#[tauri::command]
pub async fn delete_watch(app: AppHandle, id: i64) -> Result<(), AppMessage> {
    repository(&app)?
        .delete(id)
        .await
        .map_err(|error| error.message())?;
    app.state::<crate::telegram::monitoring::MonitoringState>()
        .wake
        .notify_one();

    Ok(())
}

fn repository(app: &AppHandle) -> Result<WatchRepository, AppMessage> {
    #[cfg(feature = "e2e")]
    let directory = std::env::temp_dir().join(format!("skopos-e2e-{}", std::process::id()));

    #[cfg(not(feature = "e2e"))]
    let directory = app
        .path()
        .app_local_data_dir()
        .map_err(|_| AppMessage::WatchStorageOpen)?;

    #[cfg(feature = "e2e")]
    let _ = app;

    std::fs::create_dir_all(&directory).map_err(|_| AppMessage::WatchStorageOpen)?;

    Ok(WatchRepository::new(directory.join("watches.sqlite")))
}
