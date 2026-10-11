use super::*;
use crate::telegram::e2e::{password, test_context};

#[tokio::test]
async fn live_api_reports_a_stopped_sender_without_attempting_network_io() {
    // test_context drops the sender runner. The real grammers client must
    // propagate that failure rather than hanging or fabricating auth success.
    let (mut context, _) = test_context().await;
    context.client.fixture = None;
    let api = context.client;

    assert!(matches!(
        api.invoke(&tl::functions::account::GetPassword {}).await,
        Err(InvocationError::Dropped)
    ));
    assert!(matches!(
        api.invoke_in_dc(4, &tl::functions::account::GetPassword {})
            .await,
        Err(InvocationError::Dropped)
    ));
    assert!(matches!(
        api.is_authorized().await,
        Err(InvocationError::Dropped)
    ));
    assert!(matches!(api.names().await, Err(InvocationError::Dropped)));
    assert_eq!(
        api.save_message("Promotion", None).await,
        Err(crate::app_message::AppMessage::NotificationUncertain)
    );
    assert_eq!(
        api.save_message("Promotion", Some(vec![0xff, 0xd8])).await,
        Err(crate::app_message::AppMessage::NotificationFailed)
    );
    assert!(matches!(
        api.sign_out().await,
        Err(InvocationError::Dropped)
    ));

    let photo = tl::types::InputPeerPhotoFileLocation {
        big: false,
        peer: tl::enums::InputPeer::PeerSelf,
        photo_id: 1,
    }
    .into();

    assert!(matches!(
        api.chat_photo(photo).await,
        Err(InvocationError::Dropped)
    ));

    let mut info = password();
    info.current_algo = Some(
        tl::types::PasswordKdfAlgoSha256Sha256Pbkdf2Hmacsha512iter100000Sha256ModPow {
            salt1: vec![],
            salt2: vec![],
            g: 2,
            p: vec![],
        }
        .into(),
    );

    assert!(matches!(
        api.check_password(PasswordToken::new(info), b"secret")
            .await
            .err()
            .as_deref(),
        Some(SignInError::Other(InvocationError::Dropped))
    ));
}

#[tokio::test]
async fn authorization_rejection_revokes_the_cached_session_but_password_challenges_do_not() {
    for (code, name, expected_authorized) in [
        (401, "AUTH_KEY_UNREGISTERED", false),
        (401, "SESSION_EXPIRED", false),
        (401, "SESSION_PASSWORD_NEEDED", true),
        (400, "MESSAGE_EMPTY", true),
    ] {
        let (context, fixture) = test_context().await;
        context
            .client
            .session
            .reset(Some(crate::telegram::client::SessionStatus::signed_in(
                "Pedro".into(),
            )));
        fixture
            .0
            .lock()
            .unwrap()
            .replies
            .push_back(Err(InvocationError::Rpc(
                grammers_client::sender::RpcError {
                    code,
                    name: name.into(),
                    value: None,
                    caused_by: None,
                },
            )));

        assert!(context
            .client
            .invoke(&tl::functions::account::GetPassword {})
            .await
            .is_err());

        assert_eq!(
            context
                .client
                .session
                .snapshot()
                .status
                .unwrap()
                .authorized(),
            expected_authorized
        );
    }
}

#[tokio::test]
async fn an_old_rpc_rejection_cannot_revoke_a_newer_login_and_network_failures_keep_it() {
    let (context, _) = test_context().await;
    context
        .client
        .session
        .reset(Some(crate::telegram::client::SessionStatus::signed_in(
            "First".into(),
        )));
    let old_generation = context.client.session.snapshot().generation;
    context
        .client
        .session
        .reset(Some(crate::telegram::client::SessionStatus::signed_in(
            "Second".into(),
        )));
    let rejected: Result<(), InvocationError> =
        Err(InvocationError::Rpc(grammers_client::sender::RpcError {
            code: 401,
            name: "SESSION_EXPIRED".into(),
            value: None,
            caused_by: None,
        }));

    context.client.observe(old_generation, &rejected).await;
    let current = context.client.session.snapshot().generation;
    context
        .client
        .observe::<()>(current, &Err(InvocationError::Dropped))
        .await;

    assert!(context
        .client
        .session
        .snapshot()
        .status
        .unwrap()
        .authorized());
    assert_eq!(context.client.session.snapshot().generation, current);
}
