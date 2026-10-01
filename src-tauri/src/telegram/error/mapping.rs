use crate::app_message::AppMessage;
use grammers_client::InvocationError;

#[derive(Debug)]
pub(in crate::telegram) enum AuthError {
    Message(AppMessage),
    Telegram(InvocationError),
    Storage,
    Cancelled,
}

pub(in crate::telegram) type AuthResult<T> = Result<T, AuthError>;

impl From<InvocationError> for AuthError {
    fn from(error: InvocationError) -> Self {
        Self::Telegram(error)
    }
}

impl AuthError {
    pub(in crate::telegram) fn message(&self) -> AppMessage {
        match self {
            Self::Message(message) => message.clone(),
            Self::Storage => AppMessage::AuthStorage,
            Self::Cancelled => AppMessage::AuthCancelled,
            Self::Telegram(InvocationError::Rpc(error)) => rpc_message(error),
            Self::Telegram(
                InvocationError::Io(_) | InvocationError::Transport(_) | InvocationError::Dropped,
            ) => AppMessage::AuthNetwork,
            Self::Telegram(InvocationError::Session(_)) => Self::Storage.message(),
            Self::Telegram(_) => AppMessage::AuthLoginFailed,
        }
    }

    pub(in crate::telegram) fn is_rpc(&self, name: &str) -> bool {
        matches!(self, Self::Telegram(InvocationError::Rpc(error)) if error.name == name)
    }
}

fn rpc_message(error: &grammers_client::sender::RpcError) -> AppMessage {
    match error.name.as_str() {
        "FLOOD_WAIT" => match error.value {
            Some(seconds) => AppMessage::FloodWaitSeconds {
                seconds: u64::from(seconds),
            },
            None => AppMessage::FloodWait,
        },
        "PHONE_NUMBER_INVALID" => AppMessage::InvalidPhone,
        "PHONE_CODE_EXPIRED" => AppMessage::CodeExpired,
        "PHONE_CODE_INVALID" | "PHONE_CODE_EMPTY" => AppMessage::InvalidCode,
        "PASSWORD_HASH_INVALID" => AppMessage::IncorrectPassword,
        _ => AppMessage::TelegramRejected {
            name: error.name.clone(),
        },
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
