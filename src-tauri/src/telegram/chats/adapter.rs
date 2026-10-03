use super::{
    mapping,
    photos::{self, PhotoLocation},
    Chat, ChatError,
};
use crate::app_message::AppMessage;
use crate::telegram::{api::TelegramApi, error::AuthError};
use grammers_client::tl;
use std::collections::{BTreeMap, HashMap};

pub(in crate::telegram) async fn account_id(api: &TelegramApi) -> Result<i64, ChatError> {
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
    let page = load_dialogs(api).await?;
    Ok((page.chats, page.photos))
}

pub(in crate::telegram) struct Dialogs {
    pub chats: Vec<Chat>,
    pub peers: HashMap<String, tl::enums::InputPeer>,
    photos: HashMap<String, PhotoLocation>,
}

pub(in crate::telegram) async fn load_dialogs(api: &TelegramApi) -> Result<Dialogs, ChatError> {
    let mut chats = BTreeMap::new();
    let mut locations = HashMap::new();
    let mut input_peers = HashMap::new();

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
            collect_input_peers(&peers, &mut input_peers)?;

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

    Ok(Dialogs {
        chats: chats.into_values().collect(),
        peers: input_peers,
        photos: locations,
    })
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

fn collect_input_peers(
    peers: &[tl::enums::Chat],
    input_peers: &mut HashMap<String, tl::enums::InputPeer>,
) -> Result<(), ChatError> {
    for peer in peers {
        if let Some(chat) = mapping::chat(peer) {
            let input = match peer {
                tl::enums::Chat::Chat(chat) => tl::types::InputPeerChat { chat_id: chat.id }.into(),
                tl::enums::Chat::Channel(channel) => tl::types::InputPeerChannel {
                    channel_id: channel.id,
                    access_hash: channel.access_hash.ok_or(ChatError::IncompleteList)?,
                }
                .into(),
                _ => continue,
            };
            input_peers.insert(chat.id, input);
        }
    }

    Ok(())
}
