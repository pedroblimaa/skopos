use crate::telegram::api::TelegramApi;
use base64::{engine::general_purpose::STANDARD, Engine};
use grammers_client::{
    media::{Downloadable, Photo},
    tl,
};

pub(super) async fn download(
    api: &TelegramApi,
    media: Option<tl::enums::MessageMedia>,
) -> Option<String> {
    let Some(tl::enums::MessageMedia::Photo(media)) = media else {
        return None;
    };
    let photo = Photo::from_raw_media(media);
    let thumb = photo
        .thumbs()
        .into_iter()
        .filter(|thumb| thumb.size() > 0 && thumb.size() <= 256 * 1024)
        .max_by_key(|thumb| thumb.size())?;
    let bytes = if let Some(bytes) = thumb.to_data() {
        bytes
    } else {
        let location = thumb.to_raw_input_location()?;
        tokio::time::timeout(std::time::Duration::from_secs(10), api.chat_photo(location))
            .await
            .ok()?
            .ok()?
    };
    if !bytes.starts_with(&[0xff, 0xd8]) {
        return None;
    }

    Some(format!("data:image/jpeg;base64,{}", STANDARD.encode(bytes)))
}
