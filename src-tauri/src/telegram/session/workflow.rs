use super::adapter::SessionApi;
use crate::app_message::AppMessage;
use crate::telegram::{
    client::SessionStatus,
    error::{AuthError, AuthResult},
    state::{AuthState, LoginStep},
};

pub(super) trait SessionEvents {
    fn signed_out(&self);
}

pub(super) async fn session_status<A: SessionApi>(api: &A) -> AuthResult<SessionStatus> {
    api.status().await
}

pub(super) async fn sign_out<A: SessionApi, E: SessionEvents>(
    state: &AuthState,
    api: &A,
    events: &E,
) -> AuthResult<()> {
    let generation = {
        let mut login = state.login.lock().await;
        if matches!(login.step, LoginStep::SigningOut) {
            return Err(AuthError::Message(AppMessage::SignOutInProgress));
        }

        login.begin(LoginStep::SigningOut)
    };
    state.qr_update.notify_waiters();

    if let Err(error) = api.sign_out().await {
        let mut login = state.login.lock().await;
        login.reset_if(generation, &LoginStep::SigningOut);
        return Err(error);
    }

    let mut login = state.login.lock().await;
    if !login.reset_if(generation, &LoginStep::SigningOut) {
        return Err(AuthError::Cancelled);
    }
    drop(login);

    events.signed_out();
    Ok(())
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
