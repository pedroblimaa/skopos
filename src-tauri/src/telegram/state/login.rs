use crate::app_message::AppMessage;
use grammers_client::client::PasswordToken;
use tokio::sync::{Mutex, Notify, OnceCell};

use crate::telegram::{
    client::ClientContext,
    error::{AuthError, AuthResult},
};

#[derive(Default)]
pub struct AuthState {
    pub(in crate::telegram) context: OnceCell<ClientContext>,
    pub(in crate::telegram) login: Mutex<LoginState>,
    pub(in crate::telegram) qr_update: Notify,
}

#[derive(Default)]
pub(in crate::telegram) struct LoginState {
    pub(in crate::telegram) generation: u64,
    pub(in crate::telegram) step: LoginStep,
}

#[derive(Default)]
pub(in crate::telegram) enum LoginStep {
    #[default]
    Idle,
    Qr,
    RequestingPhone,
    Phone {
        phone: String,
        hash: String,
    },
    SubmittingCode,
    Password(Box<PasswordToken>),
    SubmittingPassword,
    SigningOut,
}

impl LoginState {
    pub(in crate::telegram) fn begin(&mut self, step: LoginStep) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        self.step = step;

        self.generation
    }

    pub(in crate::telegram) fn begin_login(&mut self, step: LoginStep) -> AuthResult<u64> {
        if matches!(self.step, LoginStep::SigningOut) {
            return Err(AuthError::Message(AppMessage::SignOutInProgress));
        }

        Ok(self.begin(step))
    }

    pub(in crate::telegram) fn take_phone(&mut self) -> AuthResult<(u64, String, String)> {
        let (phone, hash) = match std::mem::take(&mut self.step) {
            LoginStep::Phone { phone, hash } => (phone, hash),
            other => {
                self.step = other;
                return Err(AuthError::Message(AppMessage::RequestCodeFirst));
            }
        };

        self.step = LoginStep::SubmittingCode;
        Ok((self.generation, phone, hash))
    }

    pub(in crate::telegram) fn take_password(&mut self) -> AuthResult<(u64, PasswordToken)> {
        let token = match std::mem::take(&mut self.step) {
            LoginStep::Password(token) => token,
            other => {
                self.step = other;
                return Err(AuthError::Message(AppMessage::RestartLogin));
            }
        };

        self.step = LoginStep::SubmittingPassword;
        Ok((self.generation, *token))
    }

    pub(in crate::telegram) fn is_qr(&self, generation: u64) -> bool {
        self.generation == generation && matches!(self.step, LoginStep::Qr)
    }

    pub(in crate::telegram) fn is_step(&self, generation: u64, step: &LoginStep) -> bool {
        self.generation == generation
            && std::mem::discriminant(&self.step) == std::mem::discriminant(step)
    }

    pub(in crate::telegram) fn reset_if(&mut self, generation: u64, step: &LoginStep) -> bool {
        if !self.is_step(generation, step) {
            return false;
        }

        self.step = LoginStep::Idle;
        true
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
