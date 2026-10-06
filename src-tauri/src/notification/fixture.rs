use super::{commands::repository, repository::Record, NotificationState};
use crate::app_message::AppMessage;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::{AppHandle, State};

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", default)]
pub(crate) struct Fixture {
    pub desktop_error: bool,
    pub desktop: Vec<String>,
}

#[tauri::command]
pub(crate) async fn configure_notifications_fixture(
    app: AppHandle,
    state: State<'_, NotificationState>,
    account: i64,
    fixture: Fixture,
    reset: bool,
) -> Result<(), AppMessage> {
    let _operation = state.cancel_and_wait().await;
    *state.fixture.lock().unwrap() = fixture;
    if reset {
        repository(&app)?.save(account, &Record::default()).await?;
    }
    Ok(())
}

#[tauri::command]
pub(crate) fn inspect_notifications_fixture(state: State<'_, NotificationState>) -> Value {
    serde_json::to_value(&*state.fixture.lock().unwrap()).unwrap()
}
