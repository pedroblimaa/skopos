use crate::telegram::client::SessionStatus;
use grammers_client::tl;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(tag = "step", rename_all = "camelCase")]
pub enum CodeRequest {
    CodeSent {
        message: String,
        length: Option<i32>,
    },
    Authorized {
        status: SessionStatus,
    },
}

pub(in crate::telegram::phone) fn code_delivery(
    kind: tl::enums::auth::SentCodeType,
) -> CodeRequest {
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

    CodeRequest::CodeSent { message, length }
}
