pub mod commands;
mod content;
mod delivery;
#[cfg(feature = "e2e")]
pub(crate) mod fixture;
mod repository;
#[cfg(test)]
mod tests;

use crate::app_message::AppMessage;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::sync::{Mutex, Notify};

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Settings {
    pub telegram_enabled: bool,
    pub desktop_enabled: bool,
    pub language: String,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            telegram_enabled: true,
            desktop_enabled: true,
            language: "pt-BR".into(),
        }
    }
}

#[derive(Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeliveryStatus {
    pub pending: usize,
    pub uncertain: usize,
    pub failure: Option<AppMessage>,
}

#[derive(Default)]
pub(crate) struct NotificationState {
    operation: Mutex<()>,
    worker: Mutex<()>,
    generation: AtomicU64,
    wake: Notify,
    #[cfg(feature = "e2e")]
    fixture: std::sync::Arc<std::sync::Mutex<fixture::Fixture>>,
}

impl NotificationState {
    pub(crate) fn cancel(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
        self.wake.notify_waiters();
    }

    pub(crate) async fn cancel_and_wait(&self) -> tokio::sync::MutexGuard<'_, ()> {
        self.cancel();
        let worker = self.worker.lock().await;
        drop(worker);
        self.operation.lock().await
    }
}

pub(crate) use delivery::enqueue;
pub(crate) use delivery::start as resume;
