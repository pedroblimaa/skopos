use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use grammers_client::{client::PasswordToken, tl};
use grammers_session::Session;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use super::{
    client::{credentials, status_for, ClientContext},
    error::{AuthError, AuthResult},
    state::{AuthState, LoginStep},
};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct QrToken {
    url: String,
    expires_at: u64,
}

enum QrOutcome {
    Token(QrToken),
    Authorized,
}

async fn export_qr(context: &ClientContext) -> AuthResult<QrOutcome> {
    let (api_id, api_hash) = credentials()?;
    let response = context
        .client
        .invoke(&tl::functions::auth::ExportLoginToken {
            api_id,
            api_hash: api_hash.to_owned(),
            except_ids: vec![],
        })
        .await?;

    let response = match response {
        tl::enums::auth::LoginToken::MigrateTo(migration) => {
            context
                .session
                .set_home_dc_id(migration.dc_id)
                .await
                .map_err(|_| AuthError::Storage)?;
            context
                .client
                .invoke_in_dc(
                    migration.dc_id,
                    &tl::functions::auth::ImportLoginToken {
                        token: migration.token,
                    },
                )
                .await?
        }
        other => other,
    };

    match response {
        tl::enums::auth::LoginToken::Token(token) => Ok(QrOutcome::Token(QrToken {
            url: format!("tg://login?token={}", URL_SAFE_NO_PAD.encode(token.token)),
            expires_at: token.expires as u64,
        })),
        tl::enums::auth::LoginToken::Success(_) => Ok(QrOutcome::Authorized),
        tl::enums::auth::LoginToken::MigrateTo(_) => Err(AuthError::Message(
            "Telegram could not complete the QR migration. Try again.",
        )),
    }
}

#[tauri::command]
pub async fn start_qr_login(app: AppHandle, state: State<'_, AuthState>) -> Result<(), String> {
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

async fn is_current(app: &AppHandle, generation: u64) -> bool {
    app.state::<AuthState>()
        .login
        .lock()
        .await
        .is_qr(generation)
}

async fn run_qr_login(app: AppHandle, context: ClientContext, generation: u64) {
    let state = app.state::<AuthState>();

    while is_current(&app, generation).await {
        let result = export_qr(&context).await;
        if !is_current(&app, generation).await {
            break;
        }

        match result {
            Ok(QrOutcome::Token(token)) => {
                let wait = token.expires_at.saturating_sub(now_seconds()).max(1);
                let _ = app.emit("telegram:qr-token", &token);
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(wait)) => {},
                    _ = state.qr_update.notified() => {},
                }
            }
            Ok(QrOutcome::Authorized) => {
                complete_qr_login(&app, &context, generation).await;
                break;
            }
            Err(error) if error.is_rpc("SESSION_PASSWORD_NEEDED") => {
                request_password(&app, &context, generation).await;
                break;
            }
            Err(error) => {
                if is_current(&app, generation).await {
                    let _ = app.emit("telegram:auth-error", error.message());
                }
                break;
            }
        }
    }
    state
        .login
        .lock()
        .await
        .reset_if(generation, &LoginStep::Qr);
}

async fn complete_qr_login(app: &AppHandle, context: &ClientContext, generation: u64) {
    let result = status_for(&context.client).await;
    let state = app.state::<AuthState>();
    let mut login = state.login.lock().await;
    if !login.reset_if(generation, &LoginStep::Qr) {
        return;
    }
    drop(login);
    match result {
        Ok(status) if status.authorized() => {
            let _ = app.emit("telegram:auth-changed", status);
        }
        Ok(_) => {
            let _ = app.emit(
                "telegram:auth-error",
                "Telegram did not finish QR login. Scan the refreshed code again.",
            );
        }
        Err(error) => {
            let _ = app.emit("telegram:auth-error", error.message());
        }
    }
}

async fn request_password(app: &AppHandle, context: &ClientContext, generation: u64) {
    let result = context
        .client
        .invoke(&tl::functions::account::GetPassword {})
        .await;
    let state = app.state::<AuthState>();
    let mut login = state.login.lock().await;
    if !login.is_qr(generation) {
        return;
    }
    match result {
        Ok(info) => {
            let token = PasswordToken::new(info.into());
            let hint = token.hint().map(str::to_owned);
            login.step = LoginStep::Password(Box::new(token));
            drop(login);
            let _ = app.emit("telegram:password-required", hint);
        }
        Err(error) => {
            login.step = LoginStep::Idle;
            drop(login);
            let _ = app.emit("telegram:auth-error", AuthError::from(error).message());
        }
    }
}

fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[tauri::command]
pub async fn stop_qr_login(state: State<'_, AuthState>) -> Result<(), String> {
    let mut login = state.login.lock().await;
    if matches!(login.step, LoginStep::Qr) {
        login.begin(LoginStep::Idle);
        state.qr_update.notify_waiters();
    }
    Ok(())
}
