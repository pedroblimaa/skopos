use super::{photos, workflow::SearchRun, SearchError};
use crate::{
    promotion::{match_messages, SourceMessage},
    telegram::chats::Chat,
    watch::Watch,
};
use grammers_client::tl;

pub(super) async fn history(
    run: &SearchRun<'_>,
    peer: tl::enums::InputPeer,
    chat: &Chat,
    watches: &[Watch],
) -> Result<Vec<SourceMessage>, SearchError> {
    let mut request = tl::functions::messages::GetHistory {
        peer,
        offset_id: 0,
        offset_date: 0,
        add_offset: 0,
        limit: 100,
        max_id: 0,
        min_id: 0,
        hash: 0,
    };
    let mut messages = Vec::new();

    loop {
        run.ensure_current().await?;
        let response =
            tokio::time::timeout(std::time::Duration::from_secs(30), run.api.invoke(&request))
                .await
                .map_err(|_| SearchError::Incomplete)??;
        let page = match response {
            tl::enums::messages::Messages::Messages(page) => page.messages,
            tl::enums::messages::Messages::Slice(page) => page.messages,
            tl::enums::messages::Messages::ChannelMessages(page) => page.messages,
            _ => return Err(SearchError::Incomplete),
        };
        if page.is_empty() {
            break;
        }

        let mut oldest = i32::MAX;
        let mut reached_cutoff = false;
        for entry in page {
            let (id, date) = match &entry {
                tl::enums::Message::Message(message) => (message.id, Some(message.date)),
                tl::enums::Message::Service(message) => (message.id, Some(message.date)),
                tl::enums::Message::Empty(message) => (message.id, None),
            };
            oldest = oldest.min(id);
            reached_cutoff |= date.is_some_and(|date| i64::from(date) < run.since);

            let tl::enums::Message::Message(message) = entry else {
                continue;
            };
            let posted_at = i64::from(message.date);
            if posted_at < run.since || posted_at > run.until || message.message.trim().is_empty() {
                continue;
            }

            let mut source = SourceMessage {
                chat_id: chat.id.clone(),
                chat_title: chat.title.clone(),
                message_id: message.id,
                posted_at,
                text: message.message,
                message_link: message_link(chat, message.id),
                image: None,
            };
            if !match_messages(std::slice::from_ref(&source), watches).is_empty() {
                source.image = photos::download(run.api, message.media).await;
                run.ensure_current().await?;
            }
            messages.push(source);
        }
        if reached_cutoff {
            break;
        }
        if oldest <= 0 || (request.offset_id != 0 && oldest >= request.offset_id) {
            return Err(SearchError::Incomplete);
        }

        request.offset_id = oldest;
    }
    Ok(messages)
}

fn message_link(chat: &Chat, id: i32) -> Option<String> {
    if let Some(username) = &chat.username {
        return Some(format!("https://t.me/{username}/{id}"));
    }
    chat.id
        .strip_prefix("channel:")
        .map(|channel| format!("https://t.me/c/{channel}/{id}"))
}
