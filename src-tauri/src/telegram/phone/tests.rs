use std::{collections::VecDeque, sync::Mutex};

use grammers_client::{client::PasswordToken, sender::RpcError, tl, InvocationError, SignInError};

use super::{
    request_phone_code, submit_password, submit_phone_code, validate_phone, AuthError, AuthEvents,
    AuthResult, AuthState, CodeRequest, LoginResult, LoginStep, PhoneRequestApi,
    PhoneRequestOutcome, PhoneSignInApi, SessionStatus, SignInOutcome,
};

struct FakeEvents;

impl AuthEvents for FakeEvents {
    fn authenticated(&self, _status: &SessionStatus) {}
}

struct FakeApi {
    request: Mutex<Option<AuthResult<PhoneRequestOutcome>>>,
    status: Mutex<Option<AuthResult<SessionStatus>>>,
    pause: Option<(&'static str, std::sync::Arc<Gate>)>,
}

impl FakeApi {
    fn new(outcome: AuthResult<PhoneRequestOutcome>) -> Self {
        Self {
            request: Mutex::new(Some(outcome)),
            status: Mutex::new(Some(Ok(SessionStatus::signed_in("Pedro".into())))),
            pause: None,
        }
    }
}

impl PhoneRequestApi for FakeApi {
    async fn request(&self, _phone: &str) -> AuthResult<PhoneRequestOutcome> {
        pause_at(&self.pause, "request").await;
        self.request.lock().unwrap().take().unwrap()
    }

    async fn status(&self) -> AuthResult<SessionStatus> {
        pause_at(&self.pause, "status").await;
        self.status.lock().unwrap().take().unwrap()
    }
}

async fn request(api: &FakeApi, phone: &str) -> AuthResult<CodeRequest> {
    request_phone_code(&AuthState::default(), api, &FakeEvents, phone.into()).await
}

#[test]
fn phone_requires_international_digits() {
    assert!(validate_phone("+5511999999999").is_ok());

    for invalid in [
        "5511999999999",
        "+55 11999999999",
        "+12",
        "+1234567890123456",
    ] {
        assert!(validate_phone(invalid).is_err());
    }
}

#[tokio::test]
async fn phone_request_saves_code_step() {
    let api = FakeApi::new(Ok(PhoneRequestOutcome::Code {
        delivery: CodeRequest::CodeSent {
            message: "Check Telegram".into(),
            length: Some(5),
        },
        hash: "hash".into(),
    }));

    let result = request(&api, "+5511999999999").await.unwrap();

    assert!(matches!(
        result,
        CodeRequest::CodeSent {
            length: Some(5),
            ..
        }
    ));
}

#[tokio::test]
async fn phone_request_can_authorize_immediately() {
    let api = FakeApi::new(Ok(PhoneRequestOutcome::Authorized));

    let result = request(&api, "+5511999999999").await.unwrap();

    assert!(matches!(result, CodeRequest::Authorized { status } if status.authorized()));
}

#[tokio::test]
async fn phone_request_reports_unsupported_telegram_steps() {
    for (outcome, expected) in [
        (PhoneRequestOutcome::EmailSetupRequired, "email setup"),
        (
            PhoneRequestOutcome::PaymentRequired,
            "additional login step",
        ),
    ] {
        let api = FakeApi::new(Ok(outcome));

        assert!(request(&api, "+5511999999999")
            .await
            .unwrap_err()
            .message()
            .contains(expected));
    }
}

#[tokio::test]
async fn phone_request_reports_network_and_status_failures() {
    let api = FakeApi::new(Err(AuthError::Message("Network unavailable")));

    assert_eq!(
        request(&api, "+5511999999999").await.unwrap_err().message(),
        "Network unavailable"
    );

    let api = FakeApi::new(Ok(PhoneRequestOutcome::Authorized));
    *api.status.lock().unwrap() = Some(Err(AuthError::Storage));

    assert!(request(&api, "+5511999999999")
        .await
        .unwrap_err()
        .message()
        .contains("local Telegram session"));

    let api = FakeApi::new(Ok(PhoneRequestOutcome::Authorized));
    *api.status.lock().unwrap() = Some(Ok(SessionStatus::signed_out()));

    assert!(request(&api, "+5511999999999")
        .await
        .unwrap_err()
        .message()
        .contains("did not finish login"));
}

fn rpc(name: &str) -> InvocationError {
    InvocationError::Rpc(RpcError {
        code: 400,
        name: name.into(),
        value: None,
        caused_by: None,
    })
}

fn token() -> PasswordToken {
    PasswordToken::new(tl::types::account::Password {
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
    })
}

struct FakeSignInApi {
    sign_in: Mutex<Option<Result<SignInOutcome, InvocationError>>>,
    status: Mutex<Option<AuthResult<SessionStatus>>>,
    password_token: Mutex<Option<Result<PasswordToken, InvocationError>>>,
    password_results: Mutex<VecDeque<Result<(), SignInError>>>,
    pause: Option<(&'static str, std::sync::Arc<Gate>)>,
}

impl FakeSignInApi {
    fn new(result: Result<SignInOutcome, InvocationError>) -> Self {
        Self {
            sign_in: Mutex::new(Some(result)),
            status: Mutex::new(Some(Ok(SessionStatus::signed_in("Pedro".into())))),
            password_token: Mutex::new(Some(Ok(token()))),
            password_results: Mutex::new(VecDeque::new()),
            pause: None,
        }
    }
}

impl PhoneSignInApi for FakeSignInApi {
    async fn sign_in(
        &self,
        _phone: &str,
        _hash: &str,
        _code: &str,
    ) -> Result<SignInOutcome, InvocationError> {
        pause_at(&self.pause, "code").await;
        self.sign_in.lock().unwrap().take().unwrap()
    }

    async fn password_token(&self) -> Result<PasswordToken, InvocationError> {
        pause_at(&self.pause, "token").await;
        self.password_token.lock().unwrap().take().unwrap()
    }

    async fn check_password(
        &self,
        _token: PasswordToken,
        _password: &str,
    ) -> Result<(), SignInError> {
        pause_at(&self.pause, "password").await;
        self.password_results.lock().unwrap().pop_front().unwrap()
    }

    async fn status(&self) -> AuthResult<SessionStatus> {
        pause_at(&self.pause, "status").await;
        self.status.lock().unwrap().take().unwrap()
    }
}

async fn phone_state() -> AuthState {
    let state = AuthState::default();
    state.login.lock().await.begin(LoginStep::Phone {
        phone: "+5511999999999".into(),
        hash: "hash".into(),
    });

    state
}

#[tokio::test]
async fn code_submission_authorizes_and_resets_state() {
    let state = phone_state().await;
    let api = FakeSignInApi::new(Ok(SignInOutcome::Authorized));

    let result = submit_phone_code(&state, &api, &FakeEvents, " 12345 ".into())
        .await
        .unwrap();

    assert!(matches!(result, LoginResult::Authorized { status } if status.authorized()));
    assert!(matches!(state.login.lock().await.step, LoginStep::Idle));
}

#[tokio::test]
async fn invalid_code_preserves_step_but_expired_code_resets_it() {
    for (error, expected_step) in [
        ("PHONE_CODE_INVALID", "phone"),
        ("PHONE_CODE_EMPTY", "phone"),
        ("PHONE_CODE_EXPIRED", "idle"),
    ] {
        let state = phone_state().await;
        let api = FakeSignInApi::new(Err(rpc(error)));

        assert!(submit_phone_code(&state, &api, &FakeEvents, "12345".into())
            .await
            .err()
            .unwrap()
            .message()
            .contains("code"));

        let step = &state.login.lock().await.step;
        assert_eq!(
            matches!(step, LoginStep::Phone { .. }),
            expected_step == "phone"
        );
    }
}

#[tokio::test]
async fn signup_and_other_errors_keep_code_entry_available() {
    for result in [Ok(SignInOutcome::SignUpRequired), Err(rpc("FLOOD_WAIT"))] {
        let state = phone_state().await;
        let api = FakeSignInApi::new(result);

        assert!(submit_phone_code(&state, &api, &FakeEvents, "12345".into())
            .await
            .is_err());
        assert!(matches!(
            state.login.lock().await.step,
            LoginStep::Phone { .. }
        ));
    }
}

#[tokio::test]
async fn code_submission_handles_password_challenge_and_retry() {
    let state = phone_state().await;
    let api = FakeSignInApi::new(Err(rpc("SESSION_PASSWORD_NEEDED")));

    let result = submit_phone_code(&state, &api, &FakeEvents, "12345".into())
        .await
        .unwrap();

    assert!(
        matches!(result, LoginResult::PasswordRequired { hint: Some(hint) } if hint == "My hint")
    );
    assert!(matches!(
        state.login.lock().await.step,
        LoginStep::Password(_)
    ));

    api.password_results
        .lock()
        .unwrap()
        .push_back(Err(SignInError::InvalidPassword(token())));

    let error = submit_password(&state, &api, &FakeEvents, "wrong".into())
        .await
        .err()
        .unwrap()
        .message();

    assert!(error.contains("incorrect"));
    assert!(matches!(
        state.login.lock().await.step,
        LoginStep::Password(_)
    ));

    api.password_results.lock().unwrap().push_back(Ok(()));

    let result = submit_password(&state, &api, &FakeEvents, "secret".into())
        .await
        .unwrap();

    assert!(matches!(result, LoginResult::Authorized { status } if status.authorized()));
}

#[tokio::test]
async fn password_lookup_failure_clears_submission_step() {
    let state = phone_state().await;
    let api = FakeSignInApi::new(Err(rpc("SESSION_PASSWORD_NEEDED")));
    *api.password_token.lock().unwrap() = Some(Err(InvocationError::Dropped));

    assert!(submit_phone_code(&state, &api, &FakeEvents, "12345".into())
        .await
        .is_err());
    assert!(matches!(state.login.lock().await.step, LoginStep::Idle));
}

#[tokio::test]
async fn code_and_password_require_the_correct_step() {
    let state = AuthState::default();
    let api = FakeSignInApi::new(Ok(SignInOutcome::Authorized));

    assert!(submit_phone_code(&state, &api, &FakeEvents, "  ".into())
        .await
        .err()
        .unwrap()
        .message()
        .contains("Enter the verification code"));
    assert!(submit_phone_code(&state, &api, &FakeEvents, "12345".into())
        .await
        .err()
        .unwrap()
        .message()
        .contains("Request a login code"));
    assert!(submit_password(&state, &api, &FakeEvents, "secret".into())
        .await
        .err()
        .unwrap()
        .message()
        .contains("Start Telegram login again"));
}

#[tokio::test]
async fn concrete_phone_adapter_classifies_telegram_responses() {
    use crate::telegram::e2e::{authorization, test_context};
    use tl::Serializable;

    let (context, fixture) = test_context().await;
    let state = AuthState::default();

    let response = request_phone_code(&state, &context, &FakeEvents, "+5511999999999".into())
        .await
        .unwrap();

    assert!(matches!(
        response,
        CodeRequest::CodeSent {
            length: Some(5),
            ..
        }
    ));

    let result = submit_phone_code(&state, &context, &FakeEvents, "12345".into())
        .await
        .unwrap();

    assert!(matches!(result, LoginResult::Authorized { .. }));

    fixture.0.lock().unwrap().scenario.immediate_authorized = true;

    assert!(matches!(
        PhoneRequestApi::request(&context, "+5511999999999")
            .await
            .unwrap(),
        PhoneRequestOutcome::Authorized
    ));

    fixture.0.lock().unwrap().scenario.immediate_authorized = false;
    let email: tl::enums::auth::SentCode = tl::types::auth::SentCode {
        r#type: tl::types::auth::SentCodeTypeSetUpEmailRequired {
            apple_signin_allowed: false,
            google_signin_allowed: false,
        }
        .into(),
        phone_code_hash: "hash".into(),
        next_type: None,
        timeout: None,
    }
    .into();
    fixture
        .0
        .lock()
        .unwrap()
        .replies
        .push_back(Ok(email.to_bytes()));

    assert!(matches!(
        PhoneRequestApi::request(&context, "+5511999999999")
            .await
            .unwrap(),
        PhoneRequestOutcome::EmailSetupRequired
    ));

    let signup: tl::enums::auth::Authorization = tl::types::auth::AuthorizationSignUpRequired {
        terms_of_service: None,
    }
    .into();
    fixture
        .0
        .lock()
        .unwrap()
        .replies
        .push_back(Ok(signup.to_bytes()));

    assert!(matches!(
        context
            .sign_in("+5511999999999", "hash", "12345")
            .await
            .unwrap(),
        SignInOutcome::SignUpRequired
    ));

    fixture
        .0
        .lock()
        .unwrap()
        .replies
        .push_back(Ok(authorization().to_bytes()));

    assert!(matches!(
        context
            .sign_in("+5511999999999", "hash", "12345")
            .await
            .unwrap(),
        SignInOutcome::Authorized
    ));

    fixture.0.lock().unwrap().scenario.password_required = true;

    request_phone_code(&state, &context, &FakeEvents, "+5511999999999".into())
        .await
        .unwrap();

    assert!(matches!(
        submit_phone_code(&state, &context, &FakeEvents, "12345".into())
            .await
            .unwrap(),
        LoginResult::PasswordRequired { .. }
    ));

    assert!(
        submit_password(&state, &context, &FakeEvents, "wrong".into())
            .await
            .is_err()
    );

    assert!(matches!(
        submit_password(&state, &context, &FakeEvents, "secret".into())
            .await
            .unwrap(),
        LoginResult::Authorized { .. }
    ));
}

#[derive(Default)]
struct Gate {
    entered: tokio::sync::Notify,
    release: tokio::sync::Notify,
}

async fn pause_at(pause: &Option<(&str, std::sync::Arc<Gate>)>, point: &str) {
    let Some((selected, gate)) = pause else {
        return;
    };

    if *selected != point {
        return;
    }

    gate.entered.notify_one();
    gate.release.notified().await;
}

async fn replace_attempt(state: &AuthState, gate: &Gate) {
    gate.entered.notified().await;
    state.login.lock().await.begin(LoginStep::Qr);
    gate.release.notify_one();
}

#[tokio::test]
async fn phone_request_cannot_commit_after_another_login_replaces_it() {
    for point in ["request", "status"] {
        let state = AuthState::default();
        let gate = std::sync::Arc::new(Gate::default());
        let mut api = FakeApi::new(Ok(PhoneRequestOutcome::Authorized));
        api.pause = Some((point, gate.clone()));

        let (result, ()) = tokio::join!(
            request_phone_code(&state, &api, &FakeEvents, "+5511999999999".into()),
            replace_attempt(&state, &gate)
        );

        assert!(result.err().unwrap().message().contains("replaced"));
        assert!(matches!(state.login.lock().await.step, LoginStep::Qr));
    }
}

#[tokio::test]
async fn code_submission_cannot_commit_a_stale_response_or_password_token() {
    for point in ["code", "token", "token_failure", "status"] {
        let state = phone_state().await;
        let gate = std::sync::Arc::new(Gate::default());
        let mut api = FakeSignInApi::new(if point.starts_with("token") {
            Err(rpc("SESSION_PASSWORD_NEEDED"))
        } else {
            Ok(SignInOutcome::Authorized)
        });
        if point == "token_failure" {
            *api.password_token.lock().unwrap() = Some(Err(InvocationError::Dropped));
            api.pause = Some(("token", gate.clone()));
        } else {
            api.pause = Some((point, gate.clone()));
        }

        let (result, ()) = tokio::join!(
            submit_phone_code(&state, &api, &FakeEvents, "12345".into()),
            replace_attempt(&state, &gate)
        );

        assert!(result.err().unwrap().message().contains("replaced"));
        assert!(matches!(state.login.lock().await.step, LoginStep::Qr));
    }
}

fn password_failure(kind: &str) -> SignInError {
    match kind {
        "invalid" => SignInError::InvalidPassword(token()),
        "network" => SignInError::Other(InvocationError::Dropped),
        _ => SignInError::InvalidCode,
    }
}

#[tokio::test]
async fn password_failure_resets_the_step_and_stale_failures_preserve_new_login() {
    for cancelled in [false, true] {
        for kind in ["invalid", "network", "other"] {
            let state = AuthState::default();
            state
                .login
                .lock()
                .await
                .begin(LoginStep::Password(Box::new(token())));
            let gate = std::sync::Arc::new(Gate::default());
            let mut api = FakeSignInApi::new(Ok(SignInOutcome::Authorized));
            api.password_results
                .lock()
                .unwrap()
                .push_back(Err(password_failure(kind)));

            let error = if cancelled {
                api.pause = Some(("password", gate.clone()));

                let (result, ()) = tokio::join!(
                    submit_password(&state, &api, &FakeEvents, "wrong".into()),
                    replace_attempt(&state, &gate)
                );
                result.err().unwrap().message()
            } else {
                submit_password(&state, &api, &FakeEvents, "wrong".into())
                    .await
                    .err()
                    .unwrap()
                    .message()
            };

            if cancelled {
                assert!(error.contains("replaced"));
                assert!(matches!(state.login.lock().await.step, LoginStep::Qr));
            } else if kind == "invalid" {
                assert!(error.contains("incorrect"));
                assert!(matches!(
                    state.login.lock().await.step,
                    LoginStep::Password(_)
                ));
            } else {
                assert!(matches!(state.login.lock().await.step, LoginStep::Idle));
            }
        }
    }
}

#[tokio::test]
async fn finishing_login_rejects_missing_authorization_and_status_failure() {
    for result in [Ok(SessionStatus::signed_out()), Err(AuthError::Storage)] {
        let state = phone_state().await;
        let api = FakeSignInApi::new(Ok(SignInOutcome::Authorized));
        *api.status.lock().unwrap() = Some(result);

        assert!(submit_phone_code(&state, &api, &FakeEvents, "12345".into())
            .await
            .is_err());
        assert!(matches!(state.login.lock().await.step, LoginStep::Idle));
    }
}

#[tokio::test]
async fn concrete_phone_adapter_rejects_payment_required_response() {
    use crate::telegram::e2e::test_context;
    use tl::Serializable;

    let (context, fixture) = test_context().await;
    let response: tl::enums::auth::SentCode = tl::types::auth::SentCodePaymentRequired {
        store_product: "login".into(),
        phone_code_hash: "hash".into(),
        support_email_address: "support".into(),
        support_email_subject: "login".into(),
        premium_days: 1,
        currency: "USD".into(),
        amount: 1,
    }
    .into();
    fixture
        .0
        .lock()
        .unwrap()
        .replies
        .push_back(Ok(response.to_bytes()));

    assert!(matches!(
        PhoneRequestApi::request(&context, "+5511999999999")
            .await
            .unwrap(),
        PhoneRequestOutcome::PaymentRequired
    ));
}

#[tokio::test]
async fn immediate_authorization_emits_after_unlocking_and_before_returning() {
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct Events<'a> {
        state: &'a AuthState,
        count: AtomicUsize,
    }

    impl AuthEvents for Events<'_> {
        fn authenticated(&self, status: &SessionStatus) {
            assert!(status.authorized());
            assert!(matches!(
                self.state.login.try_lock().unwrap().step,
                LoginStep::Idle
            ));
            self.count.fetch_add(1, Ordering::SeqCst);
        }
    }

    let state = AuthState::default();
    let events = Events {
        state: &state,
        count: AtomicUsize::new(0),
    };
    let api = FakeApi::new(Ok(PhoneRequestOutcome::Authorized));

    let result = request_phone_code(&state, &api, &events, "+5511999999999".into())
        .await
        .unwrap();

    assert!(matches!(result, CodeRequest::Authorized { status } if status.authorized()));
    assert_eq!(events.count.load(Ordering::SeqCst), 1);
}
