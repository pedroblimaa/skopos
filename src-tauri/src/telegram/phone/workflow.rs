use super::{
    adapter::{PhoneRequestApi, PhoneRequestOutcome, PhoneSignInApi, SignInOutcome},
    CodeRequest,
};
use crate::app_message::AppMessage;
use crate::telegram::{
    client::SessionStatus,
    error::{AuthError, AuthResult},
    state::{AuthState, LoginStep},
};
use grammers_client::SignInError;
use serde::Serialize;

#[derive(Serialize)]
#[serde(tag = "step", rename_all = "camelCase")]
pub enum LoginResult {
    Authorized { status: SessionStatus },
    PasswordRequired { hint: Option<String> },
}

pub(super) trait AuthEvents {
    fn authenticated(&self, status: &SessionStatus);
}

pub(super) async fn request_phone_code<A: PhoneRequestApi, E: AuthEvents>(
    state: &AuthState,
    api: &A,
    events: &E,
    phone: String,
) -> AuthResult<CodeRequest> {
    validate_phone(&phone)?;

    let generation = state
        .login
        .lock()
        .await
        .begin_login(LoginStep::RequestingPhone)?;
    state.qr_update.notify_waiters();

    let response = api.request(&phone).await;

    let mut login = state.login.lock().await;
    if !login.is_step(generation, &LoginStep::RequestingPhone) {
        return Err(AuthError::Cancelled);
    }

    let response = match response {
        Ok(response) => response,
        Err(error) => {
            login.step = LoginStep::Idle;
            return Err(error);
        }
    };

    match response {
        PhoneRequestOutcome::Code { delivery, hash } => {
            login.step = LoginStep::Phone { phone, hash };
            Ok(delivery)
        }
        PhoneRequestOutcome::Authorized => {
            drop(login);

            let result = api.status().await;
            let status = complete_authorization(
                state,
                events,
                generation,
                &LoginStep::RequestingPhone,
                result,
            )
            .await?;

            Ok(CodeRequest::Authorized { status })
        }
        PhoneRequestOutcome::EmailSetupRequired => {
            login.step = LoginStep::Idle;
            Err(AuthError::Message(AppMessage::EmailSetupRequired))
        }
        PhoneRequestOutcome::PaymentRequired => {
            login.step = LoginStep::Idle;
            Err(AuthError::Message(AppMessage::AdditionalLoginStep))
        }
    }
}

pub(super) async fn submit_phone_code<A: PhoneSignInApi, E: AuthEvents>(
    state: &AuthState,
    api: &A,
    events: &E,
    code: String,
) -> AuthResult<LoginResult> {
    if code.trim().is_empty() {
        return Err(AuthError::Message(AppMessage::MissingCode));
    }

    let (generation, phone, hash) = {
        let mut login = state.login.lock().await;
        login.take_phone()?
    };

    let result = api.sign_in(&phone, &hash, code.trim()).await;

    if !state
        .login
        .lock()
        .await
        .is_step(generation, &LoginStep::SubmittingCode)
    {
        return Err(AuthError::Cancelled);
    }

    match result {
        Ok(SignInOutcome::Authorized) => {
            finish_sign_in(state, api, events, generation, &LoginStep::SubmittingCode).await
        }
        Ok(SignInOutcome::SignUpRequired) => {
            restore_phone(state, generation, phone, hash).await;
            Err(AuthError::Message(AppMessage::SignUpRequired))
        }
        Err(error) if error.is("SESSION_PASSWORD_NEEDED") => {
            require_password(state, api, generation).await
        }
        Err(error) => {
            let expired = error.is("PHONE_CODE_EXPIRED");
            let error = AuthError::from(error);

            if expired {
                state
                    .login
                    .lock()
                    .await
                    .reset_if(generation, &LoginStep::SubmittingCode);
            } else {
                restore_phone(state, generation, phone, hash).await;
            }

            Err(error)
        }
    }
}

pub(super) async fn submit_password<A: PhoneSignInApi, E: AuthEvents>(
    state: &AuthState,
    api: &A,
    events: &E,
    password: String,
) -> AuthResult<LoginResult> {
    let (generation, token) = {
        let mut login = state.login.lock().await;
        login.take_password()?
    };

    match api.check_password(token, &password).await {
        Ok(_) => {
            finish_sign_in(
                state,
                api,
                events,
                generation,
                &LoginStep::SubmittingPassword,
            )
            .await
        }
        Err(SignInError::InvalidPassword(token)) => {
            let mut login = state.login.lock().await;
            if !login.is_step(generation, &LoginStep::SubmittingPassword) {
                return Err(AuthError::Cancelled);
            }

            login.step = LoginStep::Password(Box::new(token));
            Err(AuthError::Message(AppMessage::IncorrectPassword))
        }
        Err(error) => {
            let current = state
                .login
                .lock()
                .await
                .reset_if(generation, &LoginStep::SubmittingPassword);

            if !current {
                return Err(AuthError::Cancelled);
            }

            Err(match error {
                SignInError::Other(reason) => AuthError::from(reason),
                _ => AuthError::Message(AppMessage::PasswordVerificationFailed),
            })
        }
    }
}

async fn restore_phone(state: &AuthState, generation: u64, phone: String, hash: String) {
    let mut login = state.login.lock().await;

    if login.is_step(generation, &LoginStep::SubmittingCode) {
        login.step = LoginStep::Phone { phone, hash };
    }
}

async fn finish_sign_in<A: PhoneSignInApi, E: AuthEvents>(
    state: &AuthState,
    api: &A,
    events: &E,
    generation: u64,
    expected: &LoginStep,
) -> AuthResult<LoginResult> {
    let result = api.status().await;
    let status = complete_authorization(state, events, generation, expected, result).await?;

    Ok(LoginResult::Authorized { status })
}

async fn complete_authorization<E: AuthEvents>(
    state: &AuthState,
    events: &E,
    generation: u64,
    expected: &LoginStep,
    result: AuthResult<SessionStatus>,
) -> AuthResult<SessionStatus> {
    let mut login = state.login.lock().await;

    if !login.reset_if(generation, expected) {
        return Err(AuthError::Cancelled);
    }
    drop(login);

    let status = result?;
    if !status.authorized() {
        return Err(AuthError::Message(AppMessage::LoginIncomplete));
    }

    events.authenticated(&status);
    Ok(status)
}

async fn require_password<A: PhoneSignInApi>(
    state: &AuthState,
    api: &A,
    generation: u64,
) -> AuthResult<LoginResult> {
    let token = match api.password_token().await {
        Ok(token) => token,
        Err(error) => {
            let current = state
                .login
                .lock()
                .await
                .reset_if(generation, &LoginStep::SubmittingCode);

            if !current {
                return Err(AuthError::Cancelled);
            }

            return Err(error.into());
        }
    };

    let hint = token.hint().map(str::to_owned);
    let mut login = state.login.lock().await;
    if !login.is_step(generation, &LoginStep::SubmittingCode) {
        return Err(AuthError::Cancelled);
    }

    login.step = LoginStep::Password(Box::new(token));
    Ok(LoginResult::PasswordRequired { hint })
}

pub(super) fn validate_phone(phone: &str) -> AuthResult<()> {
    if phone.starts_with('+')
        && (8..=16).contains(&phone.len())
        && phone[1..].bytes().all(|byte| byte.is_ascii_digit())
    {
        Ok(())
    } else {
        Err(AuthError::Message(AppMessage::InternationalPhoneRequired))
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
