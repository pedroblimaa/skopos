use crate::app_message::AppMessage;
use crate::telegram::{
    client::{credentials, ClientContext},
    error::{AuthError, AuthResult},
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use grammers_client::tl;
use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct QrToken {
    pub(super) url: String,
    pub(super) expires_at: u64,
}

pub(super) enum QrOutcome {
    Token(QrToken),
    Authorized,
}

pub(super) trait QrSender {
    async fn export(
        &self,
        request: &tl::functions::auth::ExportLoginToken,
    ) -> Result<tl::enums::auth::LoginToken, grammers_client::InvocationError>;
    async fn import(
        &self,
        dc: i32,
        token: Vec<u8>,
    ) -> Result<tl::enums::auth::LoginToken, grammers_client::InvocationError>;
    async fn set_home_dc(&self, dc: i32) -> AuthResult<()>;
}

pub(super) async fn export_qr(context: &ClientContext) -> AuthResult<QrOutcome> {
    let (api_id, api_hash) = credentials()?;

    export_qr_with(context, api_id, api_hash).await
}

pub(super) async fn export_qr_with<A: QrSender>(
    api: &A,
    api_id: i32,
    api_hash: &str,
) -> AuthResult<QrOutcome> {
    let response = api
        .export(&tl::functions::auth::ExportLoginToken {
            api_id,
            api_hash: api_hash.to_owned(),
            except_ids: vec![],
        })
        .await?;

    let response = match response {
        tl::enums::auth::LoginToken::MigrateTo(migration) => {
            api.set_home_dc(migration.dc_id).await?;
            api.import(migration.dc_id, migration.token).await?
        }
        other => other,
    };

    match response {
        tl::enums::auth::LoginToken::Token(token) => Ok(QrOutcome::Token(QrToken {
            url: format!("tg://login?token={}", URL_SAFE_NO_PAD.encode(token.token)),
            expires_at: token.expires as u64,
        })),
        tl::enums::auth::LoginToken::Success(_) => Ok(QrOutcome::Authorized),
        tl::enums::auth::LoginToken::MigrateTo(_) => {
            Err(AuthError::Message(AppMessage::QrMigrationFailed))
        }
    }
}
