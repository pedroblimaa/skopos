use super::{adapter, SearchError, SearchState};
use crate::{
    promotion::{match_messages, SearchSummary, SourceMessage},
    telegram::{
        api::TelegramApi,
        chats::{adapter as chats, Chat},
        AuthState,
    },
    watch::Watch,
};
use grammers_client::tl;
use std::{
    collections::{HashMap, HashSet},
    sync::atomic::Ordering,
};

pub(super) struct SearchRun<'a> {
    pub api: &'a TelegramApi,
    pub auth: &'a AuthState,
    pub state: &'a SearchState,
    pub generation: u64,
    pub cancellation: u64,
    pub since: i64,
    pub until: i64,
    pub saved_messages: HashSet<(String, i32)>,
    pub cached_photos: HashMap<(String, i32), String>,
    pub checkpoints: HashMap<String, crate::telegram::monitoring::Checkpoint>,
}

impl SearchRun<'_> {
    pub async fn ensure_current(&self) -> Result<(), SearchError> {
        let login = self.auth.login.lock().await;

        if login.generation != self.generation
            || self.state.cancellation.load(Ordering::SeqCst) != self.cancellation
        {
            return Err(SearchError::Cancelled);
        }

        Ok(())
    }
}

pub(super) async fn search(
    run: &SearchRun<'_>,
    selected: Vec<Chat>,
    watches: &[Watch],
) -> Result<
    (
        Vec<SourceMessage>,
        SearchSummary,
        HashMap<String, crate::telegram::monitoring::Checkpoint>,
    ),
    SearchError,
> {
    let mut checkpoints = HashMap::new();
    let mut summary = SearchSummary {
        started_at: run.until,
        since: run.since,
        total_chats: selected.len(),
        ..SearchSummary::default()
    };
    let dialogs = match tokio::time::timeout(
        std::time::Duration::from_secs(30),
        chats::load_dialogs(run.api),
    )
    .await
    {
        Ok(Ok(dialogs)) => dialogs,
        Ok(Err(error)) => {
            summary.failure = Some(error.message());

            return Ok((Vec::new(), summary, checkpoints));
        }

        Err(_) => {
            summary.failure = Some(crate::app_message::AppMessage::SearchFailed);

            return Ok((Vec::new(), summary, checkpoints));
        }
    };
    let current: HashMap<_, _> = dialogs
        .chats
        .into_iter()
        .map(|chat| (chat.id.clone(), chat))
        .collect();
    let mut found = Vec::new();

    for selected_chat in selected {
        run.ensure_current().await?;
        let Some(chat) = current.get(&selected_chat.id) else {
            summary.unavailable_chats.push(selected_chat.title);
            continue;
        };
        let Some(peer) = dialogs.peers.get(&chat.id) else {
            summary.unavailable_chats.push(chat.title.clone());
            continue;
        };

        match search_chat(run, peer.clone(), chat, watches).await {
            Ok((messages, checkpoint)) => {
                checkpoints.insert(chat.id.clone(), checkpoint);
                found.extend(messages);
                summary.completed_chats += 1;
            }

            Err(error) if error.is_unavailable() => {
                summary.unavailable_chats.push(chat.title.clone())
            }

            Err(SearchError::Cancelled) => return Err(SearchError::Cancelled),
            Err(error) => {
                summary.failure = Some(error.message());
                break;
            }
        }
    }

    run.ensure_current().await?;

    Ok((found, summary, checkpoints))
}

async fn search_chat(
    run: &SearchRun<'_>,
    peer: tl::enums::InputPeer,
    chat: &Chat,
    watches: &[Watch],
) -> Result<(Vec<SourceMessage>, crate::telegram::monitoring::Checkpoint), SearchError> {
    let (messages, checkpoint) = adapter::history(run, peer, chat, watches).await?;
    let matches = match_messages(&messages, watches);
    let ids: std::collections::HashSet<_> = matches
        .iter()
        .map(|result| result.message.message_id)
        .collect();

    Ok((
        messages
            .into_iter()
            .filter(|message| {
                ids.contains(&message.message_id)
                    || run
                        .saved_messages
                        .contains(&(message.chat_id.clone(), message.message_id))
            })
            .collect(),
        checkpoint,
    ))
}
