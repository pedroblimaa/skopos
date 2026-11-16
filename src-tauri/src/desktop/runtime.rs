use super::{
    startup::{load_startup, register_startup},
    DesktopState, StartupSettings,
};
use crate::{app_message::AppMessage, telegram::monitoring::MonitoringState};
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Manager};

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
    #[cfg(feature = "e2e")]
    if super::fixture::is_exit_probe() {
        super::fixture::run_exit_probe(app.handle());
    }

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

pub(super) fn stop(app: &AppHandle) {
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
        #[cfg(feature = "e2e")]
        let _ = crate::telegram::e2e::commands::flush_coverage();
        handle.exit(0);
    });
}

pub(super) fn begin_shutdown(state: &MonitoringState) -> bool {
    if state.is_stopping.swap(true, Ordering::SeqCst) {
        return false;
    }

    state.is_suspended.store(true, Ordering::SeqCst);
    state.wake.notify_one();

    true
}

pub(super) async fn drain_operations<'a>(
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
