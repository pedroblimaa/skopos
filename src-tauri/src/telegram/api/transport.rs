//! The only boundary that substitutes calls to Telegram in tests.
use grammers_client::{client::PasswordToken, tl, Client, InvocationError, SignInError};

#[derive(Clone)]
pub(in crate::telegram) struct TelegramApi {
    pub(in crate::telegram) client: Client,
    pub(in crate::telegram) session: std::sync::Arc<crate::telegram::client::SessionCache>,
    pub(in crate::telegram) app: Option<tauri::AppHandle>,
    #[cfg(any(test, feature = "e2e"))]
    pub(in crate::telegram) fixture: Option<std::sync::Arc<crate::telegram::e2e::FixtureState>>,
}

impl TelegramApi {
    pub(in crate::telegram) async fn save_message(
        &self,
        caption: &str,
        photo: Option<Vec<u8>>,
    ) -> Result<(), crate::app_message::AppMessage> {
        let generation = self.session.snapshot().generation;
        let result = self.send_message(caption, photo).await;

        if result == Err(crate::app_message::AppMessage::RestartLogin) {
            self.reject_session(generation).await;
        }
        result
    }

    async fn send_message(
        &self,
        caption: &str,
        photo: Option<Vec<u8>>,
    ) -> Result<(), crate::app_message::AppMessage> {
        #[cfg(any(test, feature = "e2e"))]
        if let Some(fixture) = &self.fixture {
            return fixture
                .save_message(caption, photo.is_some())
                .map_err(super::super::saved::delivery_error);
        }

        let mut message = grammers_client::message::InputMessage::new()
            .html(caption)
            .link_preview(false);

        if let Some(bytes) = photo {
            let mut stream = bytes.as_slice();
            let upload = self
                .client
                .upload_stream(&mut stream, bytes.len(), "promotion.jpg".into())
                .await
                .map_err(super::super::saved::upload_error)?;
            message = message.photo(upload);
        }

        self.client
            .send_message(
                grammers_session::types::PeerId::self_user().to_ambient_ref(),
                message,
            )
            .await
            .map(|_| ())
            .map_err(super::super::saved::delivery_error)
    }

    pub(in crate::telegram) async fn chat_photo(
        &self,
        location: tl::enums::InputFileLocation,
    ) -> Result<Vec<u8>, InvocationError> {
        let photo = grammers_client::media::ChatPhoto {
            raw: location.clone(),
        };
        // grammers handles Telegram file migration and authorization on the photo's data center.
        let mut download = self.client.iter_download(&photo).chunk_size(64 * 1024);
        let mut bytes = Vec::new();

        loop {
            #[cfg(any(test, feature = "e2e"))]
            let chunk = if self.fixture.is_some() {
                let file = self
                    .invoke(&tl::functions::upload::GetFile {
                        precise: false,
                        cdn_supported: false,
                        location: location.clone(),
                        offset: bytes.len() as i64,
                        limit: 64 * 1024,
                    })
                    .await?;
                match file {
                    tl::enums::upload::File::File(file) => Some(file.bytes),
                    _ => return Err(InvocationError::Dropped),
                }
            } else {
                download.next().await?
            };
            #[cfg(not(any(test, feature = "e2e")))]
            let chunk = download.next().await?;

            let Some(chunk) = chunk else {
                return Ok(bytes);
            };

            if bytes.len() + chunk.len() > 256 * 1024 {
                return Err(InvocationError::Dropped);
            }

            let complete = chunk.len() < 64 * 1024;
            bytes.extend(chunk);

            if complete {
                return Ok(bytes);
            }
        }
    }

    pub(in crate::telegram) async fn invoke<R: tl::RemoteCall>(
        &self,
        request: &R,
    ) -> Result<R::Return, InvocationError> {
        let generation = self.session.snapshot().generation;
        #[cfg(any(test, feature = "e2e"))]
        let result = match &self.fixture {
            Some(fixture) => fixture.invoke(request),
            None => self.client.invoke(request).await,
        };
        #[cfg(not(any(test, feature = "e2e")))]
        let result = self.client.invoke(request).await;
        self.observe(generation, &result).await;
        result
    }

    pub(in crate::telegram) async fn invoke_in_dc<R: tl::RemoteCall>(
        &self,
        dc: i32,
        request: &R,
    ) -> Result<R::Return, InvocationError> {
        let generation = self.session.snapshot().generation;
        #[cfg(any(test, feature = "e2e"))]
        let result = match &self.fixture {
            Some(fixture) => fixture.invoke(request),
            None => self.client.invoke_in_dc(dc, request).await,
        };
        #[cfg(not(any(test, feature = "e2e")))]
        let result = self.client.invoke_in_dc(dc, request).await;
        self.observe(generation, &result).await;
        result
    }

    pub(in crate::telegram) async fn is_authorized(&self) -> Result<bool, InvocationError> {
        #[cfg(any(test, feature = "e2e"))]
        if let Some(fixture) = &self.fixture {
            return fixture.is_authorized();
        }

        self.client.is_authorized().await
    }

    pub(in crate::telegram) async fn names(
        &self,
    ) -> Result<(Option<String>, Option<String>), InvocationError> {
        #[cfg(any(test, feature = "e2e"))]
        if self.fixture.is_some() {
            return Ok((Some("Pedro".into()), None));
        }

        let generation = self.session.snapshot().generation;
        let result = self.client.get_me().await;
        self.observe(generation, &result).await;
        let user = result?;

        Ok((
            user.first_name().map(str::to_owned),
            user.last_name().map(str::to_owned),
        ))
    }

    pub(in crate::telegram) async fn password_token(
        &self,
    ) -> Result<PasswordToken, InvocationError> {
        let password = self.invoke(&tl::functions::account::GetPassword {}).await?;

        Ok(PasswordToken::new(password.into()))
    }

    pub(in crate::telegram) async fn check_password(
        &self,
        token: PasswordToken,
        password: &[u8],
    ) -> Result<(), Box<SignInError>> {
        #[cfg(any(test, feature = "e2e"))]
        if let Some(fixture) = &self.fixture {
            return fixture.check_password(token, password);
        }

        self.client
            .check_password(token, password)
            .await
            .map(|_| ())
            .map_err(Box::new)
    }

    pub(in crate::telegram) async fn sign_out(&self) -> Result<(), InvocationError> {
        #[cfg(any(test, feature = "e2e"))]
        if let Some(fixture) = &self.fixture {
            return fixture.sign_out();
        }

        self.client.sign_out().await.map(|_| ())
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
