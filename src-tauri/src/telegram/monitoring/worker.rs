use super::publish;
use crate::{
    app_message::AppMessage,
    telegram::{search::execution, AuthState},
};
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Listener, Manager};
use tokio::sync::{Mutex, Notify};

#[derive(Default)]
pub(crate) struct MonitoringState {
    #[cfg(feature = "e2e")]
    pub(crate) clock: std::sync::Mutex<Option<i64>>,
    pub(crate) operation: Mutex<()>,
    pub(crate) wake: Notify,
    pub(crate) is_running: AtomicBool,
    pub(crate) is_suspended: AtomicBool,
    pub(crate) is_stopping: AtomicBool,
    pub(crate) failure: Mutex<Option<AppMessage>>,
}

impl MonitoringState {
    pub(crate) fn can_run(&self) -> bool {
        !self.is_suspended.load(Ordering::SeqCst) && !self.is_stopping.load(Ordering::SeqCst)
    }
}

pub(crate) fn start(app: &AppHandle) {
    let handle = app.clone();
    app.listen("telegram:auth-changed", move |event| {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(event.payload()) {
            let state = handle.state::<MonitoringState>();
            state
                .is_suspended
                .store(value["authorized"] != true, Ordering::SeqCst);

            if let Ok(mut failure) = state.failure.try_lock() {
                *failure = None;
            }

            state.wake.notify_one();
        }
    });
    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let state = handle.state::<MonitoringState>();

        loop {
            if state.is_stopping.load(Ordering::SeqCst) {
                break;
            }

            poll(&handle, &state).await;
            tokio::select! {
                _ = tokio::time::sleep(std::time::Duration::from_secs(60)) => {},
                _ = state.wake.notified() => {},
            }
        }
    });
}

async fn poll(app: &AppHandle, state: &MonitoringState) {
    #[cfg(feature = "e2e")]
    if state.clock.lock().unwrap().is_none() {
        return;
    }

    if state.is_suspended.load(Ordering::SeqCst) {
        return;
    }

    let result = execution::automatic(app).await;
    *state.failure.lock().await = result.err();

    if state.is_suspended.load(Ordering::SeqCst) || state.is_stopping.load(Ordering::SeqCst) {
        return;
    }

    let auth = app.state::<AuthState>();
    let Ok(context) = auth.client(app).await else {
        return;
    };
    let Ok(account) = context.local_account_id().await else {
        return;
    };
    publish(app, account).await;
}
