use crate::app_message::AppMessage;
use crate::telegram::error::AuthResult;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Mutex,
};

use super::{
    session_status, sign_out, AuthError, AuthState, LoginStep, SessionApi, SessionEvents,
    SessionStatus,
};

struct FakeApi {
    status: Mutex<Option<AuthResult<SessionStatus>>>,
    sign_out: Mutex<Option<AuthResult<()>>>,
}

impl FakeApi {
    fn new() -> Self {
        Self {
            status: Mutex::new(Some(Ok(SessionStatus::signed_in("Pedro".into())))),
            sign_out: Mutex::new(Some(Ok(()))),
        }
    }
}

impl SessionApi for FakeApi {
    async fn status(&self) -> AuthResult<SessionStatus> {
        self.status.lock().unwrap().take().unwrap()
    }

    async fn sign_out(&self) -> AuthResult<()> {
        self.sign_out.lock().unwrap().take().unwrap()
    }
}

#[derive(Default)]
struct FakeEvents(AtomicUsize);

impl SessionEvents for FakeEvents {
    fn signed_out(&self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

#[tokio::test]
async fn status_reports_authorized_and_storage_failure() {
    let api = FakeApi::new();

    assert!(session_status(&api).await.unwrap().authorized());

    let api = FakeApi::new();
    *api.status.lock().unwrap() = Some(Err(AuthError::Storage));

    assert_eq!(
        session_status(&api).await.err().unwrap().message(),
        AppMessage::AuthStorage
    );
}

#[tokio::test]
async fn sign_out_emits_only_after_success() {
    let state = AuthState::default();
    let events = FakeEvents::default();

    sign_out(&state, &FakeApi::new(), &events).await.unwrap();

    assert_eq!(events.0.load(Ordering::SeqCst), 1);
    assert!(matches!(state.login.lock().await.step, LoginStep::Idle));

    let api = FakeApi::new();
    *api.sign_out.lock().unwrap() = Some(Err(AuthError::Message(AppMessage::AuthNetwork)));

    assert_eq!(
        sign_out(&state, &api, &events)
            .await
            .err()
            .unwrap()
            .message(),
        AppMessage::AuthNetwork
    );
    assert_eq!(events.0.load(Ordering::SeqCst), 1);
    assert!(matches!(state.login.lock().await.step, LoginStep::Idle));
}

#[tokio::test]
async fn sign_out_rejects_concurrent_request() {
    let state = AuthState::default();
    state.login.lock().await.begin(LoginStep::SigningOut);

    let result = sign_out(&state, &FakeApi::new(), &FakeEvents::default()).await;

    assert_eq!(
        result.err().unwrap().message(),
        AppMessage::SignOutInProgress
    );
}
