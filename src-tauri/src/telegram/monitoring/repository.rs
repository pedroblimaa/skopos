use super::Record;
use crate::app_message::AppMessage;
use libsql::{params, Builder};
use std::path::PathBuf;

pub(crate) struct Repository(pub PathBuf);

impl Repository {
    pub(crate) async fn load(&self, account: i64) -> Result<Record, AppMessage> {
        let connection = self.connect().await?;
        let mut rows = connection
            .query(
                "SELECT data FROM monitoring WHERE account_id = ?1",
                [account],
            )
            .await
            .map_err(|_| AppMessage::MonitoringStorage)?;
        let Some(row) = rows
            .next()
            .await
            .map_err(|_| AppMessage::MonitoringStorage)?
        else {
            return Ok(Record::default());
        };
        let data: String = row.get(0).map_err(|_| AppMessage::MonitoringStorage)?;
        serde_json::from_str(&data).map_err(|_| AppMessage::MonitoringStorage)
    }

    pub(crate) async fn save(&self, account: i64, record: &Record) -> Result<(), AppMessage> {
        let data = serde_json::to_string(record).map_err(|_| AppMessage::MonitoringStorage)?;
        self.connect().await?.execute(
            "INSERT INTO monitoring (account_id, data) VALUES (?1, ?2) ON CONFLICT(account_id) DO UPDATE SET data = excluded.data",
            params![account, data],
        ).await.map_err(|_| AppMessage::MonitoringStorage)?;

        Ok(())
    }

    async fn connect(&self) -> Result<libsql::Connection, AppMessage> {
        let database = Builder::new_local(self.0.to_str().ok_or(AppMessage::MonitoringStorage)?)
            .build()
            .await
            .map_err(|_| AppMessage::MonitoringStorage)?;
        let connection = database
            .connect()
            .map_err(|_| AppMessage::MonitoringStorage)?;
        connection.execute("CREATE TABLE IF NOT EXISTS monitoring (account_id INTEGER PRIMARY KEY, data TEXT NOT NULL)", ())
            .await.map_err(|_| AppMessage::MonitoringStorage)?;

        Ok(connection)
    }
}
