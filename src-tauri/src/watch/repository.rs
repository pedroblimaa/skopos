use crate::app_message::AppMessage;
use libsql::{params, Builder, TransactionBehavior};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateWatch {
    pub phrases: Vec<String>,
    pub max_price_cents: Option<i64>,
    pub min_price_cents: Option<i64>,
}

#[derive(Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Watch {
    pub id: i64,
    pub phrases: Vec<String>,
    pub max_price_cents: Option<i64>,
    pub min_price_cents: Option<i64>,
}

// NULL keeps the automatic minimum responsive to future maximum-price edits.
impl Watch {
    pub(crate) fn minimum_price_cents(&self) -> i64 {
        self.min_price_cents
            .unwrap_or_else(|| self.max_price_cents.unwrap_or(0) / 5)
    }
}

#[derive(Debug)]
pub enum WatchError {
    InvalidPhrase,
    InvalidPrice,
    Storage,
    NotFound,
}

impl WatchError {
    pub fn message(&self) -> AppMessage {
        match self {
            Self::InvalidPhrase => AppMessage::InvalidPhrase,
            Self::InvalidPrice => AppMessage::InvalidPrice,
            Self::NotFound => AppMessage::ProductNotFound,
            Self::Storage => AppMessage::WatchStorage,
        }
    }
}

pub struct WatchRepository {
    path: PathBuf,
}

impl WatchRepository {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub async fn create(&self, input: CreateWatch) -> Result<Watch, WatchError> {
        let phrases = validate(&input)?;

        let connection = self.connect().await?;
        let phrases_json = serde_json::to_string(&phrases).map_err(|_| WatchError::Storage)?;
        connection
            .execute(
                "INSERT INTO watches (phrases, max_price_cents, min_price_cents) VALUES (?1, ?2, ?3)",
                params![phrases_json, input.max_price_cents, input.min_price_cents],
            )
            .await
            .map_err(|_| WatchError::Storage)?;

        Ok(Watch {
            id: connection.last_insert_rowid(),
            phrases,
            max_price_cents: input.max_price_cents,
            min_price_cents: input.min_price_cents,
        })
    }

    pub async fn update(&self, id: i64, input: CreateWatch) -> Result<Watch, WatchError> {
        let phrases = validate(&input)?;
        let connection = self.connect().await?;
        let phrases_json = serde_json::to_string(&phrases).map_err(|_| WatchError::Storage)?;
        let changed = connection
            .execute(
                "UPDATE watches SET phrases = ?1, max_price_cents = ?2, min_price_cents = ?3 WHERE id = ?4",
                params![phrases_json, input.max_price_cents, input.min_price_cents, id],
            )
            .await
            .map_err(|_| WatchError::Storage)?;
        if changed == 0 {
            return Err(WatchError::NotFound);
        }

        Ok(Watch {
            id,
            phrases,
            max_price_cents: input.max_price_cents,
            min_price_cents: input.min_price_cents,
        })
    }

    pub async fn delete(&self, id: i64) -> Result<(), WatchError> {
        let connection = self.connect().await?;
        connection
            .execute("DELETE FROM watches WHERE id = ?1", params![id])
            .await
            .map_err(|_| WatchError::Storage)?;
        Ok(())
    }

    pub async fn list(&self) -> Result<Vec<Watch>, WatchError> {
        let connection = self.connect().await?;
        let mut rows = connection
            .query(
                "SELECT id, phrases, max_price_cents, min_price_cents FROM watches ORDER BY id DESC",
                (),
            )
            .await
            .map_err(|_| WatchError::Storage)?;
        let mut watches = Vec::new();

        while let Some(row) = rows.next().await.map_err(|_| WatchError::Storage)? {
            let phrases_json: String = row.get(1).map_err(|_| WatchError::Storage)?;
            watches.push(Watch {
                id: row.get(0).map_err(|_| WatchError::Storage)?,
                phrases: serde_json::from_str(&phrases_json).map_err(|_| WatchError::Storage)?,
                max_price_cents: row.get(2).map_err(|_| WatchError::Storage)?,
                min_price_cents: row.get(3).map_err(|_| WatchError::Storage)?,
            });
        }

        Ok(watches)
    }

    async fn connect(&self) -> Result<libsql::Connection, WatchError> {
        let path = self.path.to_str().ok_or(WatchError::Storage)?;
        let database = Builder::new_local(path)
            .build()
            .await
            .map_err(|_| WatchError::Storage)?;
        let connection = database.connect().map_err(|_| WatchError::Storage)?;
        connection
            .busy_timeout(std::time::Duration::from_secs(1))
            .map_err(|_| WatchError::Storage)?;
        connection
            .execute(
                "CREATE TABLE IF NOT EXISTS watches (id INTEGER PRIMARY KEY, phrases TEXT NOT NULL, max_price_cents INTEGER, min_price_cents INTEGER)",
                (),
            )
            .await
            .map_err(|_| WatchError::Storage)?;
        migrate_minimum(&connection)
            .await
            .map_err(|_| WatchError::Storage)?;
        Ok(connection)
    }
}

fn validate(input: &CreateWatch) -> Result<Vec<String>, WatchError> {
    let phrases: Vec<_> = input
        .phrases
        .iter()
        .map(|phrase| phrase.trim().to_owned())
        .collect();
    if phrases.is_empty() || phrases.iter().any(String::is_empty) {
        return Err(WatchError::InvalidPhrase);
    }
    if input.max_price_cents.is_some_and(|price| price <= 0)
        || input.min_price_cents.is_some_and(|price| {
            price < 0 || input.max_price_cents.is_some_and(|maximum| price > maximum)
        })
    {
        return Err(WatchError::InvalidPrice);
    }

    Ok(phrases)
}

async fn migrate_minimum(connection: &libsql::Connection) -> Result<(), libsql::Error> {
    if has_minimum_column(connection).await? {
        return Ok(());
    }
    // Concurrent startup reads can both discover the old schema before it is upgraded.
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .await?;
    if !has_minimum_column(&transaction).await? {
        transaction
            .execute("ALTER TABLE watches ADD COLUMN min_price_cents INTEGER", ())
            .await?;
    }
    transaction.commit().await
}

async fn has_minimum_column(connection: &libsql::Connection) -> Result<bool, libsql::Error> {
    let mut columns = connection.query("PRAGMA table_info(watches)", ()).await?;
    while let Some(column) = columns.next().await? {
        if column.get::<String>(1)? == "min_price_cents" {
            return Ok(true);
        }
    }
    Ok(false)
}
