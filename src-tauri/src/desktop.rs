use crate::{app_message::AppMessage, telegram::monitoring::MonitoringState};
use serde::{Deserialize, Serialize};
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Manager};
#[cfg(not(feature = "e2e"))]
use tauri_plugin_autostart::ManagerExt;
#[cfg(feature = "e2e")]
pub(crate) mod fixture;
#[cfg(test)]
mod tests;

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StartupSettings {
    pub enabled: bool,
    pub failure: Option<AppMessage>,
}

impl Default for StartupSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            failure: None,
        }
    }
}

#[derive(Default)]
pub(crate) struct DesktopState(pub std::sync::Mutex<StartupSettings>);

pub(crate) fn setup(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let settings = crate::telegram::monitoring::directory(app.handle())
        .and_then(|directory| load_startup(&directory.join("startup.json")));
    let mut settings = settings.unwrap_or(StartupSettings {
        enabled: true,
        failure: Some(AppMessage::MonitoringStorage),
    });

    if settings.failure.is_none() {
        settings.failure = register_startup(app.handle(), settings.enabled).err();
    }

    let tray_failure = setup_tray(app.handle()).err();

    if tray_failure.is_some() {
        settings.failure = tray_failure.clone();
    }
    *app.state::<DesktopState>()
        .0
        .lock()
        .map_err(|_| "desktop state unavailable")? = settings;
    let hidden = std::env::args().any(|arg| arg == "--background");

    if let Some(window) = app.get_webview_window("main") {
        if !hidden || tray_failure.is_some() {
            window.show()?;
        }
    }

    crate::telegram::monitoring::start(app.handle());

    Ok(())
}

fn setup_tray(app: &AppHandle) -> Result<(), AppMessage> {
    use tauri::{
        menu::{Menu, MenuItem},
        tray::TrayIconBuilder,
    };
    let open = MenuItem::with_id(
        app,
        "open",
        "Open Skopos / Abrir Skopos",
        true,
        None::<&str>,
    )
    .map_err(|_| AppMessage::TrayFailed)?;
    let quit = MenuItem::with_id(app, "quit", "Quit / Sair", true, None::<&str>)
        .map_err(|_| AppMessage::TrayFailed)?;
    let menu = Menu::with_items(app, &[&open, &quit]).map_err(|_| AppMessage::TrayFailed)?;
    let icon = app
        .default_window_icon()
        .cloned()
        .ok_or(AppMessage::TrayFailed)?;
    TrayIconBuilder::with_id("skopos")
        .icon(icon)
        .tooltip("Skopos")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show(app),
            "quit" => stop(app),
            _ => {}
        })
        .build(app)
        .map_err(|_| AppMessage::TrayFailed)?;

    Ok(())
}

pub(crate) fn show(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        // Window activation is best-effort; the tray menu remains available for retry.
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

fn stop(app: &AppHandle) {
    let state = app.state::<MonitoringState>();

    if !begin_shutdown(&state) {
        return;
    }

    let handle = app.clone();
    tauri::async_runtime::spawn(async move {
        let search = handle.state::<crate::telegram::search::SearchState>();
        let monitoring = handle.state::<MonitoringState>();
        let notifications = handle.state::<crate::notification::NotificationState>();
        let _operations = drain_operations(&search, &monitoring, &notifications).await;
        handle.exit(0);
    });
}

fn begin_shutdown(state: &MonitoringState) -> bool {
    if state.is_stopping.swap(true, Ordering::SeqCst) {
        return false;
    }

    state.is_suspended.store(true, Ordering::SeqCst);
    state.wake.notify_one();

    true
}

async fn drain_operations<'a>(
    search: &'a crate::telegram::search::SearchState,
    monitoring: &'a MonitoringState,
    notifications: &'a crate::notification::NotificationState,
) -> (
    tokio::sync::MutexGuard<'a, ()>,
    tokio::sync::MutexGuard<'a, ()>,
    tokio::sync::MutexGuard<'a, ()>,
) {
    let search = search.cancel_and_wait().await;
    let monitoring = monitoring.operation.lock().await;
    let notifications = notifications.cancel_and_wait().await;

    (search, monitoring, notifications)
}

pub(crate) fn window_event(window: &tauri::Window, event: &tauri::WindowEvent) {
    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
        if window.app_handle().tray_by_id("skopos").is_some() && window.hide().is_ok() {
            api.prevent_close();
        }
    }
}

#[tauri::command]
pub(crate) async fn startup_settings(app: AppHandle) -> Result<StartupSettings, AppMessage> {
    app.state::<DesktopState>()
        .0
        .lock()
        .map(|settings| settings.clone())
        .map_err(|_| AppMessage::MonitoringStorage)
}

#[tauri::command]
pub(crate) async fn save_startup_settings(
    app: AppHandle,
    enabled: bool,
) -> Result<StartupSettings, AppMessage> {
    let state = app.state::<DesktopState>();
    let mut settings = state.0.lock().map_err(|_| AppMessage::MonitoringStorage)?;
    let directory = crate::telegram::monitoring::directory(&app)?;

    save_startup(&directory, &mut settings, enabled, |enabled| {
        register_startup(&app, enabled)
    })
}

fn save_startup(
    directory: &std::path::Path,
    settings: &mut StartupSettings,
    enabled: bool,
    mut register: impl FnMut(bool) -> Result<(), AppMessage>,
) -> Result<StartupSettings, AppMessage> {
    let next = StartupSettings {
        enabled,
        failure: None,
    };
    let path = directory.join("startup.json");
    let temporary = directory.join("startup.tmp");
    let data = serde_json::to_vec(&next).map_err(|_| AppMessage::MonitoringStorage)?;
    register(enabled)?;

    if std::fs::write(&temporary, data)
        .and_then(|()| std::fs::rename(&temporary, &path))
        .is_err()
    {
        let _ = register(settings.enabled);

        return Err(AppMessage::MonitoringStorage);
    }
    *settings = next.clone();

    Ok(next)
}

fn load_startup(path: &std::path::Path) -> Result<StartupSettings, AppMessage> {
    match std::fs::read(path) {
        Ok(data) => serde_json::from_slice(&data).map_err(|_| AppMessage::MonitoringStorage),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(StartupSettings::default())
        }

        Err(_) => Err(AppMessage::MonitoringStorage),
    }
}

fn register_startup(app: &AppHandle, enabled: bool) -> Result<(), AppMessage> {
    #[cfg(not(feature = "e2e"))]
    {
        if cfg!(debug_assertions) {
            return Ok(());
        }

        let manager = app.autolaunch();
        let result = if enabled {
            manager.enable()
        } else {
            manager.disable()
        };
        result.map_err(|_| AppMessage::StartupFailed)
    }
    #[cfg(feature = "e2e")]
    {
        let _ = (app, enabled);

        Ok(())
    }
}
