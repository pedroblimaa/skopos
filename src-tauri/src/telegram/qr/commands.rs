use super::{
    protocol::QrToken,
    workflow::{self, QrEvents},
};
use crate::app_message::AppMessage;
use crate::telegram::{
    client::{ClientContext, SessionStatus},
    state::{AuthState, LoginStep},
};
use tauri::{AppHandle, Emitter, Manager, State};

#[tauri::command]
pub async fn start_qr_login(app: AppHandle, state: State<'_, AuthState>) -> Result<(), AppMessage> {
    let context = state
        .client(&app)
        .await
        .map_err(|error| error.message())?
        .clone();

    let generation = {
        let mut login = state.login.lock().await;
        login
            .begin_login(LoginStep::Qr)
            .map_err(|error| error.message())?
    };

    state.qr_update.notify_waiters();
    tauri::async_runtime::spawn(run_qr_login(app, context, generation));

    Ok(())
}

#[tauri::command]
pub async fn stop_qr_login(state: State<'_, AuthState>) -> Result<(), AppMessage> {
    let mut login = state.login.lock().await;

    if matches!(login.step, LoginStep::Qr) {
        login.begin(LoginStep::Idle);
        state.qr_update.notify_waiters();
    }

    Ok(())
}

async fn run_qr_login(app: AppHandle, context: ClientContext, generation: u64) {
    let state = app.state::<AuthState>();

    workflow::run_qr_login(&state, &context, &TauriEvents(&app), generation).await;
}

// Auth events are best-effort; command results and session status remain authoritative.
struct TauriEvents<'a>(&'a AppHandle);

impl QrEvents for TauriEvents<'_> {
    fn token(&self, token: &QrToken) {
        let _ = self.0.emit("telegram:qr-token", token);
    }

    fn authenticated(&self, status: SessionStatus) {
        let _ = self.0.emit("telegram:auth-changed", status);
    }

    fn password_required(&self, hint: Option<String>) {
        let _ = self.0.emit("telegram:password-required", hint);
    }

    fn error(&self, message: AppMessage) {
        let _ = self.0.emit("telegram:auth-error", message);
    }
}
