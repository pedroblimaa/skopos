use crate::app_message::AppMessage;
use crate::telegram::{
    client::{credentials, ClientContext},
    error::{AuthError, AuthResult},
};
use grammers_client::{tl, InvocationError};

pub(in crate::telegram::phone) trait CodeSender {
    async fn invoke(
        &self,
        request: &tl::functions::auth::SendCode,
    ) -> Result<tl::enums::auth::SentCode, InvocationError>;
    async fn set_home_dc(&self, dc: i32) -> AuthResult<()>;
}

pub(in crate::telegram::phone) async fn send_code(
    context: &ClientContext,
    phone: &str,
) -> AuthResult<tl::enums::auth::SentCode> {
    let (api_id, api_hash) = credentials()?;

    send_code_with(context, api_id, api_hash, phone).await
}

pub(super) async fn send_code_with<A: CodeSender>(
    api: &A,
    api_id: i32,
    api_hash: &str,
    phone: &str,
) -> AuthResult<tl::enums::auth::SentCode> {
    let request = tl::functions::auth::SendCode {
        phone_number: phone.to_owned(),
        api_id,
        api_hash: api_hash.to_owned(),
        settings: tl::types::CodeSettings {
            allow_flashcall: false,
            current_number: false,
            allow_app_hash: false,
            allow_missed_call: false,
            allow_firebase: false,
            logout_tokens: None,
            token: None,
            app_sandbox: None,
            unknown_number: false,
        }
        .into(),
    };

    match api.invoke(&request).await {
        Err(InvocationError::Rpc(error)) if error.code == 303 => {
            let dc = error
                .value
                .ok_or(AuthError::Message(AppMessage::MissingDataCenter))?
                as i32;
            api.set_home_dc(dc).await?;
            Ok(api.invoke(&request).await?)
        }
        result => Ok(result?),
    }
}
