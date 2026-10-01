use super::repository::{CreateWatch, Watch, WatchRepository};
use tauri::AppHandle;
#[cfg(not(feature = "e2e"))]
use tauri::Manager;

#[tauri::command]
pub async fn create_watch(app: AppHandle, input: CreateWatch) -> Result<Watch, String> {
    let repository = repository(&app)?;
    repository
        .create(input)
        .await
        .map_err(|error| error.message())
}

#[tauri::command]
pub async fn list_watches(app: AppHandle) -> Result<Vec<Watch>, String> {
    let repository = repository(&app)?;
    repository.list().await.map_err(|error| error.message())
}

#[tauri::command]
pub async fn update_watch(app: AppHandle, id: i64, input: CreateWatch) -> Result<Watch, String> {
    repository(&app)?
        .update(id, input)
        .await
        .map_err(|error| error.message())
}

#[tauri::command]
pub async fn delete_watch(app: AppHandle, id: i64) -> Result<(), String> {
    repository(&app)?
        .delete(id)
        .await
        .map_err(|error| error.message())
}

fn repository(app: &AppHandle) -> Result<WatchRepository, String> {
    #[cfg(feature = "e2e")]
    let directory = std::env::temp_dir().join(format!("skopos-e2e-{}", std::process::id()));

    #[cfg(not(feature = "e2e"))]
    let directory = app
        .path()
        .app_local_data_dir()
        .map_err(|_| "Could not open local watch storage".to_owned())?;

    #[cfg(feature = "e2e")]
    let _ = app;

    std::fs::create_dir_all(&directory)
        .map_err(|_| "Could not open local watch storage".to_owned())?;
    Ok(WatchRepository::new(directory.join("watches.sqlite")))
}
