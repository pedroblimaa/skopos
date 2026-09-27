//! Deterministic Telegram stand-in, compiled only into the desktop E2E binary.
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

#[derive(Default)]
pub struct FixtureState(Mutex<Fixture>);

#[derive(Default, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Scenario {
    authorized: bool,
    password_required: bool,
    qr_failures: u32,
    request_error: Option<String>,
    sign_out_error: Option<String>,
}

#[derive(Default)]
struct Fixture {
    scenario: Scenario,
    qr_starts: u32,
    qr_stops: u32,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStatus {
    authorized: bool,
    display_name: Option<&'static str>,
}

impl SessionStatus {
    fn for_fixture(fixture: &Fixture) -> Self {
        Self {
            authorized: fixture.scenario.authorized,
            display_name: fixture.scenario.authorized.then_some("Pedro"),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Inspect {
    qr_starts: u32,
    qr_stops: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeRequest {
    message: &'static str,
    length: u8,
}

#[derive(Serialize)]
#[serde(tag = "step", rename_all = "camelCase")]
pub enum LoginResult {
    Authorized { status: SessionStatus },
    PasswordRequired { hint: &'static str },
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct QrToken {
    url: String,
    expires_at: u64,
}

fn qr_token(suffix: &str, seconds: u64) -> QrToken {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    QrToken {
        url: format!("tg://login?token={suffix}"),
        expires_at: now + seconds,
    }
}

#[tauri::command]
pub fn configure(state: State<'_, FixtureState>, scenario: Scenario) -> Result<(), String> {
    *state.0.lock().map_err(|error| error.to_string())? = Fixture {
        scenario,
        ..Fixture::default()
    };
    Ok(())
}

#[tauri::command]
pub fn inspect(state: State<'_, FixtureState>) -> Result<Inspect, String> {
    let fixture = state.0.lock().map_err(|error| error.to_string())?;
    Ok(Inspect {
        qr_starts: fixture.qr_starts,
        qr_stops: fixture.qr_stops,
    })
}

#[tauri::command]
pub fn session_status(state: State<'_, FixtureState>) -> Result<SessionStatus, String> {
    let fixture = state.0.lock().map_err(|error| error.to_string())?;
    Ok(SessionStatus::for_fixture(&fixture))
}

#[tauri::command]
pub fn start_qr_login(app: AppHandle, state: State<'_, FixtureState>) -> Result<(), String> {
    let mut fixture = state.0.lock().map_err(|error| error.to_string())?;
    fixture.qr_starts += 1;
    if fixture.scenario.qr_failures > 0 {
        fixture.scenario.qr_failures -= 1;
        return Err("Network unavailable".into());
    }
    drop(fixture);
    app.emit("telegram:qr-token", qr_token("first", 30))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn stop_qr_login(state: State<'_, FixtureState>) -> Result<(), String> {
    state.0.lock().map_err(|error| error.to_string())?.qr_stops += 1;
    Ok(())
}

#[tauri::command]
pub fn emit_qr(app: AppHandle) -> Result<(), String> {
    app.emit("telegram:qr-token", qr_token("second", 90))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn complete_qr(app: AppHandle, state: State<'_, FixtureState>) -> Result<(), String> {
    let status = {
        let mut fixture = state.0.lock().map_err(|error| error.to_string())?;
        fixture.scenario.authorized = true;
        SessionStatus::for_fixture(&fixture)
    };
    app.emit("telegram:auth-changed", status)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn emit_error(app: AppHandle, message: String) -> Result<(), String> {
    app.emit("telegram:auth-error", message)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn request_phone_code(
    state: State<'_, FixtureState>,
    phone: String,
) -> Result<CodeRequest, String> {
    let _ = phone;
    let fixture = state.0.lock().map_err(|error| error.to_string())?;
    if let Some(error) = &fixture.scenario.request_error {
        return Err(error.clone());
    }
    Ok(CodeRequest {
        message: "Check Telegram for your code.",
        length: 5,
    })
}

#[tauri::command]
pub fn submit_phone_code(
    state: State<'_, FixtureState>,
    code: String,
) -> Result<LoginResult, String> {
    if code != "12345" {
        return Err("That verification code is invalid. Check it and try again.".into());
    }
    let mut fixture = state.0.lock().map_err(|error| error.to_string())?;
    if fixture.scenario.password_required {
        return Ok(LoginResult::PasswordRequired { hint: "My hint" });
    }
    fixture.scenario.authorized = true;
    Ok(LoginResult::Authorized {
        status: SessionStatus::for_fixture(&fixture),
    })
}

#[tauri::command]
pub fn submit_password(
    state: State<'_, FixtureState>,
    password: String,
) -> Result<LoginResult, String> {
    if password != "secret" {
        return Err("That password is incorrect. Try again.".into());
    }
    let mut fixture = state.0.lock().map_err(|error| error.to_string())?;
    fixture.scenario.authorized = true;
    Ok(LoginResult::Authorized {
        status: SessionStatus::for_fixture(&fixture),
    })
}

#[tauri::command]
pub fn sign_out(app: AppHandle, state: State<'_, FixtureState>) -> Result<(), String> {
    let status = {
        let mut fixture = state.0.lock().map_err(|error| error.to_string())?;
        if let Some(error) = &fixture.scenario.sign_out_error {
            return Err(error.clone());
        }
        fixture.scenario.authorized = false;
        SessionStatus::for_fixture(&fixture)
    };
    app.emit("telegram:auth-changed", status)
        .map_err(|error| error.to_string())
}
