use super::protocol::{export_qr, QrOutcome, QrSender};
use crate::telegram::{
    client::{status_for, ClientContext, SessionStatus},
    error::{AuthError, AuthResult},
};
use grammers_client::{client::PasswordToken, tl};
use grammers_session::Session;

pub(super) trait QrApi {
    async fn export(&self) -> AuthResult<QrOutcome>;
    async fn status(&self) -> AuthResult<SessionStatus>;
    async fn password_token(&self) -> Result<PasswordToken, grammers_client::InvocationError>;
}

impl QrApi for ClientContext {
    async fn export(&self) -> AuthResult<QrOutcome> {
        export_qr(self).await
    }

    async fn status(&self) -> AuthResult<SessionStatus> {
        status_for(&self.client).await
    }

    async fn password_token(&self) -> Result<PasswordToken, grammers_client::InvocationError> {
        self.client.password_token().await
    }
}

impl QrSender for ClientContext {
    async fn export(
        &self,
        request: &tl::functions::auth::ExportLoginToken,
    ) -> Result<tl::enums::auth::LoginToken, grammers_client::InvocationError> {
        self.client.invoke(request).await
    }

    async fn import(
        &self,
        dc: i32,
        token: Vec<u8>,
    ) -> Result<tl::enums::auth::LoginToken, grammers_client::InvocationError> {
        self.client
            .invoke_in_dc(dc, &tl::functions::auth::ImportLoginToken { token })
            .await
    }

    async fn set_home_dc(&self, dc: i32) -> AuthResult<()> {
        self.session
            .set_home_dc_id(dc)
            .await
            .map_err(|_| AuthError::Storage)
    }
}
