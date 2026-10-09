use super::{SearchSummary, SourceMessage};
use libsql::{params, Builder};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Debug)]
pub(crate) struct StorageError;

pub(crate) struct ResultRepository {
    path: PathBuf,
}

impl ResultRepository {
    pub(crate) fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub(crate) async fn load(
        &self,
        account: i64,
    ) -> Result<(Vec<SourceMessage>, Option<SearchSummary>), StorageError> {
        let connection = self.connect().await?;
        let mut rows = connection
            .query(
                "SELECT messages, summary FROM search_results WHERE account_id = ?1",
                [account],
            )
            .await
            .map_err(|_| StorageError)?;
        let Some(row) = rows.next().await.map_err(|_| StorageError)? else {
            return Ok((Vec::new(), None));
        };
        let messages: String = row.get(0).map_err(|_| StorageError)?;
        let summary: Option<String> = row.get(1).map_err(|_| StorageError)?;

        Ok((
            serde_json::from_str(&messages).map_err(|_| StorageError)?,
            summary
                .map(|json| serde_json::from_str(&json))
                .transpose()
                .map_err(|_| StorageError)?,
        ))
    }

    pub(crate) async fn merge_monitoring(
        &self,
        account: i64,
        messages: Vec<SourceMessage>,
        summary: SearchSummary,
        monitoring_data: &str,
    ) -> Result<(), StorageError> {
        let messages = serde_json::to_string(&self.merged(account, messages).await?)
            .map_err(|_| StorageError)?;
        let summary = serde_json::to_string(&summary).map_err(|_| StorageError)?;
        let connection = self.connect().await?;
        connection.execute("CREATE TABLE IF NOT EXISTS monitoring (account_id INTEGER PRIMARY KEY, data TEXT NOT NULL)", ()).await.map_err(|_| StorageError)?;
        let transaction = connection.transaction().await.map_err(|_| StorageError)?;
        transaction.execute("INSERT INTO search_results (account_id, messages, summary) VALUES (?1, ?2, ?3) ON CONFLICT(account_id) DO UPDATE SET messages = excluded.messages, summary = excluded.summary", params![account, messages, summary]).await.map_err(|_| StorageError)?;
        transaction.execute("INSERT INTO monitoring (account_id, data) VALUES (?1, ?2) ON CONFLICT(account_id) DO UPDATE SET data = excluded.data", params![account, monitoring_data]).await.map_err(|_| StorageError)?;
        transaction.commit().await.map_err(|_| StorageError)?;

        Ok(())
    }

    async fn merged(
        &self,
        account: i64,
        messages: Vec<SourceMessage>,
    ) -> Result<Vec<SourceMessage>, StorageError> {
        let (saved, _) = self.load(account).await?;
        let mut merged: BTreeMap<_, _> = saved
            .into_iter()
            .map(|message| ((message.chat_id.clone(), message.message_id), message))
            .collect();

        for mut message in messages {
            let key = (message.chat_id.clone(), message.message_id);

            if message.image.is_none() {
                message.image = merged.get(&key).and_then(|saved| saved.image.clone());
            }

            merged.insert(key, message);
        }

        Ok(merged.into_values().collect())
    }

    pub(crate) async fn clear(
        &self,
        account: i64,
        before: Option<i64>,
    ) -> Result<(), StorageError> {
        let (mut messages, summary) = self.load(account).await?;
        let summary = if let Some(before) = before {
            messages.retain(|message| message.posted_at >= before);
            summary
        } else {
            messages.clear();
            None
        };

        self.save(account, messages, summary).await
    }

    async fn save(
        &self,
        account: i64,
        messages: Vec<SourceMessage>,
        summary: Option<SearchSummary>,
    ) -> Result<(), StorageError> {
        let messages = serde_json::to_string(&messages).map_err(|_| StorageError)?;
        let summary = summary
            .map(|summary| serde_json::to_string(&summary))
            .transpose()
            .map_err(|_| StorageError)?;
        let connection = self.connect().await?;

        // The account's message set and summary commit together in one SQLite statement.
        connection.execute("INSERT INTO search_results (account_id, messages, summary) VALUES (?1, ?2, ?3) ON CONFLICT(account_id) DO UPDATE SET messages = excluded.messages, summary = excluded.summary", params![account, messages, summary]).await.map_err(|_| StorageError)?;

        Ok(())
    }

    async fn connect(&self) -> Result<libsql::Connection, StorageError> {
        let database = Builder::new_local(self.path.to_str().ok_or(StorageError)?)
            .build()
            .await
            .map_err(|_| StorageError)?;
        let connection = database.connect().map_err(|_| StorageError)?;

        connection.execute("CREATE TABLE IF NOT EXISTS search_results (account_id INTEGER PRIMARY KEY, messages TEXT NOT NULL, summary TEXT)", ()).await.map_err(|_| StorageError)?;

        Ok(connection)
    }
}
