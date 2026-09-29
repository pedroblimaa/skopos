use super::protocol::{code_delivery, send_code, CodeSender};
use super::CodeRequest;
use crate::telegram::{
    client::{status_for, ClientContext, SessionStatus},
    error::{AuthError, AuthResult},
};
use grammers_client::{client::PasswordToken, tl, InvocationError, SignInError};

use grammers_session::Session;

pub(super) enum PhoneRequestOutcome {
    Code { delivery: CodeRequest, hash: String },
    Authorized,
    EmailSetupRequired,
    PaymentRequired,
}

pub(super) trait PhoneRequestApi {
    async fn request(&self, phone: &str) -> AuthResult<PhoneRequestOutcome>;
    async fn status(&self) -> AuthResult<SessionStatus>;
}

pub(super) enum SignInOutcome {
    Authorized,
    SignUpRequired,
}

pub(super) trait PhoneSignInApi {
    async fn sign_in(
        &self,
        phone: &str,
        hash: &str,
        code: &str,
    ) -> Result<SignInOutcome, grammers_client::InvocationError>;
    async fn password_token(&self) -> Result<PasswordToken, grammers_client::InvocationError>;
    async fn check_password(&self, token: PasswordToken, password: &str)
        -> Result<(), SignInError>;
    async fn status(&self) -> AuthResult<SessionStatus>;
}

impl PhoneSignInApi for ClientContext {
    async fn sign_in(
        &self,
        phone: &str,
        hash: &str,
        code: &str,
    ) -> Result<SignInOutcome, grammers_client::InvocationError> {
        let result = self
            .client
            .invoke(&tl::functions::auth::SignIn {
                phone_number: phone.into(),
                phone_code_hash: hash.into(),
                phone_code: Some(code.into()),
                email_verification: None,
            })
            .await?;

        Ok(match result {
            tl::enums::auth::Authorization::Authorization(_) => SignInOutcome::Authorized,
            tl::enums::auth::Authorization::SignUpRequired(_) => SignInOutcome::SignUpRequired,
        })
    }

    async fn password_token(&self) -> Result<PasswordToken, grammers_client::InvocationError> {
        self.client.password_token().await
    }

    async fn check_password(
        &self,
        token: PasswordToken,
        password: &str,
    ) -> Result<(), SignInError> {
        self.client
            .check_password(token, password.as_bytes())
            .await
            .map_err(|error| *error)
    }

    async fn status(&self) -> AuthResult<SessionStatus> {
        status_for(&self.client).await
    }
}

impl PhoneRequestApi for ClientContext {
    async fn request(&self, phone: &str) -> AuthResult<PhoneRequestOutcome> {
        match send_code(self, phone).await? {
            tl::enums::auth::SentCode::Code(sent) => {
                if matches!(
                    sent.r#type,
                    tl::enums::auth::SentCodeType::SetUpEmailRequired(_)
                ) {
                    return Ok(PhoneRequestOutcome::EmailSetupRequired);
                }

                Ok(PhoneRequestOutcome::Code {
                    delivery: code_delivery(sent.r#type),
                    hash: sent.phone_code_hash,
                })
            }
            tl::enums::auth::SentCode::Success(_) => Ok(PhoneRequestOutcome::Authorized),
            tl::enums::auth::SentCode::PaymentRequired(_) => {
                Ok(PhoneRequestOutcome::PaymentRequired)
            }
        }
    }

    async fn status(&self) -> AuthResult<SessionStatus> {
        status_for(&self.client).await
    }
}

impl CodeSender for ClientContext {
    async fn invoke(
        &self,
        request: &tl::functions::auth::SendCode,
    ) -> Result<tl::enums::auth::SentCode, InvocationError> {
        self.client.invoke(request).await
    }

    async fn set_home_dc(&self, dc: i32) -> AuthResult<()> {
        self.session
            .set_home_dc_id(dc)
            .await
            .map_err(|_| AuthError::Storage)
    }
}
