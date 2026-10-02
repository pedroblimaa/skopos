use super::*;
use crate::telegram::e2e::{chats::account, test_context};
use grammers_client::InvocationError;
use tl::Serializable;

#[tokio::test]
async fn downloads_the_current_accounts_profile_photo() {
    let (context, fixture) = test_context().await;
    {
        let mut fixture = fixture.0.lock().unwrap();
        fixture.scenario.authorized = true;
        fixture.scenario.profile_photo = true;
    }

    let photo = load_photo(&context.client).await.unwrap().unwrap();

    assert!(photo.starts_with("data:image/jpeg;base64,/9j/"));
}

#[tokio::test]
async fn returns_none_for_missing_or_empty_photos() {
    let (context, fixture) = test_context().await;

    for photo in [None, Some(tl::enums::UserProfilePhoto::Empty)] {
        let mut user = account(1);
        user.photo = photo;
        fixture
            .0
            .lock()
            .unwrap()
            .replies
            .push_back(Ok(vec![tl::enums::User::from(user)].to_bytes()));

        assert!(load_photo(&context.client).await.unwrap().is_none());
    }
}

#[tokio::test]
async fn rejects_missing_or_foreign_account_responses() {
    let (context, fixture) = test_context().await;
    let mut other = account(2);
    other.is_self = false;

    let responses: [Vec<tl::enums::User>; 3] = [
        vec![],
        vec![other.into()],
        vec![tl::types::UserEmpty { id: 1 }.into()],
    ];

    for users in responses {
        fixture
            .0
            .lock()
            .unwrap()
            .replies
            .push_back(Ok(users.to_bytes()));

        assert!(matches!(
            load_photo(&context.client).await,
            Err(AuthError::Message(AppMessage::RestartLogin))
        ));
    }
}

#[tokio::test]
async fn preserves_authentication_and_download_errors() {
    let (context, fixture) = test_context().await;

    assert!(matches!(load_photo(&context.client).await,
        Err(AuthError::Telegram(InvocationError::Rpc(error))) if error.code == 401));

    {
        let mut fixture = fixture.0.lock().unwrap();
        fixture.scenario.authorized = true;
        fixture.scenario.profile_photo = true;
        fixture.scenario.photo_error = true;
    }

    assert!(matches!(
        load_photo(&context.client).await,
        Err(AuthError::Telegram(InvocationError::Dropped))
    ));
}

#[tokio::test]
async fn rejects_invalid_image_data_and_failed_account_retrieval() {
    let (context, fixture) = test_context().await;
    let mut user = account(1);
    user.photo = Some(
        tl::types::UserProfilePhoto {
            has_video: false,
            personal: false,
            photo_id: 42,
            stripped_thumb: None,
            dc_id: 2,
        }
        .into(),
    );
    let file: tl::enums::upload::File = tl::types::upload::File {
        r#type: tl::enums::storage::FileType::FileJpeg,
        mtime: 0,
        bytes: vec![0, 1],
    }
    .into();
    fixture.0.lock().unwrap().replies.extend([
        Ok(vec![tl::enums::User::from(user)].to_bytes()),
        Ok(file.to_bytes()),
    ]);

    assert!(matches!(
        load_photo(&context.client).await,
        Err(AuthError::Message(AppMessage::AuthLoginFailed))
    ));

    fixture
        .0
        .lock()
        .unwrap()
        .replies
        .push_back(Err(InvocationError::Dropped));

    assert!(matches!(
        load_photo(&context.client).await,
        Err(AuthError::Telegram(InvocationError::Dropped))
    ));
}
