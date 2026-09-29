use grammers_client::InvocationError;

#[derive(Debug)]
pub(in crate::telegram) enum AuthError {
    Message(&'static str),
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
    pub(in crate::telegram) fn message(&self) -> String {
        match self {
            Self::Message(message) => (*message).into(),
            Self::Storage => "Could not access the local Telegram session. Check app data permissions and try again.".into(),
            Self::Cancelled => "This login attempt was replaced. Try again.".into(),
            Self::Telegram(InvocationError::Rpc(error)) => rpc_message(error),
            Self::Telegram(InvocationError::Io(_) | InvocationError::Transport(_) | InvocationError::Dropped) => {
                "Could not reach Telegram. Check your connection and try again.".into()
            }
            Self::Telegram(InvocationError::Session(_)) => Self::Storage.message(),
            Self::Telegram(_) => "Telegram could not complete login. Try again.".into(),
        }
    }

    pub(in crate::telegram) fn is_rpc(&self, name: &str) -> bool {
        matches!(self, Self::Telegram(InvocationError::Rpc(error)) if error.name == name)
    }
}

fn rpc_message(error: &grammers_client::sender::RpcError) -> String {
    match error.name.as_str() {
        "FLOOD_WAIT" => match error.value {
            Some(seconds) => format!("Telegram is limiting login attempts. Try again in {seconds} seconds."),
            None => "Telegram is limiting login attempts. Wait before trying again.".into(),
        },
        "PHONE_NUMBER_INVALID" => "Enter a valid phone number with its country code.".into(),
        "PHONE_CODE_EXPIRED" => "This code expired. Request a new one.".into(),
        "PHONE_CODE_INVALID" | "PHONE_CODE_EMPTY" => "That verification code is invalid. Check it and try again.".into(),
        "PASSWORD_HASH_INVALID" => "That password is incorrect. Try again.".into(),
        _ => format!("Telegram rejected the request ({}). Try again or check the account in the official app.", error.name),
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
