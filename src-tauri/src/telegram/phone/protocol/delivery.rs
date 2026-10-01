use crate::app_message::AppMessage;
use crate::telegram::client::SessionStatus;
use grammers_client::tl;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(tag = "step", rename_all = "camelCase")]
pub enum CodeRequest {
    CodeSent {
        message: AppMessage,
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
        SentCodeType::App(code) => (AppMessage::DeliveryApp, Some(code.length)),
        SentCodeType::Sms(code) => (AppMessage::DeliverySms, Some(code.length)),
        SentCodeType::Call(code) => (AppMessage::DeliveryCall, Some(code.length)),
        SentCodeType::EmailCode(code) => (
            AppMessage::DeliveryEmail {
                email: code.email_pattern,
            },
            Some(code.length),
        ),
        SentCodeType::FragmentSms(code) => (
            AppMessage::DeliveryFragment { url: code.url },
            Some(code.length),
        ),
        SentCodeType::MissedCall(code) => (
            AppMessage::DeliveryMissedCall {
                prefix: code.prefix,
            },
            Some(code.length),
        ),
        SentCodeType::FirebaseSms(code) => (AppMessage::DeliverySms, Some(code.length)),
        SentCodeType::FlashCall(_) => (AppMessage::DeliveryFlashCall, None),
        SentCodeType::SetUpEmailRequired(_) => (AppMessage::EmailSetupRequired, None),
        SentCodeType::SmsWord(_) | SentCodeType::SmsPhrase(_) => {
            (AppMessage::DeliverySmsPhrase, None)
        }
    };

    CodeRequest::CodeSent { message, length }
}
