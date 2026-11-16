use crate::{app_message::AppMessage, promotion::ProductMatch};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Deserialize, Serialize)]
pub(crate) struct Checkpoint {
    pub message_id: i32,
    pub checked_at: i64,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct Record {
    pub enabled: bool,
    pub days: HashMap<String, Slots>,
    pub checkpoints: HashMap<String, Checkpoint>,
    pub pending: Vec<ProductMatch>,
    pub pending_day: String,
    pub last_attempt: Option<i64>,
    pub failure: Option<AppMessage>,
}

impl Default for Record {
    fn default() -> Self {
        Self {
            enabled: true,
            days: HashMap::new(),
            checkpoints: HashMap::new(),
            pending: Vec::new(),
            pending_day: String::new(),
            last_attempt: None,
            failure: None,
        }
    }
}

#[derive(Clone, Default, Deserialize, Serialize)]
pub(crate) struct Slots {
    pub(super) launch: bool,
    pub(super) evening: bool,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Status {
    pub account_id: i64,
    pub enabled: bool,
    pub is_running: bool,
    pub last_attempt: Option<i64>,
    pub next_due: Option<i64>,
    pub failure: Option<AppMessage>,
}
