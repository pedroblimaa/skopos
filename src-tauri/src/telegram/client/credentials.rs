use crate::app_message::AppMessage;
use crate::telegram::error::{AuthError, AuthResult};

pub(in crate::telegram) fn credentials() -> AuthResult<(i32, &'static str)> {
    #[cfg(test)]
    let values = (Some("1"), Some("test"));
    #[cfg(not(test))]
    let values = (option_env!("TG_ID"), option_env!("TG_HASH"));

    parse_credentials(values.0, values.1)
}

pub(super) fn parse_credentials<'a>(
    id: Option<&str>,
    hash: Option<&'a str>,
) -> AuthResult<(i32, &'a str)> {
    let id = id
        .filter(|value| !value.is_empty())
        .ok_or(AuthError::Message(AppMessage::MissingCredentials))?
        .parse::<i32>()
        .map_err(|_| AuthError::Message(AppMessage::InvalidApiId))?;
    let hash = hash
        .filter(|value| !value.is_empty())
        .ok_or(AuthError::Message(AppMessage::MissingCredentials))?;

    Ok((id, hash))
}
