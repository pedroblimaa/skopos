mod protocol;

use grammers_client::{client::PasswordToken, tl, SignInError};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

use super::{
    client::{status_for, ClientContext, SessionStatus},
    error::{AuthError, AuthResult},
    state::{AuthState, LoginStep},
};

pub use protocol::CodeRequest;
use protocol::{code_delivery, send_code};

#[derive(Serialize)]
#[serde(tag = "step", rename_all = "camelCase")]
pub enum LoginResult {
    Authorized { status: SessionStatus },
    PasswordRequired { hint: Option<String> },
}

fn validate_phone(phone: &str) -> AuthResult<()> {
    if phone.starts_with('+')
        && (8..=16).contains(&phone.len())
        && phone[1..].bytes().all(|byte| byte.is_ascii_digit())
    {
        Ok(())
    } else {
        Err(AuthError::Message(
            "Enter a phone number in international format, such as +5511999999999.",
        ))
    }
}

#[tauri::command]
pub async fn request_phone_code(
    app: AppHandle,
    state: State<'_, AuthState>,
    phone: String,
) -> Result<CodeRequest, String> {
    validate_phone(&phone).map_err(|error| error.message())?;
    let context = state.client(&app).await.map_err(|error| error.message())?;
    let generation = state
        .login
        .lock()
        .await
        .begin_login(LoginStep::RequestingPhone)
        .map_err(|error| error.message())?;
    state.qr_update.notify_waiters();

    let response = send_code(context, &phone).await;
    let mut login = state.login.lock().await;
    if !login.is_step(generation, &LoginStep::RequestingPhone) {
        return Err(AuthError::Cancelled.message());
    }
    let response = match response {
        Ok(response) => response,
        Err(error) => {
            login.step = LoginStep::Idle;
            return Err(error.message());
        }
    };
    match response {
        tl::enums::auth::SentCode::Code(sent) => {
            if matches!(
                sent.r#type,
                tl::enums::auth::SentCodeType::SetUpEmailRequired(_)
            ) {
                login.step = LoginStep::Idle;
                return Err(
                    "Telegram requires email setup. Complete it in an official Telegram app first."
                        .into(),
                );
            }
            let delivery = code_delivery(sent.r#type);
            login.step = LoginStep::Phone {
                phone,
                hash: sent.phone_code_hash,
            };
            Ok(delivery)
        }
        tl::enums::auth::SentCode::Success(_) => {
            drop(login);
            let result = status_for(&context.client).await;
            let mut login = state.login.lock().await;
            if !login.reset_if(generation, &LoginStep::RequestingPhone) {
                return Err(AuthError::Cancelled.message());
            }
            drop(login);
            let status = result.map_err(|error| error.message())?;
            if !status.authorized() {
                return Err("Telegram did not finish login. Start again.".into());
            }
            let _ = app.emit("telegram:auth-changed", status);
            Ok(CodeRequest::authorized())
        }
        tl::enums::auth::SentCode::PaymentRequired(_) => {
            login.step = LoginStep::Idle;
            Err("Telegram requires an additional login step that Skopos cannot complete.".into())
        }
    }
}

#[tauri::command]
pub async fn submit_phone_code(
    app: AppHandle,
    state: State<'_, AuthState>,
    code: String,
) -> Result<LoginResult, String> {
    if code.trim().is_empty() {
        return Err("Enter the verification code.".into());
    }
    let context = state.client(&app).await.map_err(|error| error.message())?;
    let (generation, phone, hash) = {
        let mut login = state.login.lock().await;
        login.take_phone().map_err(|error| error.message())?
    };

    let result = context
        .client
        .invoke(&tl::functions::auth::SignIn {
            phone_number: phone.clone(),
            phone_code_hash: hash.clone(),
            phone_code: Some(code.trim().to_owned()),
            email_verification: None,
        })
        .await;

    if !state
        .login
        .lock()
        .await
        .is_step(generation, &LoginStep::SubmittingCode)
    {
        return Err(AuthError::Cancelled.message());
    }
    match result {
        Ok(tl::enums::auth::Authorization::Authorization(_)) => {
            finish_sign_in(
                &app,
                &state,
                context,
                generation,
                &LoginStep::SubmittingCode,
            )
            .await
        }
        Ok(tl::enums::auth::Authorization::SignUpRequired(_)) => {
            restore_phone(&state, generation, phone, hash).await;
            Err("This number needs a Telegram account. Sign up in the official app first.".into())
        }
        Err(error) if error.is("SESSION_PASSWORD_NEEDED") => {
            let password_result = context
                .client
                .invoke(&tl::functions::account::GetPassword {})
                .await;
            let password = match password_result {
                Ok(password) => password,
                Err(error) => {
                    state
                        .login
                        .lock()
                        .await
                        .reset_if(generation, &LoginStep::SubmittingCode);
                    return Err(AuthError::from(error).message());
                }
            };
            let token = PasswordToken::new(password.into());
            let hint = token.hint().map(str::to_owned);
            let mut login = state.login.lock().await;
            if !login.is_step(generation, &LoginStep::SubmittingCode) {
                return Err(AuthError::Cancelled.message());
            }
            login.step = LoginStep::Password(Box::new(token));
            Ok(LoginResult::PasswordRequired { hint })
        }
        Err(error) => {
            let expired = error.is("PHONE_CODE_EXPIRED");
            let message = if error.is("PHONE_CODE_INVALID") || error.is("PHONE_CODE_EMPTY") {
                "That verification code is invalid. Check it and try again.".into()
            } else {
                AuthError::from(error).message()
            };
            if expired {
                state
                    .login
                    .lock()
                    .await
                    .reset_if(generation, &LoginStep::SubmittingCode);
            } else {
                restore_phone(&state, generation, phone, hash).await;
            }
            Err(message)
        }
    }
}

async fn restore_phone(state: &AuthState, generation: u64, phone: String, hash: String) {
    let mut login = state.login.lock().await;
    if login.is_step(generation, &LoginStep::SubmittingCode) {
        login.step = LoginStep::Phone { phone, hash };
    }
}

async fn finish_sign_in(
    app: &AppHandle,
    state: &AuthState,
    context: &ClientContext,
    generation: u64,
    expected: &LoginStep,
) -> Result<LoginResult, String> {
    let result = status_for(&context.client).await;
    let mut login = state.login.lock().await;
    if !login.reset_if(generation, expected) {
        return Err(AuthError::Cancelled.message());
    }
    drop(login);
    let status = result.map_err(|error| error.message())?;
    if !status.authorized() {
        return Err("Telegram did not finish login. Start again.".into());
    }
    let _ = app.emit("telegram:auth-changed", &status);
    Ok(LoginResult::Authorized { status })
}

#[tauri::command]
pub async fn submit_password(
    app: AppHandle,
    state: State<'_, AuthState>,
    password: String,
) -> Result<LoginResult, String> {
    let context = state.client(&app).await.map_err(|error| error.message())?;
    let (generation, token) = {
        let mut login = state.login.lock().await;
        login.take_password().map_err(|error| error.message())?
    };

    match context
        .client
        .check_password(token, password.as_bytes())
        .await
    {
        Ok(_) => {
            finish_sign_in(
                &app,
                &state,
                context,
                generation,
                &LoginStep::SubmittingPassword,
            )
            .await
        }
        Err(SignInError::InvalidPassword(token)) => {
            let mut login = state.login.lock().await;
            if !login.is_step(generation, &LoginStep::SubmittingPassword) {
                return Err(AuthError::Cancelled.message());
            }
            login.step = LoginStep::Password(Box::new(token));
            Err("That password is incorrect. Try again.".into())
        }
        Err(error) => {
            let current = state
                .login
                .lock()
                .await
                .reset_if(generation, &LoginStep::SubmittingPassword);
            if !current {
                return Err(AuthError::Cancelled.message());
            }
            Err(match error {
                SignInError::Other(reason) => AuthError::from(reason).message(),
                _ => "Could not complete two-step verification. Start Telegram login again.".into(),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::validate_phone;

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
}
