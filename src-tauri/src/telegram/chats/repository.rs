use super::{Chat, ChatError, ChatKind};
use libsql::{params, Builder};
use std::{collections::HashSet, path::PathBuf};

pub(super) struct ChatRepository {
    path: PathBuf,
}

impl ChatRepository {
    pub(super) fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub(super) async fn get(&self, account: i64) -> Result<Vec<Chat>, ChatError> {
        let connection = self.connect().await?;
        let mut rows = connection
            .query(
                "SELECT chats FROM selected_chats WHERE account_id = ?1",
                [account],
            )
            .await
            .map_err(|_| ChatError::Storage)?;
        let Some(row) = rows.next().await.map_err(|_| ChatError::Storage)? else {
            return Ok(Vec::new());
        };
        let json: String = row.get(0).map_err(|_| ChatError::Storage)?;

        serde_json::from_str(&json).map_err(|_| ChatError::Storage)
    }

    pub(super) async fn save(&self, account: i64, chats: Vec<Chat>) -> Result<(), ChatError> {
        let mut ids = HashSet::new();
        for chat in &chats {
            let (peer, id) = chat.id.split_once(':').ok_or(ChatError::InvalidSelection)?;
            let is_valid_peer = match chat.kind {
                ChatKind::Group => peer == "chat" || peer == "channel",
                ChatKind::Channel => peer == "channel",
            };
            if !is_valid_peer
                || id.parse::<i64>().map_or(true, |id| id <= 0)
                || chat.title.trim().is_empty()
                || !ids.insert(&chat.id)
            {
                return Err(ChatError::InvalidSelection);
            }
        }

        let json = serde_json::to_string(&chats).map_err(|_| ChatError::Storage)?;
        let connection = self.connect().await?;
        // One row stores the whole selection, so replacement is a single atomic statement.
        connection.execute(
            "INSERT INTO selected_chats (account_id, chats) VALUES (?1, ?2) ON CONFLICT(account_id) DO UPDATE SET chats = excluded.chats",
            params![account, json],
        ).await.map_err(|_| ChatError::Storage)?;

        Ok(())
    }

    async fn connect(&self) -> Result<libsql::Connection, ChatError> {
        let path = self.path.to_str().ok_or(ChatError::Storage)?;
        let database = Builder::new_local(path)
            .build()
            .await
            .map_err(|_| ChatError::Storage)?;
        let connection = database.connect().map_err(|_| ChatError::Storage)?;
        connection.execute("CREATE TABLE IF NOT EXISTS selected_chats (account_id INTEGER PRIMARY KEY, chats TEXT NOT NULL)", ())
            .await.map_err(|_| ChatError::Storage)?;

        Ok(connection)
    }
}
