use super::{
    fixture::{Fixture, QrReply},
    FixtureState, Scenario,
};
use crate::telegram::state::{AuthState, LoginStep};
use serde::Serialize;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub(crate) async fn configure(
    state: State<'_, Arc<FixtureState>>,
    auth: State<'_, AuthState>,
    scenario: Scenario,
) -> Result<(), String> {
    auth.login.lock().await.begin(LoginStep::Idle);
    auth.qr_update.notify_waiters();

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
}

#[tauri::command]
pub(crate) async fn inspect(
    state: State<'_, Arc<FixtureState>>,
    auth: State<'_, AuthState>,
) -> Result<Inspect, String> {
    let qr_starts = state.0.lock().unwrap().qr_requests;

    Ok(Inspect {
        qr_starts,
        qr_active: matches!(auth.login.lock().await.step, LoginStep::Qr),
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
