mod app_message;
mod telegram;
mod watch;
#[cfg(feature = "e2e")]
use telegram::e2e;

// Keep production command registration identical in normal and E2E builds.
macro_rules! app_handler {
    ($($extra:path),* $(,)?) => {
        tauri::generate_handler![
            telegram::session::commands::session_status,
            telegram::qr::commands::start_qr_login,
            telegram::qr::commands::stop_qr_login,
            telegram::phone::commands::request_phone_code,
            telegram::phone::commands::submit_phone_code,
            telegram::phone::commands::submit_password,
            telegram::session::commands::sign_out,
            watch::commands::create_watch,
            watch::commands::list_watches,
            watch::commands::update_watch,
            watch::commands::delete_watch,
            $($extra),*
        ]
    };
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();
    #[cfg(feature = "e2e")]
    let builder = builder
        .plugin(tauri_plugin_wdio::init())
        .plugin(tauri_plugin_wdio_webdriver::init());

    let builder = builder.manage(telegram::AuthState::default());
    #[cfg(not(feature = "e2e"))]
    let builder = builder.invoke_handler(app_handler!());
    #[cfg(feature = "e2e")]
    let builder = builder
        .manage(std::sync::Arc::new(e2e::FixtureState::default()))
        .invoke_handler(app_handler![
            e2e::commands::configure,
            e2e::commands::inspect,
            e2e::commands::refresh_qr,
            e2e::commands::authorize_qr,
            e2e::commands::fail_qr,
            e2e::commands::require_qr_password,
            e2e::commands::flush_coverage,
        ]);

    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
