use super::{
    adapter,
    photos::{self, AccountPhotos, ChatPhotos},
    repository::ChatRepository,
    Chat, ChatError,
};
use crate::{app_message::AppMessage, telegram::AuthState};
use tauri::Manager;
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn list_chats(
    app: AppHandle,
    state: State<'_, AuthState>,
    photos: State<'_, ChatPhotos>,
) -> Result<Vec<Chat>, AppMessage> {
    let context = state.client(&app).await.map_err(|error| error.message())?;
    let account = context
        .local_account_id()
        .await
        .map_err(|error| error.message())?;

    let (chats, locations) = adapter::list(&context.client)
        .await
        .map_err(|error| error.message())?;

    *photos.0.lock().await = Some(AccountPhotos { account, locations });

    Ok(chats)
}

#[tauri::command]
pub async fn get_chat_photo(
    app: AppHandle,
    state: State<'_, AuthState>,
    photos: State<'_, ChatPhotos>,
    id: String,
) -> Result<Option<String>, AppMessage> {
    let context = state.client(&app).await.map_err(|error| error.message())?;
    let account = context
        .local_account_id()
        .await
        .map_err(|error| error.message())?;
    let location = {
        let cached = photos.0.lock().await;
        cached
            .as_ref()
            .filter(|cached| cached.account == account)
            .and_then(|cached| cached.locations.get(&id))
            .cloned()
    };
    let Some(location) = location else {
        return Ok(None);
    };

    photos::download(&context.client, location)
        .await
        .map(Some)
        .map_err(|error| error.message())
}

#[tauri::command]
pub async fn get_selected_chats(
    app: AppHandle,
    state: State<'_, AuthState>,
) -> Result<Vec<Chat>, AppMessage> {
    let context = state.client(&app).await.map_err(|error| error.message())?;
    let account = context
        .local_account_id()
        .await
        .map_err(|error| error.message())?;

    repository(&app)?
        .get(account)
        .await
        .map_err(|error| error.message())
}

#[tauri::command]
pub async fn save_selected_chats(
    app: AppHandle,
    state: State<'_, AuthState>,
    chats: Vec<Chat>,
) -> Result<(), AppMessage> {
    let context = state.client(&app).await.map_err(|error| error.message())?;
    let account = context
        .local_account_id()
        .await
        .map_err(|error| error.message())?;

    repository(&app)?
        .save(account, chats)
        .await
        .map_err(|error| error.message())?;
    app.state::<crate::telegram::monitoring::MonitoringState>()
        .wake
        .notify_one();

    Ok(())
}

fn repository(app: &AppHandle) -> Result<ChatRepository, AppMessage> {
    #[cfg(feature = "e2e")]
    let directory = std::env::temp_dir().join(format!("skopos-e2e-{}", std::process::id()));
    #[cfg(feature = "e2e")]
    let _ = app;

    #[cfg(not(feature = "e2e"))]
    let directory = app
        .path()
        .app_local_data_dir()
        .map_err(|_| ChatError::Storage.message())?;
    std::fs::create_dir_all(&directory).map_err(|_| ChatError::Storage.message())?;

    Ok(ChatRepository::new(directory.join("chats.sqlite")))
}
