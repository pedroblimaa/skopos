use super::*;

pub(crate) fn is_exit_probe() -> bool {
    std::env::var_os("SKOPOS_E2E_EXIT_PROBE").is_some()
}

pub(crate) fn run_exit_probe(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let search = app.state::<crate::telegram::search::SearchState>();
        let operation = search.cancel_and_wait().await;

        stop(&app);
        stop(&app);
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        let state = app.state::<MonitoringState>();
        assert!(state.is_stopping.load(Ordering::SeqCst));
        assert!(state.is_suspended.load(Ordering::SeqCst));
        assert!(!app
            .get_webview_window("main")
            .unwrap()
            .is_visible()
            .unwrap());
        std::fs::write(
            crate::telegram::monitoring::directory(&app)
                .unwrap()
                .join("shutdown-verified"),
            b"operations drained before exit",
        )
        .unwrap();
        drop(operation);
    });
}

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
