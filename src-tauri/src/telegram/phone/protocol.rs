use grammers_client::{tl, InvocationError};
use grammers_session::Session;
use serde::Serialize;

use crate::telegram::{
    client::{credentials, ClientContext},
    error::{AuthError, AuthResult},
};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeRequest {
    message: String,
    length: Option<i32>,
}

impl CodeRequest {
    pub(super) fn authorized() -> Self {
        Self {
            message: "Telegram authorized your session.".into(),
            length: None,
        }
    }
}

pub(super) fn code_delivery(kind: tl::enums::auth::SentCodeType) -> CodeRequest {
    use tl::enums::auth::SentCodeType;
    let (message, length) = match kind {
        SentCodeType::App(code) => (
            "Enter the code sent to your Telegram app.".into(),
            Some(code.length),
        ),
        SentCodeType::Sms(code) => ("Enter the code sent by SMS.".into(), Some(code.length)),
        SentCodeType::Call(code) => (
            "Enter the code from Telegram's phone call.".into(),
            Some(code.length),
        ),
        SentCodeType::EmailCode(code) => (
            format!("Enter the code sent to {}.", code.email_pattern),
            Some(code.length),
        ),
        SentCodeType::FragmentSms(code) => (
            format!("Enter the code from {}.", code.url),
            Some(code.length),
        ),
        SentCodeType::MissedCall(code) => (
            format!(
                "Enter the code from the missed call beginning with {}.",
                code.prefix
            ),
            Some(code.length),
        ),
        SentCodeType::FirebaseSms(code) => {
            ("Enter the code sent by SMS.".into(), Some(code.length))
        }
        SentCodeType::FlashCall(_) => (
            "Telegram is calling your phone. Follow its instructions to finish login.".into(),
            None,
        ),
        SentCodeType::SetUpEmailRequired(_) => (
            "Telegram requires email setup. Complete it in an official Telegram app first.".into(),
            None,
        ),
        SentCodeType::SmsWord(_) | SentCodeType::SmsPhrase(_) => {
            ("Enter the code or phrase sent by SMS.".into(), None)
        }
    };
    CodeRequest { message, length }
}

pub(super) async fn send_code(
    context: &ClientContext,
    phone: &str,
) -> AuthResult<tl::enums::auth::SentCode> {
    let (api_id, api_hash) = credentials()?;
    let request = tl::functions::auth::SendCode {
        phone_number: phone.to_owned(),
        api_id,
        api_hash: api_hash.to_owned(),
        settings: tl::types::CodeSettings {
            allow_flashcall: false,
            current_number: false,
            allow_app_hash: false,
            allow_missed_call: false,
            allow_firebase: false,
            logout_tokens: None,
            token: None,
            app_sandbox: None,
            unknown_number: false,
        }
        .into(),
    };
    match context.client.invoke(&request).await {
        Err(InvocationError::Rpc(error)) if error.code == 303 => {
            let dc = error.value.ok_or(AuthError::Message(
                "Telegram did not provide a new data center.",
            ))? as i32;
            context
                .session
                .set_home_dc_id(dc)
                .await
                .map_err(|_| AuthError::Storage)?;
            Ok(context.client.invoke(&request).await?)
        }
        result => Ok(result?),
    }
}
