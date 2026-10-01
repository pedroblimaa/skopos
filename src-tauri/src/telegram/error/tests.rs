use crate::app_message::AppMessage;
use grammers_client::{sender::RpcError, InvocationError};

use super::AuthError;

fn rpc(name: &str, value: Option<u32>) -> AuthError {
    AuthError::from(InvocationError::Rpc(RpcError {
        code: 400,
        name: name.into(),
        value,
        caused_by: None,
    }))
}

#[test]
fn maps_actionable_telegram_errors() {
    let examples = [
        ("PHONE_NUMBER_INVALID", None, AppMessage::InvalidPhone),
        ("PHONE_CODE_EXPIRED", None, AppMessage::CodeExpired),
        ("PHONE_CODE_INVALID", None, AppMessage::InvalidCode),
        ("PHONE_CODE_EMPTY", None, AppMessage::InvalidCode),
        ("PASSWORD_HASH_INVALID", None, AppMessage::IncorrectPassword),
        (
            "FLOOD_WAIT",
            Some(30),
            AppMessage::FloodWaitSeconds { seconds: 30 },
        ),
        ("FLOOD_WAIT", None, AppMessage::FloodWait),
        (
            "UNKNOWN_ERROR",
            None,
            AppMessage::TelegramRejected {
                name: "UNKNOWN_ERROR".into(),
            },
        ),
    ];
    for (name, value, expected) in examples {
        let error = rpc(name, value);

        assert_eq!(error.message(), expected, "{name}");
        assert!(error.is_rpc(name));
        assert!(!error.is_rpc("DIFFERENT_ERROR"));
    }
}

#[test]
fn maps_local_and_transport_errors() {
    assert_eq!(
        AuthError::Message(AppMessage::RestartLogin).message(),
        AppMessage::RestartLogin
    );
    assert_eq!(AuthError::Storage.message(), AppMessage::AuthStorage);
    assert_eq!(AuthError::Cancelled.message(), AppMessage::AuthCancelled);
    assert_eq!(
        AuthError::from(InvocationError::Dropped).message(),
        AppMessage::AuthNetwork
    );
    assert_eq!(
        AuthError::from(InvocationError::Io(std::io::Error::other("offline"))).message(),
        AppMessage::AuthNetwork
    );
    assert_eq!(
        AuthError::from(InvocationError::Session(Box::new(std::io::Error::other(
            "storage"
        ))))
        .message(),
        AppMessage::AuthStorage
    );
    assert_eq!(
        AuthError::from(InvocationError::InvalidDc).message(),
        AppMessage::AuthLoginFailed
    );
}
