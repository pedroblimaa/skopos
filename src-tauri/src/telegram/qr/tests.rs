use super::super::protocol::{export_qr_with, QrSender};
use crate::telegram::error::AuthResult;
use grammers_client::tl;
use std::{collections::VecDeque, sync::Mutex, time::Duration};

use grammers_client::{client::PasswordToken, sender::RpcError, InvocationError};

use super::{
    complete_qr_login, now_seconds, request_password, run_qr_login, AuthError, AuthState,
    LoginStep, QrApi, QrEvents, QrOutcome, QrToken,
};
use crate::telegram::client::SessionStatus;

struct FakeApi {
    outcomes: Mutex<VecDeque<AuthResult<QrOutcome>>>,
    status: Mutex<Option<AuthResult<SessionStatus>>>,
    password_error: Mutex<Option<InvocationError>>,
}

impl FakeApi {
    fn new(outcomes: Vec<AuthResult<QrOutcome>>) -> Self {
        Self {
            outcomes: Mutex::new(outcomes.into()),
            status: Mutex::new(Some(Ok(SessionStatus::signed_in("Pedro".into())))),
            password_error: Mutex::new(None),
        }
    }
}

impl QrApi for FakeApi {
    async fn export(&self) -> AuthResult<QrOutcome> {
        self.outcomes.lock().unwrap().pop_front().unwrap()
    }

    async fn status(&self) -> AuthResult<SessionStatus> {
        self.status.lock().unwrap().take().unwrap()
    }

    async fn password_token(&self) -> Result<PasswordToken, InvocationError> {
        if let Some(error) = self.password_error.lock().unwrap().take() {
            return Err(error);
        }

        Ok(PasswordToken::new(tl::types::account::Password {
            has_recovery: false,
            has_secure_values: false,
            has_password: true,
            current_algo: None,
            srp_b: None,
            srp_id: None,
            hint: Some("My hint".into()),
            email_unconfirmed_pattern: None,
            new_algo: tl::enums::PasswordKdfAlgo::Unknown,
            new_secure_algo: tl::enums::SecurePasswordKdfAlgo::Unknown,
            secure_random: vec![],
            pending_reset_date: None,
            login_email_pattern: None,
        }))
    }
}

#[derive(Default)]
struct FakeEvents(Mutex<Vec<String>>);

impl FakeEvents {
    fn names(&self) -> Vec<String> {
        self.0.lock().unwrap().clone()
    }
}

impl QrEvents for FakeEvents {
    fn token(&self, _token: &QrToken) {
        self.0.lock().unwrap().push("token".into());
    }

    fn authenticated(&self, _status: SessionStatus) {
        self.0.lock().unwrap().push("authenticated".into());
    }

    fn password_required(&self, hint: Option<String>) {
        self.0.lock().unwrap().push(format!("password:{hint:?}"));
    }

    fn error(&self, message: String) {
        self.0.lock().unwrap().push(format!("error:{message}"));
    }
}

fn rpc(name: &str) -> AuthError {
    AuthError::from(InvocationError::Rpc(RpcError {
        code: 400,
        name: name.into(),
        value: None,
        caused_by: None,
    }))
}

async fn qr_generation(state: &AuthState) -> u64 {
    state.login.lock().await.begin(LoginStep::Qr)
}

#[tokio::test]
async fn refreshes_token_and_completes_login() {
    let state = AuthState::default();
    let generation = qr_generation(&state).await;
    let api = FakeApi::new(vec![
        Ok(QrOutcome::Token(QrToken {
            url: "tg://login?token=first".into(),
            expires_at: now_seconds(),
        })),
        Ok(QrOutcome::Authorized),
    ]);
    let events = FakeEvents::default();

    tokio::time::timeout(
        Duration::from_secs(3),
        run_qr_login(&state, &api, &events, generation),
    )
    .await
    .unwrap();

    assert_eq!(events.names(), ["token", "authenticated"]);
    assert!(matches!(state.login.lock().await.step, LoginStep::Idle));
}

#[tokio::test]
async fn password_challenge_preserves_password_step() {
    let state = AuthState::default();
    let generation = qr_generation(&state).await;
    let api = FakeApi::new(vec![Err(rpc("SESSION_PASSWORD_NEEDED"))]);
    let events = FakeEvents::default();

    run_qr_login(&state, &api, &events, generation).await;

    assert_eq!(events.names(), ["password:Some(\"My hint\")"]);
    assert!(matches!(
        state.login.lock().await.step,
        LoginStep::Password(_)
    ));
}

#[tokio::test]
async fn reports_export_and_password_lookup_failures() {
    let state = AuthState::default();
    let generation = qr_generation(&state).await;
    let api = FakeApi::new(vec![Err(AuthError::Storage)]);
    let events = FakeEvents::default();

    run_qr_login(&state, &api, &events, generation).await;

    assert!(events.names()[0].contains("local Telegram session"));
    assert!(matches!(state.login.lock().await.step, LoginStep::Idle));

    let generation = qr_generation(&state).await;
    let api = FakeApi::new(vec![Err(rpc("SESSION_PASSWORD_NEEDED"))]);
    *api.password_error.lock().unwrap() = Some(InvocationError::Dropped);
    let events = FakeEvents::default();

    run_qr_login(&state, &api, &events, generation).await;

    assert!(events.names()[0].contains("reach Telegram"));
    assert!(matches!(state.login.lock().await.step, LoginStep::Idle));
}

#[tokio::test]
async fn rejects_incomplete_authorization_and_stale_generation() {
    let state = AuthState::default();
    let generation = qr_generation(&state).await;
    let api = FakeApi::new(vec![]);
    *api.status.lock().unwrap() = Some(Ok(SessionStatus::signed_out()));
    let events = FakeEvents::default();

    complete_qr_login(&state, &api, &events, generation).await;

    assert!(events.names()[0].contains("did not finish QR login"));

    let stale = generation;
    let _current = qr_generation(&state).await;
    let api = FakeApi::new(vec![]);
    let events = FakeEvents::default();

    complete_qr_login(&state, &api, &events, stale).await;

    assert!(events.names().is_empty());
}

#[tokio::test]
async fn reports_status_failure_and_ignores_stale_password() {
    let state = AuthState::default();
    let generation = qr_generation(&state).await;
    let api = FakeApi::new(vec![]);
    *api.status.lock().unwrap() = Some(Err(AuthError::Storage));
    let events = FakeEvents::default();

    complete_qr_login(&state, &api, &events, generation).await;

    assert!(events.names()[0].contains("local Telegram session"));

    let stale = generation;
    let _current = qr_generation(&state).await;
    let events = FakeEvents::default();

    request_password(&state, &FakeApi::new(vec![]), &events, stale).await;

    assert!(events.names().is_empty());
}

struct FakeSender {
    export: Mutex<Option<Result<tl::enums::auth::LoginToken, InvocationError>>>,
    import: Mutex<Option<Result<tl::enums::auth::LoginToken, InvocationError>>>,
    dc: Mutex<Option<i32>>,
    storage_error: bool,
}

impl FakeSender {
    fn new(export: tl::enums::auth::LoginToken) -> Self {
        Self {
            export: Mutex::new(Some(Ok(export))),
            import: Mutex::new(None),
            dc: Mutex::new(None),
            storage_error: false,
        }
    }
}

impl QrSender for FakeSender {
    async fn export(
        &self,
        request: &tl::functions::auth::ExportLoginToken,
    ) -> Result<tl::enums::auth::LoginToken, InvocationError> {
        assert_eq!(request.api_id, 1);
        self.export.lock().unwrap().take().unwrap()
    }

    async fn import(
        &self,
        dc: i32,
        _token: Vec<u8>,
    ) -> Result<tl::enums::auth::LoginToken, InvocationError> {
        assert_eq!(dc, 4);
        self.import.lock().unwrap().take().unwrap()
    }

    async fn set_home_dc(&self, dc: i32) -> AuthResult<()> {
        *self.dc.lock().unwrap() = Some(dc);
        if self.storage_error {
            Err(AuthError::Storage)
        } else {
            Ok(())
        }
    }
}

fn wire_token() -> tl::enums::auth::LoginToken {
    tl::enums::auth::LoginToken::Token(tl::types::auth::LoginToken {
        expires: 123,
        token: vec![1, 2, 3],
    })
}

fn wire_migration() -> tl::enums::auth::LoginToken {
    tl::enums::auth::LoginToken::MigrateTo(tl::types::auth::LoginTokenMigrateTo {
        dc_id: 4,
        token: vec![7],
    })
}

#[tokio::test]
async fn exports_qr_token_and_imports_migrated_token() {
    let sender = FakeSender::new(wire_token());

    let outcome = export_qr_with(&sender, 1, "hash").await.unwrap();

    assert!(
        matches!(outcome, QrOutcome::Token(token) if token.url == "tg://login?token=AQID" && token.expires_at == 123)
    );

    let sender = FakeSender::new(wire_migration());
    *sender.import.lock().unwrap() = Some(Ok(wire_token()));

    let outcome = export_qr_with(&sender, 1, "hash").await.unwrap();

    assert!(matches!(outcome, QrOutcome::Token(_)));
    assert_eq!(*sender.dc.lock().unwrap(), Some(4));
}

#[tokio::test]
async fn reports_qr_export_and_migration_failures() {
    let sender = FakeSender::new(wire_token());
    *sender.export.lock().unwrap() = Some(Err(InvocationError::Dropped));

    assert!(export_qr_with(&sender, 1, "hash").await.is_err());

    let mut sender = FakeSender::new(wire_migration());
    sender.storage_error = true;

    assert!(export_qr_with(&sender, 1, "hash")
        .await
        .err()
        .unwrap()
        .message()
        .contains("local Telegram session"));

    let sender = FakeSender::new(wire_migration());
    *sender.import.lock().unwrap() = Some(Ok(wire_migration()));

    assert!(export_qr_with(&sender, 1, "hash")
        .await
        .err()
        .unwrap()
        .message()
        .contains("QR migration"));
}

#[tokio::test]
async fn concrete_qr_adapter_migrates_and_persists_the_home_dc() {
    use crate::telegram::e2e::test_context;
    use grammers_session::Session;
    use tl::Serializable;

    let (context, fixture) = test_context().await;

    {
        let mut responses = fixture.0.lock().unwrap();
        responses.replies.push_back(Ok(wire_migration().to_bytes()));
        responses.replies.push_back(Ok(wire_token().to_bytes()));
    }

    let response = QrApi::export(&context).await.unwrap();

    assert!(matches!(response, QrOutcome::Token(token) if token.expires_at == 123));
    assert_eq!(context.session.home_dc_id().unwrap(), 4);
}
