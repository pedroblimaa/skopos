//! The only boundary that substitutes calls to Telegram in tests.
use grammers_client::{client::PasswordToken, tl, Client, InvocationError, SignInError};

#[derive(Clone)]
pub(in crate::telegram) struct TelegramApi {
    pub(in crate::telegram) client: Client,
    #[cfg(any(test, feature = "e2e"))]
    pub(in crate::telegram) fixture: Option<std::sync::Arc<crate::telegram::e2e::FixtureState>>,
}

impl TelegramApi {
    pub(in crate::telegram) async fn invoke<R: tl::RemoteCall>(
        &self,
        request: &R,
    ) -> Result<R::Return, InvocationError> {
        #[cfg(any(test, feature = "e2e"))]
        if let Some(fixture) = &self.fixture {
            return fixture.invoke(request);
        }

        self.client.invoke(request).await
    }

    pub(in crate::telegram) async fn invoke_in_dc<R: tl::RemoteCall>(
        &self,
        dc: i32,
        request: &R,
    ) -> Result<R::Return, InvocationError> {
        #[cfg(any(test, feature = "e2e"))]
        if let Some(fixture) = &self.fixture {
            return fixture.invoke(request);
        }

        self.client.invoke_in_dc(dc, request).await
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

        let user = self.client.get_me().await?;

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
