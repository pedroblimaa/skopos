use super::AuthState;
use crate::app_message::AppMessage;
use base64::{engine::general_purpose::STANDARD, Engine};
use grammers_client::InvocationError;
use tauri::{AppHandle, Manager};

pub(crate) async fn send(
    app: &AppHandle,
    account: i64,
    caption: String,
    image: Option<&str>,
) -> Result<(), AppMessage> {
    let auth = app.state::<AuthState>();
    if auth.account_id(app).await? != account {
        return Err(AppMessage::RestartLogin);
    }

    let context = auth.client(app).await.map_err(|error| error.message())?;
    let photo = image
        .and_then(|image| image.strip_prefix("data:image/jpeg;base64,"))
        .and_then(|image| STANDARD.decode(image).ok())
        .filter(|bytes| bytes.len() <= 256 * 1024 && bytes.starts_with(&[0xff, 0xd8]));

    let result = context.client.save_message(&caption, photo).await;
    // A rejected image is safe to replace; an ambiguous send may already be saved.
    if result == Err(AppMessage::NotificationPhoto) {
        return context.client.save_message(&caption, None).await;
    }

    result
}

pub(super) fn delivery_error(error: InvocationError) -> AppMessage {
    match error {
        InvocationError::Rpc(error) if error.code >= 500 => AppMessage::NotificationUncertain,
        InvocationError::Rpc(error) => match error.name.as_str() {
            "FLOOD_WAIT" => AppMessage::NotificationRateLimit {
                seconds: error.value.map_or(60, u64::from),
            },
            "AUTH_KEY_UNREGISTERED" | "SESSION_REVOKED" | "SESSION_EXPIRED" => {
                AppMessage::RestartLogin
            }
            "PHOTO_INVALID" | "PHOTO_INVALID_DIMENSIONS" | "IMAGE_PROCESS_FAILED" => {
                AppMessage::NotificationPhoto
            }
            _ => AppMessage::NotificationFailed,
        },
        InvocationError::Session(_) => AppMessage::AuthStorage,
        _ => AppMessage::NotificationUncertain,
    }
}

pub(super) fn upload_error(error: std::io::Error) -> AppMessage {
    // Upload failures precede message delivery, so retrying cannot duplicate a save.
    if let Some(InvocationError::Rpc(error)) = error
        .get_ref()
        .and_then(|error| error.downcast_ref::<InvocationError>())
    {
        if error.name == "FLOOD_WAIT" {
            return AppMessage::NotificationRateLimit {
                seconds: error.value.map_or(60, u64::from),
            };
        }
    }

    AppMessage::NotificationFailed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telegram::e2e::test_context;
    use grammers_client::sender::RpcError;

    #[tokio::test]
    async fn telegram_boundary_saves_text_and_media_and_reports_rejections() {
        let (context, fixture) = test_context().await;
        context.client.save_message("Offer", None).await.unwrap();
        context
            .client
            .save_message("Photo offer", Some(vec![0xff, 0xd8]))
            .await
            .unwrap();

        for (name, expected) in [
            (
                "FLOOD_WAIT",
                AppMessage::NotificationRateLimit { seconds: 60 },
            ),
            ("SESSION_REVOKED", AppMessage::RestartLogin),
            ("PHOTO_INVALID", AppMessage::NotificationPhoto),
            ("DROPPED", AppMessage::NotificationUncertain),
            ("MESSAGE_EMPTY", AppMessage::NotificationFailed),
        ] {
            fixture.0.lock().unwrap().scenario.saved_message_error = Some(name.into());

            assert_eq!(
                context
                    .client
                    .save_message("Offer", Some(vec![0xff, 0xd8]))
                    .await,
                Err(expected)
            );
        }
    }

    #[test]
    fn rpc_errors_preserve_cooldowns_and_uncertain_server_errors() {
        let error = |code, name: &str, value| {
            InvocationError::Rpc(RpcError {
                code,
                name: name.into(),
                value,
                caused_by: None,
            })
        };

        assert_eq!(
            delivery_error(error(420, "FLOOD_WAIT", Some(30))),
            AppMessage::NotificationRateLimit { seconds: 30 }
        );
        assert_eq!(
            delivery_error(error(500, "INTERNAL", None)),
            AppMessage::NotificationUncertain
        );
        assert_eq!(
            delivery_error(InvocationError::Session(Box::new(std::io::Error::other(
                "session unavailable"
            )))),
            AppMessage::AuthStorage
        );
        for name in ["AUTH_KEY_UNREGISTERED", "SESSION_EXPIRED"] {
            assert_eq!(
                delivery_error(error(401, name, None)),
                AppMessage::RestartLogin
            );
        }
        for name in ["PHOTO_INVALID_DIMENSIONS", "IMAGE_PROCESS_FAILED"] {
            assert_eq!(
                delivery_error(error(400, name, None)),
                AppMessage::NotificationPhoto
            );
        }

        assert_eq!(
            upload_error(std::io::Error::other("upload failed")),
            AppMessage::NotificationFailed
        );
        assert_eq!(
            upload_error(std::io::Error::other(error(420, "FLOOD_WAIT", Some(45)))),
            AppMessage::NotificationRateLimit { seconds: 45 }
        );
        assert_eq!(
            upload_error(std::io::Error::other(error(400, "PHOTO_INVALID", None))),
            AppMessage::NotificationFailed
        );
    }
}
