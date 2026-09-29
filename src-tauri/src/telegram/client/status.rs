use crate::telegram::{api::TelegramApi, error::AuthResult};
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStatus {
    authorized: bool,
    display_name: Option<String>,
}

impl SessionStatus {
    pub(in crate::telegram) fn signed_out() -> Self {
        Self {
            authorized: false,
            display_name: None,
        }
    }

    pub(in crate::telegram) fn authorized(&self) -> bool {
        self.authorized
    }

    pub(in crate::telegram) fn signed_in(display_name: String) -> Self {
        Self {
            authorized: true,
            display_name: Some(display_name),
        }
    }
}

pub(in crate::telegram) async fn status_for(client: &TelegramApi) -> AuthResult<SessionStatus> {
    if !client.is_authorized().await? {
        return Ok(SessionStatus::signed_out());
    }

    let (first_name, last_name) = client.names().await?;
    let display_name = format!(
        "{} {}",
        first_name.as_deref().unwrap_or("Telegram user"),
        last_name.as_deref().unwrap_or("")
    )
    .trim()
    .to_owned();

    Ok(SessionStatus::signed_in(display_name))
}
