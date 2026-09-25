use std::sync::Arc;

use grammers_client::{sender::SenderPool, Client};
use grammers_session::storages::SqliteSession;
use serde::Serialize;
use tauri::{AppHandle, Manager};

use super::{
    error::{AuthError, AuthResult},
    state::AuthState,
};

#[derive(Clone)]
pub(super) struct ClientContext {
    pub(super) client: Client,
    pub(super) session: Arc<SqliteSession>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStatus {
    authorized: bool,
    display_name: Option<String>,
}

impl SessionStatus {
    pub(super) fn signed_out() -> Self {
        Self {
            authorized: false,
            display_name: None,
        }
    }

    pub(super) fn authorized(&self) -> bool {
        self.authorized
    }
}

pub(super) fn credentials() -> AuthResult<(i32, &'static str)> {
    const MISSING: &str =
        "Telegram API credentials are missing. Set TG_ID and TG_HASH in .env, then rebuild Skopos.";
    let id = option_env!("TG_ID")
        .filter(|value| !value.is_empty())
        .ok_or(AuthError::Message(MISSING))?
        .parse::<i32>()
        .map_err(|_| AuthError::Message("The configured Telegram API ID is invalid."))?;
    let hash = option_env!("TG_HASH")
        .filter(|value| !value.is_empty())
        .ok_or(AuthError::Message(MISSING))?;
    Ok((id, hash))
}

impl AuthState {
    pub(super) async fn client(&self, app: &AppHandle) -> AuthResult<&ClientContext> {
        self.context
            .get_or_try_init(|| async {
                let (api_id, _) = credentials()?;
                let directory = app
                    .path()
                    .app_local_data_dir()
                    .map_err(|_| AuthError::Storage)?;
                std::fs::create_dir_all(&directory).map_err(|_| AuthError::Storage)?;
                let session = Arc::new(
                    SqliteSession::open(directory.join("telegram.session"))
                        .await
                        .map_err(|_| AuthError::Storage)?,
                );
                let pool = SenderPool::new(Arc::clone(&session), api_id);
                let client = Client::new(pool.handle);
                tauri::async_runtime::spawn(pool.runner.run());

                let mut updates = pool.updates;
                let app_handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    while updates.recv().await.is_some() {
                        let state = app_handle.state::<AuthState>();
                        state.qr_update.notify_one();
                    }
                });

                Ok(ClientContext { client, session })
            })
            .await
    }
}

pub(super) async fn status_for(client: &Client) -> AuthResult<SessionStatus> {
    if !client.is_authorized().await? {
        return Ok(SessionStatus::signed_out());
    }
    let user = client.get_me().await?;
    let display_name = format!(
        "{} {}",
        user.first_name().unwrap_or("Telegram user"),
        user.last_name().unwrap_or("")
    )
    .trim()
    .to_owned();
    Ok(SessionStatus {
        authorized: true,
        display_name: Some(display_name),
    })
}
