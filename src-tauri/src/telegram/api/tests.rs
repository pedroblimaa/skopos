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
