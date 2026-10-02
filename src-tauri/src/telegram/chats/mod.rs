mod adapter;
pub mod commands;
mod mapping;
mod photos;
mod repository;
#[cfg(test)]
mod tests;

pub use photos::ChatPhotos;

use crate::app_message::AppMessage;
use crate::telegram::error::AuthError;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Chat {
    pub id: String,
    pub title: String,
    pub kind: ChatKind,
    pub username: Option<String>,
    pub available: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ChatKind {
    Group,
    Channel,
}

#[derive(Debug)]
enum ChatError {
    Auth(AuthError),
    Telegram(grammers_client::InvocationError),
    Storage,
    InvalidSelection,
    IncompleteList,
}

impl ChatError {
    fn message(&self) -> AppMessage {
        match self {
            Self::Auth(error) => error.message(),
            Self::Telegram(grammers_client::InvocationError::Rpc(error)) if error.code == 401 => {
                AppMessage::RestartLogin
            }
            Self::Telegram(grammers_client::InvocationError::Rpc(error))
                if error.name == "FLOOD_WAIT" =>
            {
                match error.value {
                    Some(seconds) => AppMessage::ChatRateLimitSeconds {
                        seconds: u64::from(seconds),
                    },
                    None => AppMessage::ChatRateLimit,
                }
            }
            Self::Telegram(_) => AppMessage::ChatLoadFailed,
            Self::Storage => AppMessage::ChatStorage,
            Self::InvalidSelection => AppMessage::InvalidChatSelection,
            Self::IncompleteList => AppMessage::ChatListIncomplete,
        }
    }
}

impl From<grammers_client::InvocationError> for ChatError {
    fn from(error: grammers_client::InvocationError) -> Self {
        Self::Telegram(error)
    }
}
