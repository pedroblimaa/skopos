use super::{
    adapter::QrApi,
    protocol::{QrOutcome, QrToken},
};
use crate::telegram::{
    client::SessionStatus,
    error::AuthError,
    state::{AuthState, LoginStep},
};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub(super) trait QrEvents {
    fn token(&self, token: &QrToken);
    fn authenticated(&self, status: SessionStatus);
    fn password_required(&self, hint: Option<String>);
    fn error(&self, message: String);
}

pub(super) async fn run_qr_login<A: QrApi, E: QrEvents>(
    state: &AuthState,
    api: &A,
    events: &E,
    generation: u64,
) {
    while is_current(state, generation).await {
        let result = api.export().await;

        if !is_current(state, generation).await {
            break;
        }

        match result {
            Ok(QrOutcome::Token(token)) => {
                let wait = token.expires_at.saturating_sub(now_seconds()).max(1);
                events.token(&token);

                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(wait)) => {},
                    _ = state.qr_update.notified() => {},
                }
            }
            Ok(QrOutcome::Authorized) => {
                complete_qr_login(state, api, events, generation).await;
                break;
            }
            Err(error) if error.is_rpc("SESSION_PASSWORD_NEEDED") => {
                request_password(state, api, events, generation).await;
                break;
            }
            Err(error) => {
                if is_current(state, generation).await {
                    events.error(error.message());
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

async fn complete_qr_login<A: QrApi, E: QrEvents>(
    state: &AuthState,
    api: &A,
    events: &E,
    generation: u64,
) {
    let result = api.status().await;

    let mut login = state.login.lock().await;
    if !login.reset_if(generation, &LoginStep::Qr) {
        return;
    }
    drop(login);

    match result {
        Ok(status) if status.authorized() => {
            events.authenticated(status);
        }
        Ok(_) => {
            events.error("Telegram did not finish QR login. Scan the refreshed code again.".into());
        }
        Err(error) => {
            events.error(error.message());
        }
    }
}

async fn request_password<A: QrApi, E: QrEvents>(
    state: &AuthState,
    api: &A,
    events: &E,
    generation: u64,
) {
    let result = api.password_token().await;

    let mut login = state.login.lock().await;
    if !login.is_qr(generation) {
        return;
    }

    match result {
        Ok(token) => {
            let hint = token.hint().map(str::to_owned);
            login.step = LoginStep::Password(Box::new(token));
            drop(login);

            events.password_required(hint);
        }
        Err(error) => {
            login.step = LoginStep::Idle;
            drop(login);

            events.error(AuthError::from(error).message());
        }
    }
}

async fn is_current(state: &AuthState, generation: u64) -> bool {
    state.login.lock().await.is_qr(generation)
}

fn now_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
