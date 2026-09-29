use super::credentials;
use crate::telegram::{
    api::TelegramApi,
    error::{AuthError, AuthResult},
    state::AuthState,
};
use grammers_client::{sender::SenderPool, Client};
use grammers_session::storages::SqliteSession;
use std::sync::Arc;
use tauri::{AppHandle, Manager};

#[derive(Clone)]
pub(in crate::telegram) struct ClientContext {
    pub(in crate::telegram) client: TelegramApi,
    pub(in crate::telegram) session: Arc<SqliteSession>,
}

impl AuthState {
    pub(in crate::telegram) async fn client(&self, app: &AppHandle) -> AuthResult<&ClientContext> {
        self.context
            .get_or_try_init(|| initialize_client(app))
            .await
    }
}

async fn initialize_client(app: &AppHandle) -> AuthResult<ClientContext> {
    let (api_id, _) = credentials()?;
    let session = open_session(app).await?;

    let pool = SenderPool::new(Arc::clone(&session), api_id);
    let client = TelegramApi {
        client: Client::new(pool.handle),
        #[cfg(feature = "e2e")]
        fixture: Some(Arc::clone(
            &app.state::<Arc<crate::telegram::e2e::FixtureState>>(),
        )),
        #[cfg(all(test, not(feature = "e2e")))]
        fixture: None,
    };
    tauri::async_runtime::spawn(pool.runner.run());

    let mut updates = pool.updates;
    let app_handle = app.clone();
    tauri::async_runtime::spawn(async move {
        while updates.recv().await.is_some() {
            app_handle.state::<AuthState>().qr_update.notify_one();
        }
    });

    Ok(ClientContext { client, session })
}

async fn open_session(app: &AppHandle) -> AuthResult<Arc<SqliteSession>> {
    let directory = app
        .path()
        .app_local_data_dir()
        .map_err(|_| AuthError::Storage)?;

    std::fs::create_dir_all(&directory).map_err(|_| AuthError::Storage)?;
    let session = SqliteSession::open(directory.join("telegram.session"))
        .await
        .map_err(|_| AuthError::Storage)?;

    Ok(Arc::new(session))
}
