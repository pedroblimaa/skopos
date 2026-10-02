use super::{
    mapping,
    photos::{self, PhotoLocation},
    Chat, ChatError,
};
use crate::app_message::AppMessage;
use crate::telegram::{api::TelegramApi, error::AuthError};
use grammers_client::tl;
use std::collections::{BTreeMap, HashMap};

pub(super) async fn account_id(api: &TelegramApi) -> Result<i64, ChatError> {
    let users = api
        .invoke(&tl::functions::users::GetUsers {
            id: vec![tl::enums::InputUser::UserSelf],
        })
        .await?;

    match users.into_iter().next() {
        Some(tl::enums::User::User(user)) if user.is_self => Ok(user.id),
        _ => Err(ChatError::Auth(AuthError::Message(
            AppMessage::RestartLogin,
        ))),
    }
}

pub(super) async fn list(
    api: &TelegramApi,
) -> Result<(Vec<Chat>, HashMap<String, PhotoLocation>), ChatError> {
    let mut chats = BTreeMap::new();
    let mut locations = HashMap::new();

    for folder in [0, 1] {
        let mut request = tl::functions::messages::GetDialogs {
            exclude_pinned: false,
            folder_id: Some(folder),
            offset_date: 0,
            offset_id: 0,
            offset_peer: tl::enums::InputPeer::Empty,
            limit: 100,
            hash: 0,
        };
        let mut previous_cursor = None;

        loop {
            let response = api.invoke(&request).await?;
            let (dialogs, messages, users, peers, complete) = match response {
                tl::enums::messages::Dialogs::Dialogs(page) => {
                    (page.dialogs, page.messages, page.users, page.chats, true)
                }
                tl::enums::messages::Dialogs::Slice(page) => {
                    let complete = page.dialogs.len() < request.limit as usize;
                    (
                        page.dialogs,
                        page.messages,
                        page.users,
                        page.chats,
                        complete,
                    )
                }
                tl::enums::messages::Dialogs::NotModified(_) => {
                    return Err(ChatError::IncompleteList)
                }
            };

            collect_page(&dialogs, &peers, &mut chats, &mut locations);

            if complete || dialogs.is_empty() {
                break;
            }

            let cursor = mapping::advance(&mut request, &dialogs, &messages, &users, &peers)?;
            if previous_cursor.as_ref() == Some(&cursor) {
                return Err(ChatError::IncompleteList);
            }
            previous_cursor = Some(cursor);
        }
    }

    Ok((chats.into_values().collect(), locations))
}

fn collect_page(
    dialogs: &[tl::enums::Dialog],
    peers: &[tl::enums::Chat],
    chats: &mut BTreeMap<String, Chat>,
    locations: &mut HashMap<String, PhotoLocation>,
) {
    for peer in peers {
        let Some(chat) = mapping::chat(peer) else {
            continue;
        };
        if !dialogs
            .iter()
            .any(|dialog| mapping::contains(dialog, &chat.id))
        {
            continue;
        }

        if let Some(location) = photos::location(peer) {
            locations.insert(chat.id.clone(), location);
        }
        chats.insert(chat.id.clone(), chat);
    }
}
