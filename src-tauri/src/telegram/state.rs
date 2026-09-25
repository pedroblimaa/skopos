use grammers_client::client::PasswordToken;
use tokio::sync::{Mutex, Notify, OnceCell};

use super::{
    client::ClientContext,
    error::{AuthError, AuthResult},
};

#[derive(Default)]
pub struct AuthState {
    pub(super) context: OnceCell<ClientContext>,
    pub(super) login: Mutex<LoginState>,
    pub(super) qr_update: Notify,
}

#[derive(Default)]
pub(super) struct LoginState {
    pub(super) generation: u64,
    pub(super) step: LoginStep,
}

#[derive(Default)]
pub(super) enum LoginStep {
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
    pub(super) fn begin(&mut self, step: LoginStep) -> u64 {
        self.generation = self.generation.wrapping_add(1);
        self.step = step;
        self.generation
    }

    pub(super) fn begin_login(&mut self, step: LoginStep) -> AuthResult<u64> {
        if matches!(self.step, LoginStep::SigningOut) {
            return Err(AuthError::Message(
                "Sign-out is still in progress. Try again shortly.",
            ));
        }
        Ok(self.begin(step))
    }

    pub(super) fn take_phone(&mut self) -> AuthResult<(u64, String, String)> {
        let (phone, hash) = match std::mem::take(&mut self.step) {
            LoginStep::Phone { phone, hash } => (phone, hash),
            other => {
                self.step = other;
                return Err(AuthError::Message("Request a login code first."));
            }
        };
        self.step = LoginStep::SubmittingCode;
        Ok((self.generation, phone, hash))
    }

    pub(super) fn take_password(&mut self) -> AuthResult<(u64, PasswordToken)> {
        let token = match std::mem::take(&mut self.step) {
            LoginStep::Password(token) => token,
            other => {
                self.step = other;
                return Err(AuthError::Message("Start Telegram login again."));
            }
        };
        self.step = LoginStep::SubmittingPassword;
        Ok((self.generation, *token))
    }

    pub(super) fn is_qr(&self, generation: u64) -> bool {
        self.generation == generation && matches!(self.step, LoginStep::Qr)
    }

    pub(super) fn is_step(&self, generation: u64, step: &LoginStep) -> bool {
        self.generation == generation
            && std::mem::discriminant(&self.step) == std::mem::discriminant(step)
    }

    pub(super) fn reset_if(&mut self, generation: u64, step: &LoginStep) -> bool {
        if !self.is_step(generation, step) {
            return false;
        }
        self.step = LoginStep::Idle;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::{LoginState, LoginStep};

    #[test]
    fn old_generation_cannot_reset_a_new_step() {
        let mut state = LoginState::default();
        let old = state.begin(LoginStep::Qr);
        let current = state.begin(LoginStep::RequestingPhone);

        assert!(!state.reset_if(old, &LoginStep::Qr));
        assert!(state.is_step(current, &LoginStep::RequestingPhone));
    }

    #[test]
    fn wrong_action_preserves_login_step() {
        let mut state = LoginState::default();
        let generation = state.begin(LoginStep::Phone {
            phone: "+5511999999999".into(),
            hash: "hash".into(),
        });

        assert!(state.take_password().is_err());
        assert!(state.is_step(
            generation,
            &LoginStep::Phone {
                phone: String::new(),
                hash: String::new()
            }
        ));
        assert!(state.take_phone().is_ok());
        assert!(state.is_step(generation, &LoginStep::SubmittingCode));
    }

    #[test]
    fn sign_out_blocks_new_login() {
        let mut state = LoginState::default();
        state.begin(LoginStep::SigningOut);
        assert!(state.begin_login(LoginStep::Qr).is_err());
        assert!(matches!(state.step, LoginStep::SigningOut));
    }
}
