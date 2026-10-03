mod app_message;
mod links;
mod promotion;
mod telegram;
mod watch;
#[cfg(feature = "e2e")]
use telegram::e2e;

// Keep production command registration identical in normal and E2E builds.
macro_rules! app_handler {
    ($($extra:path),* $(,)?) => {
        tauri::generate_handler![
            telegram::session::commands::session_status,
            telegram::session::commands::get_profile_photo,
            telegram::qr::commands::start_qr_login,
            telegram::qr::commands::stop_qr_login,
            telegram::phone::commands::request_phone_code,
            telegram::phone::commands::submit_phone_code,
            telegram::phone::commands::submit_password,
            telegram::session::commands::sign_out,
            telegram::chats::commands::list_chats,
            telegram::chats::commands::get_chat_photo,
            telegram::chats::commands::get_selected_chats,
            telegram::chats::commands::save_selected_chats,
            watch::commands::create_watch,
            watch::commands::list_watches,
            watch::commands::update_watch,
            watch::commands::delete_watch,
            telegram::search::commands::search_products,
            telegram::search::commands::load_search_results,
            telegram::search::commands::clear_search_results,
            links::open_promotion_link,
            $($extra),*
        ]
    };
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default().plugin(tauri_plugin_opener::init());
    #[cfg(feature = "e2e")]
    let builder = builder
        .plugin(tauri_plugin_wdio::init())
        .plugin(tauri_plugin_wdio_webdriver::init());

    let builder = builder
        .manage(telegram::AuthState::default())
        .manage(telegram::search::SearchState::default())
        .manage(telegram::chats::ChatPhotos::default());
    #[cfg(not(feature = "e2e"))]
    let builder = builder.invoke_handler(app_handler!());
    #[cfg(feature = "e2e")]
    let builder = builder
        .manage(std::sync::Arc::new(e2e::FixtureState::default()))
        .invoke_handler(app_handler![
            e2e::commands::configure,
            e2e::commands::focus_window,
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
