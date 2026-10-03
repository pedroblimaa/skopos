use super::{
    workflow::{self, SearchRun},
    SearchState,
};
use crate::{
    app_message::AppMessage,
    promotion::{match_messages, ResultRepository, SearchResults},
    telegram::{
        chats::{adapter, repository::ChatRepository},
        AuthState,
    },
    watch::WatchRepository,
};
use std::{
    path::PathBuf,
    sync::atomic::Ordering,
    time::{SystemTime, UNIX_EPOCH},
};
#[cfg(not(feature = "e2e"))]
use tauri::Manager;
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn search_products(
    app: AppHandle,
    auth: State<'_, AuthState>,
    state: State<'_, SearchState>,
) -> Result<SearchResults, AppMessage> {
    let _operation = state
        .operation
        .try_lock()
        .map_err(|_| AppMessage::SearchBusy)?;
    let generation = auth.login.lock().await.generation;
    let cancellation = state.cancellation.load(Ordering::SeqCst);
    let until = now()?;
    let directory = directory(&app)?;
    let watches = WatchRepository::new(directory.join("watches.sqlite"))
        .list()
        .await
        .map_err(|error| error.message())?;
    if watches.is_empty() {
        return Err(AppMessage::SearchNeedsProducts);
    }

    let context = auth.client(&app).await.map_err(|error| error.message())?;
    let account = adapter::account_id(&context.client)
        .await
        .map_err(|error| error.message())?;
    let selected = ChatRepository::new(directory.join("chats.sqlite"))
        .get(account)
        .await
        .map_err(|error| error.message())?;
    if selected.is_empty() {
        return Err(AppMessage::SearchNeedsChats);
    }

    let repository = ResultRepository::new(directory.join("results.sqlite"));
    let (saved, _) = repository
        .load(account)
        .await
        .map_err(|_| AppMessage::SearchStorage)?;
    let saved_messages = saved
        .into_iter()
        .map(|message| (message.chat_id, message.message_id))
        .collect();

    let run = SearchRun {
        api: &context.client,
        auth: &auth,
        state: &state,
        generation,
        cancellation,
        since: until - 86_400,
        until,
        saved_messages,
    };
    let (messages, summary) = workflow::search(&run, selected, &watches)
        .await
        .map_err(|error| error.message())?;
    run.ensure_current()
        .await
        .map_err(|error| error.message())?;

    repository
        .merge(account, messages, summary)
        .await
        .map_err(|_| AppMessage::SearchStorage)?;

    let (messages, summary) = repository
        .load(account)
        .await
        .map_err(|_| AppMessage::SearchStorage)?;
    let current_watches = WatchRepository::new(directory.join("watches.sqlite"))
        .list()
        .await
        .map_err(|error| error.message())?;

    Ok(SearchResults {
        matches: match_messages(&messages, &current_watches),
        summary,
    })
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
    let account = adapter::account_id(&context.client)
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

fn now() -> Result<i64, AppMessage> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| i64::try_from(duration.as_secs()).ok())
        .ok_or(AppMessage::SearchFailed)
}
