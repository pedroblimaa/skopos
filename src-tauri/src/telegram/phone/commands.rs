use super::{
    workflow::{self, validate_phone, AuthEvents},
    CodeRequest, LoginResult,
};
use crate::telegram::{client::SessionStatus, state::AuthState};
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub async fn request_phone_code(
    app: AppHandle,
    state: State<'_, AuthState>,
    phone: String,
) -> Result<CodeRequest, String> {
    validate_phone(&phone).map_err(|error| error.message())?;

    let context = state.client(&app).await.map_err(|error| error.message())?;

    workflow::request_phone_code(&state, context, &TauriEvents(&app), phone)
        .await
        .map_err(|error| error.message())
}

#[tauri::command]
pub async fn submit_phone_code(
    app: AppHandle,
    state: State<'_, AuthState>,
    code: String,
) -> Result<LoginResult, String> {
    if code.trim().is_empty() {
        return Err("Enter the verification code.".into());
    }

    let context = state.client(&app).await.map_err(|error| error.message())?;

    workflow::submit_phone_code(&state, context, &TauriEvents(&app), code)
        .await
        .map_err(|error| error.message())
}

#[tauri::command]
pub async fn submit_password(
    app: AppHandle,
    state: State<'_, AuthState>,
    password: String,
) -> Result<LoginResult, String> {
    let context = state.client(&app).await.map_err(|error| error.message())?;

    workflow::submit_password(&state, context, &TauriEvents(&app), password)
        .await
        .map_err(|error| error.message())
}

// Auth events are best-effort; command results and session status remain authoritative.
struct TauriEvents<'a>(&'a AppHandle);

impl AuthEvents for TauriEvents<'_> {
    fn authenticated(&self, status: &SessionStatus) {
        let _ = self.0.emit("telegram:auth-changed", status);
    }
}
