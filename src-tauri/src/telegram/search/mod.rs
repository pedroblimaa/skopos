mod adapter;
pub mod commands;
pub(crate) mod execution;
mod photos;
#[cfg(test)]
mod tests;
mod workflow;

use crate::app_message::AppMessage;
use grammers_client::InvocationError;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::Mutex;

#[derive(Default)]
pub struct SearchState {
    pub(super) operation: Mutex<()>,
    pub(super) cancellation: AtomicU64,
}

impl SearchState {
    pub(crate) async fn cancel_and_wait(&self) -> tokio::sync::MutexGuard<'_, ()> {
        self.cancellation.fetch_add(1, Ordering::SeqCst);
        self.operation.lock().await
    }
}

#[derive(Debug)]
pub(super) enum SearchError {
    Telegram(InvocationError),
    Incomplete,
    Cancelled,
}

impl SearchError {
    fn message(&self) -> AppMessage {
        match self {
            Self::Telegram(InvocationError::Rpc(error)) if error.code == 401 => {
                AppMessage::RestartLogin
            }

            Self::Telegram(InvocationError::Rpc(error)) if error.name == "FLOOD_WAIT" => {
                match error.value {
                    Some(seconds) => AppMessage::ChatRateLimitSeconds {
                        seconds: u64::from(seconds),
                    },
                    None => AppMessage::ChatRateLimit,
                }
            }
            Self::Cancelled => AppMessage::AuthCancelled,
            _ => AppMessage::SearchFailed,
        }
    }

    fn is_unavailable(&self) -> bool {
        matches!(self, Self::Telegram(InvocationError::Rpc(error)) if matches!(error.name.as_str(), "CHANNEL_PRIVATE" | "CHAT_ID_INVALID" | "CHANNEL_INVALID" | "USER_BANNED_IN_CHANNEL" | "CHAT_ADMIN_REQUIRED"))
    }
}

impl From<InvocationError> for SearchError {
    fn from(error: InvocationError) -> Self {
        Self::Telegram(error)
    }
}
