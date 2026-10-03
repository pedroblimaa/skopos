use crate::app_message::AppMessage;
use tauri_plugin_opener::OpenerExt;

#[tauri::command]
pub async fn open_promotion_link(app: tauri::AppHandle, url: String) -> Result<(), AppMessage> {
    let url = web_url(&url)?;

    app.opener()
        .open_url(url.as_str(), None::<&str>)
        .map_err(|_| AppMessage::OpenLinkFailed)
}

fn web_url(value: &str) -> Result<tauri::Url, AppMessage> {
    let url = tauri::Url::parse(value).map_err(|_| AppMessage::OpenLinkFailed)?;
    if !matches!(url.scheme(), "http" | "https") || url.host_str().is_none() {
        return Err(AppMessage::OpenLinkFailed);
    }

    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_web_links_and_rejects_files_scripts_and_custom_protocols() {
        for url in [
            "https://a.aliexpress.com/_c4eQxgBL",
            "http://shop.example/item?q=1&ref=2",
        ] {
            assert_eq!(web_url(url).unwrap().as_str(), url);
        }

        for url in [
            "javascript:alert(1)",
            "file:///C:/private.txt",
            "tg://resolve?domain=deals",
            "invalid",
            "https://",
        ] {
            assert_eq!(web_url(url).unwrap_err(), AppMessage::OpenLinkFailed);
        }
    }
}
