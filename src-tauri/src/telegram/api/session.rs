use super::TelegramApi;
use crate::telegram::{client::SessionStatus, state::LoginStep, AuthState};
use grammers_client::InvocationError;
use std::sync::atomic::Ordering;
use tauri::{Emitter, Manager};

impl TelegramApi {
    pub(in crate::telegram) async fn observe<T>(
        &self,
        generation: u64,
        result: &Result<T, InvocationError>,
    ) {
        let revoked = matches!(result, Err(InvocationError::Rpc(error))
            if error.code == 401 && error.name != "SESSION_PASSWORD_NEEDED");

        if revoked {
            self.reject_session(generation).await;
        }
    }

    pub(super) async fn reject_session(&self, generation: u64) {
        if !self.session.revoke(generation) {
            return;
        }

        let Some(app) = &self.app else {
            return;
        };
        let auth = app.state::<AuthState>();
        let mut login = auth.login.lock().await;
        login.begin(LoginStep::Idle);
        drop(login);
        auth.qr_update.notify_waiters();
        app.state::<crate::telegram::search::SearchState>()
            .cancellation
            .fetch_add(1, Ordering::SeqCst);
        app.state::<crate::notification::NotificationState>()
            .cancel();
        let monitoring = app.state::<crate::telegram::monitoring::MonitoringState>();
        monitoring.is_suspended.store(true, Ordering::SeqCst);
        monitoring.wake.notify_one();
        // Session state is authoritative; the event disposes authenticated UI caches.
        let _ = app.emit("telegram:auth-changed", SessionStatus::signed_out());
    }
}
