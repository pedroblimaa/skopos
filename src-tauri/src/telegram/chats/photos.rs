use super::ChatError;
use crate::telegram::api::TelegramApi;
use base64::{engine::general_purpose::STANDARD, Engine};
use grammers_client::tl;
use std::collections::HashMap;
use tokio::sync::Mutex;

#[derive(Default)]
pub struct ChatPhotos(pub(super) Mutex<Option<AccountPhotos>>);

pub(super) struct AccountPhotos {
    pub account: i64,
    pub locations: HashMap<String, PhotoLocation>,
}

#[derive(Clone)]
pub(super) struct PhotoLocation {
    location: tl::enums::InputFileLocation,
}

pub(super) fn location(chat: &tl::enums::Chat) -> Option<PhotoLocation> {
    let (peer, photo) = match chat {
        tl::enums::Chat::Chat(chat) => (
            tl::types::InputPeerChat { chat_id: chat.id }.into(),
            &chat.photo,
        ),
        tl::enums::Chat::Channel(channel) => (
            tl::types::InputPeerChannel {
                channel_id: channel.id,
                access_hash: channel.access_hash?,
            }
            .into(),
            &channel.photo,
        ),
        _ => return None,
    };
    let tl::enums::ChatPhoto::Photo(photo) = photo else {
        return None;
    };

    Some(PhotoLocation {
        location: tl::types::InputPeerPhotoFileLocation {
            big: false,
            peer,
            photo_id: photo.photo_id,
        }
        .into(),
    })
}

pub(super) async fn download(api: &TelegramApi, photo: PhotoLocation) -> Result<String, ChatError> {
    let bytes = api.chat_photo(photo.location).await?;
    if !bytes.starts_with(&[0xff, 0xd8]) {
        return Err(ChatError::IncompleteList);
    }

    Ok(format!("data:image/jpeg;base64,{}", STANDARD.encode(bytes)))
}
