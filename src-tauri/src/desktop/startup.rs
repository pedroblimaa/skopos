use crate::app_message::AppMessage;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
#[cfg(not(feature = "e2e"))]
use tauri_plugin_autostart::ManagerExt;

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

pub(super) fn save_startup(
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

pub(super) fn load_startup(path: &std::path::Path) -> Result<StartupSettings, AppMessage> {
    match std::fs::read(path) {
        Ok(data) => serde_json::from_slice(&data).map_err(|_| AppMessage::MonitoringStorage),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(StartupSettings::default())
        }

        Err(_) => Err(AppMessage::MonitoringStorage),
    }
}

pub(super) fn register_startup(app: &AppHandle, enabled: bool) -> Result<(), AppMessage> {
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
