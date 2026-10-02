use super::{
    workflow::{self, validate_phone, AuthEvents},
    CodeRequest, LoginResult,
};
use crate::app_message::AppMessage;
use crate::telegram::{
    client::SessionStatus,
    state::{AuthState, LoginStep},
};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeSubmissionError {
    message: AppMessage,
    can_retry_code: bool,
}

#[tauri::command]
pub async fn request_phone_code(
    app: AppHandle,
    state: State<'_, AuthState>,
    phone: String,
) -> Result<CodeRequest, AppMessage> {
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
) -> Result<LoginResult, CodeSubmissionError> {
    if code.trim().is_empty() {
        return Err(CodeSubmissionError {
            message: AppMessage::MissingCode,
            can_retry_code: true,
        });
    }

    let context = state
        .client(&app)
        .await
        .map_err(|error| CodeSubmissionError {
            message: error.message(),
            can_retry_code: false,
        })?;

    match workflow::submit_phone_code(&state, context, &TauriEvents(&app), code).await {
        Ok(result) => Ok(result),
        Err(error) => Err(CodeSubmissionError {
            message: error.message(),
            can_retry_code: matches!(state.login.lock().await.step, LoginStep::Phone { .. }),
        }),
    }
}

#[tauri::command]
pub async fn submit_password(
    app: AppHandle,
    state: State<'_, AuthState>,
    password: String,
) -> Result<LoginResult, AppMessage> {
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
