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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telegram::e2e::{history::photo_media, test_context};

    #[tokio::test]
    async fn handles_cached_photos_downloads_and_missing_media() {
        let (context, fixture) = test_context().await;
        let jpeg = include_bytes!("../e2e/avatar.jpg").to_vec();

        let cached = download(&context.client, Some(photo_media(true, jpeg.clone())))
            .await
            .unwrap();
        assert_eq!(
            cached,
            format!("data:image/jpeg;base64,{}", STANDARD.encode(jpeg))
        );
        assert!(download(&context.client, None).await.is_none());

        fixture.0.lock().unwrap().scenario.promotion_photos = true;
        let downloaded = download(&context.client, Some(photo_media(false, vec![])))
            .await
            .unwrap();
        assert_eq!(cached, downloaded);
    }

    #[tokio::test]
    async fn failed_invalid_and_oversized_photos_are_optional() {
        let (context, fixture) = test_context().await;
        fixture.0.lock().unwrap().scenario.photo_error = true;

        assert!(download(&context.client, Some(photo_media(false, vec![])))
            .await
            .is_none());

        assert!(download(
            &context.client,
            Some(photo_media(true, b"not jpeg".to_vec()))
        )
        .await
        .is_none());

        assert!(download(
            &context.client,
            Some(photo_media(true, vec![0; 256 * 1024 + 1]))
        )
        .await
        .is_none());
    }
}
