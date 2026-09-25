mod telegram;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(telegram::AuthState::default())
        .invoke_handler(tauri::generate_handler![
            telegram::session::session_status,
            telegram::qr::start_qr_login,
            telegram::qr::stop_qr_login,
            telegram::phone::request_phone_code,
            telegram::phone::submit_phone_code,
            telegram::phone::submit_password,
            telegram::session::sign_out,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
