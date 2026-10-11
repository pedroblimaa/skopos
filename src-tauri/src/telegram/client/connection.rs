use super::credentials;
use crate::telegram::{
    api::TelegramApi,
    error::{AuthError, AuthResult},
    state::AuthState,
};
use grammers_client::{sender::SenderPool, Client};
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
    pub(in crate::telegram) async fn refresh_status(&self) -> AuthResult<super::SessionStatus> {
        self.client.session.reset(None);
        *self.account_id.lock().await = None;
        super::status_for(&self.client).await
    }

    pub(in crate::telegram) async fn local_account_id(&self) -> AuthResult<i64> {
        if !super::status_for(&self.client).await?.authorized() {
            return Err(AuthError::Message(
                crate::app_message::AppMessage::RestartLogin,
            ));
        }

        let snapshot = self.client.session.snapshot();

        if !snapshot
            .status
            .as_ref()
            .is_some_and(super::SessionStatus::authorized)
        {
            return Err(AuthError::Cancelled);
        }

        let generation = snapshot.generation;
        let mut account = self.account_id.lock().await;

        if self.client.session.snapshot().generation != generation {
            return Err(AuthError::Cancelled);
        }

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
            None => fetch_account_id(&self.client).await?,
        };

        if self.client.session.snapshot().generation != generation {
            return Err(AuthError::Cancelled);
        }
        *account = Some(id);

        Ok(id)
    }
}

async fn fetch_account_id(client: &TelegramApi) -> AuthResult<i64> {
    use crate::telegram::chats::{adapter, ChatError};

    match adapter::account_id(client).await {
        Ok(account) => Ok(account),
        Err(ChatError::Auth(error)) => Err(error),
        Err(ChatError::Telegram(error)) => Err(error.into()),
        Err(error) => Err(AuthError::Message(error.message())),
    }
}

impl AuthState {
    pub(crate) async fn account_id(
        &self,
        app: &AppHandle,
    ) -> Result<i64, crate::app_message::AppMessage> {
        let context = self.client(app).await.map_err(|error| error.message())?;
        context
            .local_account_id()
            .await
            .map_err(|error| error.message())
    }

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
        session: Arc::new(super::SessionCache::default()),
        app: Some(app.clone()),
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
