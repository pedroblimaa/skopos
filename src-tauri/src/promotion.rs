mod matcher;
mod repository;
#[cfg(test)]
mod tests;

pub(crate) use matcher::match_messages;
pub(crate) use repository::ResultRepository;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceMessage {
    pub chat_id: String,
    pub chat_title: String,
    pub message_id: i32,
    pub posted_at: i64,
    pub text: String,
    pub message_link: Option<String>,
    #[serde(default)]
    pub image: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProductMatch {
    pub watch_id: i64,
    pub price_cents: Option<i64>,
    pub message: SourceMessage,
}

#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SearchSummary {
    pub started_at: i64,
    pub since: i64,
    pub completed_chats: usize,
    pub total_chats: usize,
    pub unavailable_chats: Vec<String>,
    pub failure: Option<crate::app_message::AppMessage>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SearchResults {
    pub matches: Vec<ProductMatch>,
    pub summary: Option<SearchSummary>,
}
