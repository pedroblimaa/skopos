use super::credentials;
use crate::telegram::{
    api::TelegramApi,
    error::{AuthError, AuthResult},
    state::AuthState,
};
use grammers_client::{sender::SenderPool, tl, Client};
use grammers_session::{storages::SqliteSession, types::PeerId, Session};
use std::sync::Arc;
use tauri::{AppHandle, Manager};
use tokio::sync::Mutex;

#[derive(Clone)]
pub(in crate::telegram) struct ClientContext {
    pub(in crate::telegram) client: TelegramApi,
    pub(in crate::telegram) session: Arc<SqliteSession>,
    pub(in crate::telegram) account_id: Arc<Mutex<Option<i64>>>,
}

impl ClientContext {
    pub(in crate::telegram) async fn local_account_id(&self) -> AuthResult<i64> {
        #[cfg(any(test, feature = "e2e"))]
        if let Some(fixture) = &self.client.fixture {
            let fixture = fixture.0.lock().map_err(|_| AuthError::Storage)?;
            if !fixture.scenario.authorized {
                return Err(AuthError::Message(
                    crate::app_message::AppMessage::RestartLogin,
                ));
            }
            return Ok(fixture.scenario.account_id.unwrap_or(1));
        }

        let mut account = self.account_id.lock().await;
        if let Some(id) = *account {
            return Ok(id);
        }

        let saved = self
            .session
            .peer(PeerId::self_user())
            .await
            .map_err(|_| AuthError::Storage)?
            .and_then(|peer| peer.id().bare_id());
        let id = match saved {
            Some(id) => id,
            None => {
                let users = self
                    .client
                    .invoke(&tl::functions::users::GetUsers {
                        id: vec![tl::enums::InputUser::UserSelf],
                    })
                    .await?;
                users
                    .into_iter()
                    .find_map(|user| match user {
                        tl::enums::User::User(user) if user.is_self => Some(user.id),
                        _ => None,
                    })
                    .ok_or(AuthError::Message(
                        crate::app_message::AppMessage::RestartLogin,
                    ))?
            }
        };

        *account = Some(id);
        Ok(id)
    }
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

    Ok(ClientContext {
        client,
        session,
        account_id: Arc::new(Mutex::new(None)),
    })
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
