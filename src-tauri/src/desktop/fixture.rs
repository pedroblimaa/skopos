use super::*;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DesktopStatus {
    visible: bool,
    stopping: bool,
    suspended: bool,
}

#[tauri::command]
pub(crate) async fn desktop_fixture(
    app: AppHandle,
    action: String,
) -> Result<DesktopStatus, String> {
    let window = app
        .get_webview_window("main")
        .ok_or("Missing test window")?;
    let state = app.state::<MonitoringState>();

    match action.as_str() {
        "close" => window.close().map_err(|error| error.to_string())?,
        "show" => show(&app),
        "inspect" => {}
        _ => return Err("Unknown desktop fixture action".into()),
    }

    Ok(DesktopStatus {
        visible: window.is_visible().map_err(|error| error.to_string())?,
        stopping: state.is_stopping.load(Ordering::SeqCst),
        suspended: state.is_suspended.load(Ordering::SeqCst),
    })
}
