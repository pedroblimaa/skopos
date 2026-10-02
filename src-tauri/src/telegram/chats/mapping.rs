use super::{Chat, ChatError, ChatKind};
use grammers_client::tl;

pub(super) fn chat(peer: &tl::enums::Chat) -> Option<Chat> {
    match peer {
        tl::enums::Chat::Chat(chat)
            if !chat.left && !chat.deactivated && chat.migrated_to.is_none() =>
        {
            Some(Chat {
                id: format!("chat:{}", chat.id),
                title: chat.title.clone(),
                kind: ChatKind::Group,
                username: None,
                available: true,
            })
        }
        tl::enums::Chat::Channel(channel)
            if !channel.left && channel.access_hash.is_some()
                && !channel.banned_rights.as_ref().is_some_and(|rights| matches!(rights, tl::enums::ChatBannedRights::Rights(rights) if rights.view_messages)) => {
            Some(Chat {
                id: format!("channel:{}", channel.id),
                title: channel.title.clone(),
                kind: if channel.broadcast {
                    ChatKind::Channel
                } else {
                    ChatKind::Group
                },
                username: channel.username.clone(),
                available: true,
            })
        }
        _ => None,
    }
}

pub(super) fn contains(dialog: &tl::enums::Dialog, id: &str) -> bool {
    matches!(dialog, tl::enums::Dialog::Dialog(dialog) if peer_id(&dialog.peer) == id)
}

pub(super) fn advance(
    request: &mut tl::functions::messages::GetDialogs,
    dialogs: &[tl::enums::Dialog],
    messages: &[tl::enums::Message],
    users: &[tl::enums::User],
    chats: &[tl::enums::Chat],
) -> Result<(String, i32, i32), ChatError> {
    let dialog = dialogs
        .iter()
        .rev()
        .find_map(|dialog| match dialog {
            tl::enums::Dialog::Dialog(dialog) => Some(dialog),
            _ => None,
        })
        .ok_or(ChatError::IncompleteList)?;
    let id = peer_id(&dialog.peer);
    let date = messages
        .iter()
        .find_map(|message| match message {
            tl::enums::Message::Message(message)
                if message.id == dialog.top_message && peer_id(&message.peer_id) == id =>
            {
                Some(message.date)
            }
            tl::enums::Message::Service(message)
                if message.id == dialog.top_message && peer_id(&message.peer_id) == id =>
            {
                Some(message.date)
            }
            _ => None,
        })
        .ok_or(ChatError::IncompleteList)?;

    request.offset_peer = input_peer(&dialog.peer, users, chats)?;
    request.offset_id = dialog.top_message;
    request.offset_date = date;
    request.exclude_pinned = true;

    Ok((id, dialog.top_message, date))
}

fn input_peer(
    peer: &tl::enums::Peer,
    users: &[tl::enums::User],
    chats: &[tl::enums::Chat],
) -> Result<tl::enums::InputPeer, ChatError> {
    match peer {
        tl::enums::Peer::Chat(peer) => Ok(tl::types::InputPeerChat {
            chat_id: peer.chat_id,
        }
        .into()),
        tl::enums::Peer::User(peer) => users
            .iter()
            .find_map(|user| match user {
                tl::enums::User::User(user) if user.id == peer.user_id => user_input_peer(user),
                _ => None,
            })
            .ok_or(ChatError::IncompleteList),
        tl::enums::Peer::Channel(peer) => chats
            .iter()
            .find_map(|chat| match chat {
                tl::enums::Chat::Channel(channel) if channel.id == peer.channel_id => Some(
                    tl::types::InputPeerChannel {
                        channel_id: channel.id,
                        access_hash: channel.access_hash?,
                    }
                    .into(),
                ),
                tl::enums::Chat::ChannelForbidden(channel) if channel.id == peer.channel_id => {
                    Some(
                        tl::types::InputPeerChannel {
                            channel_id: channel.id,
                            access_hash: channel.access_hash,
                        }
                        .into(),
                    )
                }
                _ => None,
            })
            .ok_or(ChatError::IncompleteList),
    }
}

fn user_input_peer(user: &tl::types::User) -> Option<tl::enums::InputPeer> {
    if user.is_self {
        return Some(tl::enums::InputPeer::PeerSelf);
    }

    Some(
        tl::types::InputPeerUser {
            user_id: user.id,
            access_hash: user.access_hash?,
        }
        .into(),
    )
}

fn peer_id(peer: &tl::enums::Peer) -> String {
    match peer {
        tl::enums::Peer::Chat(peer) => format!("chat:{}", peer.chat_id),
        tl::enums::Peer::Channel(peer) => format!("channel:{}", peer.channel_id),
        tl::enums::Peer::User(peer) => format!("user:{}", peer.user_id),
    }
}
