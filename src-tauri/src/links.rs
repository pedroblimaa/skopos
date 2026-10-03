use crate::app_message::AppMessage;
use tauri_plugin_opener::OpenerExt;

#[tauri::command]
pub async fn open_promotion_link(app: tauri::AppHandle, url: String) -> Result<(), AppMessage> {
    let url = tauri::Url::parse(&url).map_err(|_| AppMessage::OpenLinkFailed)?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(AppMessage::OpenLinkFailed);
    }

    app.opener()
        .open_url(url.as_str(), None::<&str>)
        .map_err(|_| AppMessage::OpenLinkFailed)
}
