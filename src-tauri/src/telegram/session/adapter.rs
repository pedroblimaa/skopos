use crate::telegram::{
    client::{status_for, ClientContext, SessionStatus},
    error::{AuthError, AuthResult},
};

pub(super) trait SessionApi {
    async fn status(&self) -> AuthResult<SessionStatus>;
    async fn sign_out(&self) -> AuthResult<()>;
}

impl SessionApi for ClientContext {
    async fn status(&self) -> AuthResult<SessionStatus> {
        status_for(&self.client).await
    }

    async fn sign_out(&self) -> AuthResult<()> {
        self.client.sign_out().await.map_err(AuthError::from)
    }
}
