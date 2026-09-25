use tauri::{AppHandle, Emitter, State};

use super::{
    client::{status_for, SessionStatus},
    error::AuthError,
    state::{AuthState, LoginStep},
};

#[tauri::command]
pub async fn session_status(
    app: AppHandle,
    state: State<'_, AuthState>,
) -> Result<SessionStatus, String> {
    let context = state.client(&app).await.map_err(|error| error.message())?;
    status_for(&context.client)
        .await
        .map_err(|error| error.message())
}

#[tauri::command]
pub async fn sign_out(app: AppHandle, state: State<'_, AuthState>) -> Result<(), String> {
    let context = state.client(&app).await.map_err(|error| error.message())?;
    let generation = {
        let mut login = state.login.lock().await;
        if matches!(login.step, LoginStep::SigningOut) {
            return Err("Sign-out is already in progress.".into());
        }
        login.begin(LoginStep::SigningOut)
    };
    state.qr_update.notify_waiters();

    if let Err(error) = context.client.sign_out().await {
        let mut login = state.login.lock().await;
        login.reset_if(generation, &LoginStep::SigningOut);
        return Err(AuthError::from(error).message());
    }

    let mut login = state.login.lock().await;
    if !login.reset_if(generation, &LoginStep::SigningOut) {
        return Err(AuthError::Cancelled.message());
    }
    drop(login);
    let _ = app.emit("telegram:auth-changed", SessionStatus::signed_out());
    Ok(())
}
