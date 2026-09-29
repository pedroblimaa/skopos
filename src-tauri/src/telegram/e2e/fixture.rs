use super::responses::{authorization, password, rpc};
use grammers_client::{client::PasswordToken, tl, InvocationError, SignInError};
use serde::Deserialize;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tl::{Deserializable, Identifiable, Serializable};

#[derive(Default)]
pub(crate) struct FixtureState(pub(in crate::telegram) Mutex<Fixture>);

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct Scenario {
    pub(in crate::telegram) authorized: bool,
    pub(in crate::telegram) password_required: bool,
    pub(in crate::telegram) code_expired: bool,
    pub(in crate::telegram) password_error: bool,
    pub(in crate::telegram) immediate_authorized: bool,
    pub(in crate::telegram) qr_failures: u32,
    pub(in crate::telegram) request_error: Option<String>,
    pub(in crate::telegram) status_error: Option<String>,
    pub(in crate::telegram) status_error_after_login: bool,
    pub(in crate::telegram) sign_out_error: Option<String>,
}

#[derive(Default)]
pub(in crate::telegram) struct Fixture {
    pub(in crate::telegram) scenario: Scenario,
    pub(in crate::telegram) replies: std::collections::VecDeque<Result<Vec<u8>, InvocationError>>,
    pub(super) qr_requests: u32,
    pub(super) refreshed: bool,
    pub(super) qr_reply: Option<QrReply>,
}

pub(super) enum QrReply {
    Authorized,
    PasswordRequired,
    Error,
}

impl FixtureState {
    pub(in crate::telegram) fn invoke<R: tl::RemoteCall>(
        &self,
        request: &R,
    ) -> Result<R::Return, InvocationError> {
        let bytes = request.to_bytes();
        let header = bytes.get(..4).ok_or(InvocationError::Dropped)?;
        let id = u32::from_le_bytes(header.try_into().map_err(|_| InvocationError::Dropped)?);

        let mut fixture = self.0.lock().unwrap();
        let response = match fixture.replies.pop_front() {
            Some(reply) => reply?,
            None => fixture.response(id, &bytes[4..])?,
        };

        R::Return::from_bytes(&response).map_err(|_| InvocationError::Dropped)
    }

    pub(in crate::telegram) fn is_authorized(&self) -> Result<bool, InvocationError> {
        let fixture = self.0.lock().unwrap();
        if fixture.scenario.status_error.is_some()
            || (fixture.scenario.status_error_after_login && fixture.scenario.authorized)
        {
            return Err(InvocationError::Dropped);
        }

        Ok(fixture.scenario.authorized)
    }

    pub(in crate::telegram) fn check_password(
        &self,
        token: PasswordToken,
        password: &[u8],
    ) -> Result<(), Box<SignInError>> {
        if self.0.lock().unwrap().scenario.password_error {
            return Err(Box::new(SignInError::Other(InvocationError::Dropped)));
        }

        if password != b"secret" {
            return Err(Box::new(SignInError::InvalidPassword(token)));
        }

        self.0.lock().unwrap().scenario.authorized = true;
        Ok(())
    }

    pub(in crate::telegram) fn sign_out(&self) -> Result<(), InvocationError> {
        let mut fixture = self.0.lock().unwrap();
        if fixture.scenario.sign_out_error.is_some() {
            return Err(InvocationError::Dropped);
        }

        fixture.scenario.authorized = false;
        Ok(())
    }
}

impl Fixture {
    fn response(&mut self, id: u32, body: &[u8]) -> Result<Vec<u8>, InvocationError> {
        match id {
            tl::functions::auth::SendCode::CONSTRUCTOR_ID => Ok(self.send_code()?.to_bytes()),
            tl::functions::auth::SignIn::CONSTRUCTOR_ID => self.sign_in(body),
            tl::functions::account::GetPassword::CONSTRUCTOR_ID => {
                let response: tl::enums::account::Password = password().into();
                Ok(response.to_bytes())
            }
            tl::functions::auth::ExportLoginToken::CONSTRUCTOR_ID
            | tl::functions::auth::ImportLoginToken::CONSTRUCTOR_ID => {
                Ok(self.login_token()?.to_bytes())
            }
            _ => Err(InvocationError::Dropped),
        }
    }

    fn send_code(&mut self) -> Result<tl::enums::auth::SentCode, InvocationError> {
        if self.scenario.request_error.is_some() {
            return Err(InvocationError::Dropped);
        }

        if self.scenario.immediate_authorized {
            self.scenario.authorized = true;
            return Ok(tl::types::auth::SentCodeSuccess {
                authorization: authorization(),
            }
            .into());
        }

        Ok(tl::types::auth::SentCode {
            r#type: tl::types::auth::SentCodeTypeApp { length: 5 }.into(),
            phone_code_hash: "hash".into(),
            next_type: None,
            timeout: None,
        }
        .into())
    }

    fn sign_in(&mut self, body: &[u8]) -> Result<Vec<u8>, InvocationError> {
        let request =
            tl::functions::auth::SignIn::from_bytes(body).map_err(|_| InvocationError::Dropped)?;

        if self.scenario.code_expired {
            return Err(rpc("PHONE_CODE_EXPIRED"));
        }

        if request.phone_code.as_deref() != Some("12345") {
            return Err(rpc("PHONE_CODE_INVALID"));
        }
        if self.scenario.password_required {
            return Err(rpc("SESSION_PASSWORD_NEEDED"));
        }

        self.scenario.authorized = true;
        Ok(authorization().to_bytes())
    }

    fn login_token(&mut self) -> Result<tl::enums::auth::LoginToken, InvocationError> {
        self.qr_requests += 1;
        if self.scenario.qr_failures > 0 {
            self.scenario.qr_failures -= 1;
            return Err(InvocationError::Dropped);
        }

        match self.qr_reply.take() {
            Some(QrReply::Error) => Err(InvocationError::Dropped),
            Some(QrReply::PasswordRequired) => Err(rpc("SESSION_PASSWORD_NEEDED")),
            Some(QrReply::Authorized) => {
                self.scenario.authorized = true;
                Ok(tl::types::auth::LoginTokenSuccess {
                    authorization: authorization(),
                }
                .into())
            }
            None => Ok(self.pending_token()),
        }
    }

    fn pending_token(&self) -> tl::enums::auth::LoginToken {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i32;
        let (token, lifetime) = if self.refreshed {
            (vec![2], 90)
        } else {
            (vec![1], 30)
        };

        tl::types::auth::LoginToken {
            token,
            expires: now + lifetime,
        }
        .into()
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
