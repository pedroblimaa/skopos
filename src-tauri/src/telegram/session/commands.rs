use super::{
    profile,
    workflow::{self, SessionEvents},
};
use crate::app_message::AppMessage;
use crate::telegram::{client::SessionStatus, state::AuthState};
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub async fn session_status(
    app: AppHandle,
    state: State<'_, AuthState>,
) -> Result<SessionStatus, AppMessage> {
    let context = state.client(&app).await.map_err(|error| error.message())?;

    workflow::session_status(context)
        .await
        .map_err(|error| error.message())
}

#[tauri::command]
pub async fn get_profile_photo(
    app: AppHandle,
    state: State<'_, AuthState>,
) -> Result<Option<String>, AppMessage> {
    let context = state.client(&app).await.map_err(|error| error.message())?;

    profile::load_photo(&context.client)
        .await
        .map_err(|error| error.message())
}

#[tauri::command]
pub async fn sign_out(
    app: AppHandle,
    state: State<'_, AuthState>,
    search: State<'_, crate::telegram::search::SearchState>,
    notifications: State<'_, crate::notification::NotificationState>,
) -> Result<(), AppMessage> {
    let _operation = search.cancel_and_wait().await;
    let _notifications = notifications.cancel_and_wait().await;
    let context = state.client(&app).await.map_err(|error| error.message())?;

    workflow::sign_out(&state, context, &TauriEvents(&app))
        .await
        .map_err(|error| error.message())
}

// Auth events are best-effort; command results and session status remain authoritative.
struct TauriEvents<'a>(&'a AppHandle);

impl SessionEvents for TauriEvents<'_> {
    fn signed_out(&self) {
        let _ = self
            .0
            .emit("telegram:auth-changed", SessionStatus::signed_out());
    }
}
