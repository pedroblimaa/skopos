use super::{
    fixture::{Fixture, QrReply},
    FixtureState, Scenario,
};
use crate::telegram::state::{AuthState, LoginStep};
use serde::Serialize;
use std::sync::Arc;
use tauri::{AppHandle, Manager, State};

#[tauri::command]
pub(crate) async fn block_storage_fixture(
    app: AppHandle,
    blocked: bool,
    database: Option<String>,
) -> Result<(), String> {
    let directory = std::env::temp_dir().join(format!("skopos-e2e-{}", std::process::id()));
    let target = match database.as_deref() {
        Some(
            name @ ("watches.sqlite" | "chats.sqlite" | "results.sqlite" | "notifications.sqlite"),
        ) => directory.join(name),
        None => directory,
        Some(_) => return Err("Unknown fixture database".into()),
    };
    let backup = target.with_extension("backup");
    let monitoring = app.state::<crate::telegram::monitoring::MonitoringState>();
    monitoring
        .is_suspended
        .store(true, std::sync::atomic::Ordering::SeqCst);
    let search = app.state::<crate::telegram::search::SearchState>();
    let _search = search.cancel_and_wait().await;
    let _monitoring = monitoring.operation.lock().await;
    let notifications = app.state::<crate::notification::NotificationState>();
    let _notifications = notifications.cancel_and_wait().await;

    if blocked {
        std::fs::rename(&target, &backup).map_err(|error| error.to_string())?;
        let result = if database.is_some() {
            std::fs::create_dir(&target)
        } else {
            std::fs::write(&target, b"storage unavailable")
        };

        if let Err(error) = result {
            std::fs::rename(&backup, &target).map_err(|error| error.to_string())?;
            return Err(error.to_string());
        }
    } else {
        if database.is_some() {
            std::fs::remove_dir(&target).map_err(|error| error.to_string())?;
        } else {
            std::fs::remove_file(&target).map_err(|error| error.to_string())?;
        }
        std::fs::rename(&backup, &target).map_err(|error| error.to_string())?;
    }

    if !blocked || database.is_some() {
        monitoring
            .is_suspended
            .store(false, std::sync::atomic::Ordering::SeqCst);
    }

    Ok(())
}

#[tauri::command]
pub(crate) fn focus_window(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window("main")
        .ok_or("Missing test window")?;

    window.set_focus().map_err(|error| error.to_string())?;
    let webview: &tauri::Webview = window.as_ref();

    webview.set_focus().map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) async fn reject_session_fixture(
    app: AppHandle,
    auth: State<'_, AuthState>,
) -> Result<(), crate::app_message::AppMessage> {
    let context = auth.client(&app).await.map_err(|error| error.message())?;
    let generation = context.client.session.snapshot().generation;
    let rejected: Result<(), grammers_client::InvocationError> = Err(
        grammers_client::InvocationError::Rpc(grammers_client::sender::RpcError {
            code: 401,
            name: "SESSION_EXPIRED".into(),
            value: None,
            caused_by: None,
        }),
    );

    context.client.observe(generation, &rejected).await;

    Ok(())
}

#[tauri::command]
pub(crate) async fn configure(
    state: State<'_, Arc<FixtureState>>,
    auth: State<'_, AuthState>,
    monitoring: State<'_, crate::telegram::monitoring::MonitoringState>,
    scenario: Scenario,
) -> Result<(), String> {
    monitoring
        .is_suspended
        .store(!scenario.authorized, std::sync::atomic::Ordering::SeqCst);
    *monitoring.clock.lock().unwrap() = None;
    auth.login.lock().await.begin(LoginStep::Idle);
    auth.qr_update.notify_waiters();

    if let Some(context) = auth.context.get() {
        context.client.session.reset(None);
        *context.account_id.lock().await = None;
    }

    *state.0.lock().unwrap() = Fixture {
        scenario,
        ..Fixture::default()
    };

    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Inspect {
    qr_starts: u32,
    qr_active: bool,
    saved_messages: Vec<serde_json::Value>,
}

#[tauri::command]
pub(crate) async fn inspect(
    state: State<'_, Arc<FixtureState>>,
    auth: State<'_, AuthState>,
) -> Result<Inspect, String> {
    let qr_starts = state.0.lock().unwrap().qr_requests;
    let saved_messages = state.0.lock().unwrap().saved_messages.clone();

    Ok(Inspect {
        qr_starts,
        qr_active: matches!(auth.login.lock().await.step, LoginStep::Qr),
        saved_messages,
    })
}

#[tauri::command]
pub(crate) fn refresh_qr(state: State<'_, Arc<FixtureState>>, auth: State<'_, AuthState>) {
    state.0.lock().unwrap().refreshed = true;
    auth.qr_update.notify_one();
}

#[tauri::command]
pub(crate) fn authorize_qr(state: State<'_, Arc<FixtureState>>, auth: State<'_, AuthState>) {
    state.0.lock().unwrap().qr_reply = Some(QrReply::Authorized);
    auth.qr_update.notify_one();
}

#[tauri::command]
pub(crate) fn fail_qr(state: State<'_, Arc<FixtureState>>, auth: State<'_, AuthState>) {
    state.0.lock().unwrap().qr_reply = Some(QrReply::Error);
    auth.qr_update.notify_one();
}

#[tauri::command]
pub(crate) fn require_qr_password(state: State<'_, Arc<FixtureState>>, auth: State<'_, AuthState>) {
    state.0.lock().unwrap().qr_reply = Some(QrReply::PasswordRequired);
    auth.qr_update.notify_one();
}

// WebDriver may terminate the process forcibly. Flush the instrumented binary
// before teardown so the report contains its executed production commands.
#[tauri::command]
pub(crate) fn flush_coverage() -> Result<(), String> {
    #[cfg(coverage)]
    {
        extern "C" {
            fn __llvm_profile_write_file() -> std::ffi::c_int;
        }
        // SAFETY: LLVM supplies this no-argument function in instrumented builds.
        if unsafe { __llvm_profile_write_file() } != 0 {
            return Err("Could not write native coverage profile".into());
        }
    }

    Ok(())
}
