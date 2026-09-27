#[cfg(feature = "e2e")]
mod e2e;
#[cfg(not(feature = "e2e"))]
mod telegram;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default();
    #[cfg(feature = "e2e")]
    let builder = builder
        .plugin(tauri_plugin_wdio::init())
        .plugin(tauri_plugin_wdio_webdriver::init());
    #[cfg(not(feature = "e2e"))]
    let builder = builder
        .manage(telegram::AuthState::default())
        .invoke_handler(tauri::generate_handler![
            telegram::session::session_status,
            telegram::qr::start_qr_login,
            telegram::qr::stop_qr_login,
            telegram::phone::request_phone_code,
            telegram::phone::submit_phone_code,
            telegram::phone::submit_password,
            telegram::session::sign_out,
        ]);
    #[cfg(feature = "e2e")]
    let builder =
        builder
            .manage(e2e::FixtureState::default())
            .invoke_handler(tauri::generate_handler![
                e2e::configure,
                e2e::inspect,
                e2e::emit_qr,
                e2e::complete_qr,
                e2e::emit_error,
                e2e::session_status,
                e2e::start_qr_login,
                e2e::stop_qr_login,
                e2e::request_phone_code,
                e2e::submit_phone_code,
                e2e::submit_password,
                e2e::sign_out,
            ]);
    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
