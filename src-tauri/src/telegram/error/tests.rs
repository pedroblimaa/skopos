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
        ("PHONE_NUMBER_INVALID", None, "valid phone number"),
        ("PHONE_CODE_EXPIRED", None, "expired"),
        ("PHONE_CODE_INVALID", None, "invalid"),
        ("PHONE_CODE_EMPTY", None, "invalid"),
        ("PASSWORD_HASH_INVALID", None, "incorrect"),
        ("FLOOD_WAIT", Some(30), "30 seconds"),
        ("FLOOD_WAIT", None, "Wait before trying again"),
        ("UNKNOWN_ERROR", None, "UNKNOWN_ERROR"),
    ];
    for (name, value, expected) in examples {
        let error = rpc(name, value);

        assert!(error.message().contains(expected), "{name}");
        assert!(error.is_rpc(name));
        assert!(!error.is_rpc("DIFFERENT_ERROR"));
    }
}

#[test]
fn maps_local_and_transport_errors() {
    assert_eq!(AuthError::Message("retry").message(), "retry");
    assert!(AuthError::Storage
        .message()
        .contains("local Telegram session"));
    assert!(AuthError::Cancelled.message().contains("replaced"));
    assert!(AuthError::from(InvocationError::Dropped)
        .message()
        .contains("reach Telegram"));
    assert!(
        AuthError::from(InvocationError::Io(std::io::Error::other("offline")))
            .message()
            .contains("reach Telegram")
    );
    assert!(
        AuthError::from(InvocationError::Session(Box::new(std::io::Error::other(
            "storage"
        ))))
        .message()
        .contains("local Telegram session")
    );
    assert!(AuthError::from(InvocationError::InvalidDc)
        .message()
        .contains("could not complete login"));
}
