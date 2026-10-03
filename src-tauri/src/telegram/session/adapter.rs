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
        self.client.sign_out().await.map_err(AuthError::from)?;
        *self.account_id.lock().await = None;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telegram::e2e::test_context;

    #[tokio::test]
    async fn successful_sign_out_clears_identity_and_a_failed_sign_out_preserves_it() {
        let (context, fixture) = test_context().await;
        fixture.0.lock().unwrap().scenario.authorized = true;
        assert_eq!(context.local_account_id().await.unwrap(), 1);
        fixture.0.lock().unwrap().scenario.sign_out_error = Some("network".into());

        assert!(context.sign_out().await.is_err());
        assert_eq!(*context.account_id.lock().await, Some(1));

        fixture.0.lock().unwrap().scenario.sign_out_error = None;
        context.sign_out().await.unwrap();
        assert_eq!(*context.account_id.lock().await, None);

        fixture.0.lock().unwrap().scenario.authorized = true;
        fixture.0.lock().unwrap().scenario.account_id = Some(99);
        assert_eq!(context.local_account_id().await.unwrap(), 99);
    }
}
