use super::SearchState;
use crate::{
    app_message::AppMessage,
    promotion::{match_messages, ResultRepository, SearchResults},
    telegram::AuthState,
    watch::WatchRepository,
};
use std::path::PathBuf;
#[cfg(not(feature = "e2e"))]
use tauri::Manager;
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn search_products(
    app: AppHandle,
    notification_day: String,
) -> Result<SearchResults, AppMessage> {
    super::execution::manual(&app, notification_day).await
}

#[tauri::command]
pub async fn load_search_results(
    app: AppHandle,
    auth: State<'_, AuthState>,
    state: State<'_, SearchState>,
) -> Result<SearchResults, AppMessage> {
    let _operation = state.operation.lock().await;
    let context = auth.client(&app).await.map_err(|error| error.message())?;
    let account = context
        .local_account_id()
        .await
        .map_err(|error| error.message())?;
    let directory = directory(&app)?;
    let watches = WatchRepository::new(directory.join("watches.sqlite"))
        .list()
        .await
        .map_err(|error| error.message())?;
    let (messages, summary) = ResultRepository::new(directory.join("results.sqlite"))
        .load(account)
        .await
        .map_err(|_| AppMessage::SearchStorage)?;

    Ok(SearchResults {
        matches: match_messages(&messages, &watches),
        summary,
    })
}

#[tauri::command]
pub async fn clear_search_results(
    app: AppHandle,
    auth: State<'_, AuthState>,
    state: State<'_, SearchState>,
    before: Option<i64>,
) -> Result<(), AppMessage> {
    if before.is_some_and(|before| before < 0) {
        return Err(AppMessage::InvalidSearchCutoff);
    }

    let _operation = state
        .operation
        .try_lock()
        .map_err(|_| AppMessage::SearchBusy)?;
    let context = auth.client(&app).await.map_err(|error| error.message())?;
    let account = context
        .local_account_id()
        .await
        .map_err(|error| error.message())?;

    ResultRepository::new(directory(&app)?.join("results.sqlite"))
        .clear(account, before)
        .await
        .map_err(|_| AppMessage::SearchStorage)
}

fn directory(app: &AppHandle) -> Result<PathBuf, AppMessage> {
    #[cfg(feature = "e2e")]
    let directory = std::env::temp_dir().join(format!("skopos-e2e-{}", std::process::id()));
    #[cfg(feature = "e2e")]
    let _ = app;
    #[cfg(not(feature = "e2e"))]
    let directory = app
        .path()
        .app_local_data_dir()
        .map_err(|_| AppMessage::SearchStorage)?;
    std::fs::create_dir_all(&directory).map_err(|_| AppMessage::SearchStorage)?;

    Ok(directory)
}
