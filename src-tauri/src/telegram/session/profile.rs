use crate::app_message::AppMessage;
use crate::telegram::{
    api::TelegramApi,
    error::{AuthError, AuthResult},
};
use base64::{engine::general_purpose::STANDARD, Engine};
use grammers_client::tl;

pub(super) async fn load_photo(api: &TelegramApi) -> AuthResult<Option<String>> {
    let users = api
        .invoke(&tl::functions::users::GetUsers {
            id: vec![tl::enums::InputUser::UserSelf],
        })
        .await?;
    let user = users
        .into_iter()
        .find_map(|user| match user {
            tl::enums::User::User(user) if user.is_self => Some(user),
            _ => None,
        })
        .ok_or(AuthError::Message(AppMessage::RestartLogin))?;
    let Some(tl::enums::UserProfilePhoto::Photo(photo)) = user.photo else {
        return Ok(None);
    };

    let location = tl::types::InputPeerPhotoFileLocation {
        big: false,
        peer: tl::enums::InputPeer::PeerSelf,
        photo_id: photo.photo_id,
    }
    .into();
    let bytes = api.chat_photo(location).await?;
    if !bytes.starts_with(&[0xff, 0xd8]) {
        return Err(AuthError::Message(AppMessage::AuthLoginFailed));
    }

    Ok(Some(format!(
        "data:image/jpeg;base64,{}",
        STANDARD.encode(bytes)
    )))
}

#[cfg(test)]
#[path = "profile_tests.rs"]
mod tests;
