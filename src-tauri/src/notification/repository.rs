use super::{DeliveryStatus, Settings};
use crate::{app_message::AppMessage, promotion::SourceMessage};
use libsql::{params, Builder};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Clone, Deserialize, Serialize, PartialEq, Eq)]
pub(super) struct Signature {
    pub watch_id: i64,
    pub price: Option<i64>,
    pub link: Option<String>,
}

#[derive(Clone, Deserialize, Serialize)]
pub(super) struct Item {
    #[serde(default)]
    pub telegram_signatures: Vec<Signature>,
    #[serde(default)]
    pub desktop_signatures: Vec<Signature>,
    pub message: SourceMessage,
    pub price: Option<i64>,
    pub title: String,
    pub telegram: String,
    pub desktop: bool,
}

#[derive(Deserialize, Serialize)]
pub(super) struct Separator {
    pub day: String,
    pub telegram: String,
}

#[derive(Default, Deserialize, Serialize)]
pub(super) struct Record {
    #[serde(default)]
    pub last_telegram: HashMap<i64, Signature>,
    #[serde(default)]
    pub last_desktop: HashMap<i64, Signature>,
    pub settings: Settings,
    pub items: Vec<Item>,
    pub failure: Option<AppMessage>,
    pub next_attempt_at: u64,
    #[serde(default)]
    pub delivery_day: Option<String>,
    #[serde(default)]
    pub separator: Option<Separator>,
}

impl Record {
    pub fn status(&self, failure: Option<AppMessage>) -> DeliveryStatus {
        DeliveryStatus {
            pending: self
                .items
                .iter()
                .filter(|item| item.telegram == "pending")
                .count(),
            uncertain: self
                .items
                .iter()
                .filter(|item| item.telegram == "uncertain")
                .count()
                + usize::from(
                    self.separator
                        .as_ref()
                        .is_some_and(|separator| separator.telegram == "uncertain"),
                ),
            failure: failure.or_else(|| self.failure.clone()),
        }
    }
}

pub(super) struct Repository(pub PathBuf);

impl Repository {
    pub async fn load(&self, account: i64) -> Result<Record, AppMessage> {
        let connection = self.connect().await?;
        let mut rows = connection
            .query(
                "SELECT data FROM notifications WHERE account_id = ?1",
                [account],
            )
            .await
            .map_err(|_| AppMessage::NotificationStorage)?;
        let Some(row) = rows
            .next()
            .await
            .map_err(|_| AppMessage::NotificationStorage)?
        else {
            return Ok(Record::default());
        };
        let data: String = row.get(0).map_err(|_| AppMessage::NotificationStorage)?;

        decode(&data)
    }

    pub async fn save(&self, account: i64, record: &Record) -> Result<(), AppMessage> {
        let data = serde_json::to_string(record).map_err(|_| AppMessage::NotificationStorage)?;
        self.connect().await?.execute("INSERT INTO notifications (account_id, data) VALUES (?1, ?2) ON CONFLICT(account_id) DO UPDATE SET data = excluded.data", params![account, data]).await.map_err(|_| AppMessage::NotificationStorage)?;

        Ok(())
    }

    async fn connect(&self) -> Result<libsql::Connection, AppMessage> {
        let database = Builder::new_local(self.0.to_str().ok_or(AppMessage::NotificationStorage)?)
            .build()
            .await
            .map_err(|_| AppMessage::NotificationStorage)?;
        let connection = database
            .connect()
            .map_err(|_| AppMessage::NotificationStorage)?;
        connection.execute("CREATE TABLE IF NOT EXISTS notifications (account_id INTEGER PRIMARY KEY, data TEXT NOT NULL)", ()).await.map_err(|_| AppMessage::NotificationStorage)?;

        Ok(connection)
    }
}

pub(super) fn decode(data: &str) -> Result<Record, AppMessage> {
    let mut value: serde_json::Value =
        serde_json::from_str(data).map_err(|_| AppMessage::NotificationStorage)?;

    if matches!(
        value["failure"]["code"].as_str(),
        Some("notificationToken" | "notificationStartBot")
    ) {
        value["failure"] = serde_json::Value::Null;
    }

    serde_json::from_value(value).map_err(|_| AppMessage::NotificationStorage)
}
