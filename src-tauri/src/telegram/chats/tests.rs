use super::*;
use crate::telegram::e2e::{chats as fixtures, test_context};
use grammers_client::{tl, InvocationError};
use tl::Serializable;

#[tokio::test]
async fn downloads_jpeg_photos_and_rejects_invalid_or_oversized_files() {
    let (context, fixture) = test_context().await;
    let mut group = fixtures::group(1);
    group.photo = tl::types::ChatPhoto {
        has_video: false,
        photo_id: 42,
        stripped_thumb: None,
        dc_id: 2,
    }
    .into();
    let location = photos::location(&group.into()).unwrap();
    let file = |bytes| {
        tl::enums::upload::File::from(tl::types::upload::File {
            r#type: tl::enums::storage::FileType::FileJpeg,
            mtime: 0,
            bytes,
        })
        .to_bytes()
    };
    fixture
        .0
        .lock()
        .unwrap()
        .replies
        .push_back(Ok(file(vec![0xff, 0xd8])));

    assert_eq!(
        photos::download(&context.client, location.clone())
            .await
            .unwrap(),
        "data:image/jpeg;base64,/9g="
    );

    fixture
        .0
        .lock()
        .unwrap()
        .replies
        .push_back(Ok(file(vec![0, 1])));

    assert!(matches!(
        photos::download(&context.client, location.clone()).await,
        Err(ChatError::IncompleteList)
    ));
    fixture
        .0
        .lock()
        .unwrap()
        .replies
        .extend((0..5).map(|_| Ok(file(vec![1; 64 * 1024]))));

    assert!(matches!(
        photos::download(&context.client, location).await,
        Err(ChatError::Telegram(InvocationError::Dropped))
    ));
    assert!(photos::location(&fixtures::group(1).into()).is_none());
    assert!(photos::location(&tl::types::ChatEmpty { id: 1 }.into()).is_none());
}

#[test]
fn translates_structured_errors_at_the_boundary() {
    use crate::app_message::AppMessage;
    use grammers_client::sender::RpcError;
    let rpc = |code, name: &str, value| {
        ChatError::Telegram(InvocationError::Rpc(RpcError {
            code,
            name: name.into(),
            value,
            caused_by: None,
        }))
    };

    assert!(matches!(
        rpc(401, "AUTH_KEY_UNREGISTERED", None).message(),
        AppMessage::RestartLogin
    ));
    assert!(matches!(
        rpc(420, "FLOOD_WAIT", Some(30)).message(),
        AppMessage::ChatRateLimitSeconds { seconds: 30 }
    ));
    assert!(matches!(
        rpc(420, "FLOOD_WAIT", None).message(),
        AppMessage::ChatRateLimit
    ));
    assert!(matches!(
        ChatError::Storage.message(),
        AppMessage::ChatStorage
    ));
    assert!(matches!(
        ChatError::InvalidSelection.message(),
        AppMessage::InvalidChatSelection
    ));
    assert!(matches!(
        ChatError::IncompleteList.message(),
        AppMessage::ChatListIncomplete
    ));
}

#[tokio::test]
async fn includes_archived_channels_and_resolves_account_in_rust() {
    let (context, fixture) = test_context().await;
    fixture.0.lock().unwrap().scenario.authorized = true;
    fixture.0.lock().unwrap().scenario.account_id = Some(9007199254740993);

    assert_eq!(
        adapter::account_id(&context.client).await.unwrap(),
        9007199254740993
    );

    let (chats, _) = adapter::list(&context.client).await.unwrap();

    assert_eq!(
        chats
            .iter()
            .map(|chat| chat.id.as_str())
            .collect::<Vec<_>>(),
        ["channel:2", "chat:1"]
    );
    assert_eq!(chats[0].username.as_deref(), Some("channel2"));
}

#[tokio::test]
async fn paginates_deduplicates_and_never_returns_partial_results() {
    let (context, fixture) = test_context().await;
    let peer: tl::enums::Peer = tl::types::PeerChat { chat_id: 1 }.into();
    let first: tl::enums::messages::Dialogs = tl::types::messages::DialogsSlice {
        count: 101,
        dialogs: vec![fixtures::dialog(peer.clone()); 100],
        messages: vec![fixtures::message(peer)],
        chats: vec![fixtures::group(1).into()],
        users: vec![],
    }
    .into();
    fixture.0.lock().unwrap().replies.extend([
        Ok(first.to_bytes()),
        Ok(fixtures::page(false, false).to_bytes()),
        Ok(fixtures::page(true, false).to_bytes()),
    ]);

    let (chats, _) = adapter::list(&context.client).await.unwrap();

    assert_eq!(chats.len(), 2);

    fixture
        .0
        .lock()
        .unwrap()
        .replies
        .extend([Ok(first.to_bytes()), Err(InvocationError::Dropped)]);

    assert!(matches!(
        adapter::list(&context.client).await,
        Err(ChatError::Telegram(InvocationError::Dropped))
    ));
    fixture
        .0
        .lock()
        .unwrap()
        .replies
        .extend([Ok(first.to_bytes()), Ok(first.to_bytes())]);

    assert!(matches!(
        adapter::list(&context.client).await,
        Err(ChatError::IncompleteList)
    ));
}

#[test]
fn maps_peer_types_and_filters_inaccessible_chats() {
    let mut group = fixtures::group(9007199254740993);

    assert_eq!(
        mapping::chat(&group.clone().into()).unwrap().id,
        "chat:9007199254740993"
    );

    group.left = true;

    assert!(mapping::chat(&group.into()).is_none());
    assert!(mapping::chat(&tl::types::ChatEmpty { id: 1 }.into()).is_none());

    let mut channel = fixtures::channel(1);
    channel.broadcast = false;

    assert!(matches!(
        mapping::chat(&channel.clone().into()).unwrap().kind,
        ChatKind::Group
    ));

    channel.access_hash = None;

    assert!(mapping::chat(&channel.into()).is_none());
}

#[test]
fn advances_all_offsets_and_rejects_missing_message_metadata() {
    let peer: tl::enums::Peer = tl::types::PeerChannel { channel_id: 2 }.into();
    let dialogs = vec![fixtures::dialog(peer.clone())];
    let mut request = tl::functions::messages::GetDialogs {
        exclude_pinned: false,
        folder_id: Some(0),
        offset_date: 0,
        offset_id: 0,
        offset_peer: tl::enums::InputPeer::Empty,
        limit: 100,
        hash: 0,
    };

    mapping::advance(
        &mut request,
        &dialogs,
        &[fixtures::message(peer)],
        &[],
        &[fixtures::channel(2).into()],
    )
    .unwrap();

    assert_eq!(request.offset_id, 10);
    assert_eq!(request.offset_date, 1234);
    assert!(request.exclude_pinned);
    assert!(matches!(
        request.offset_peer,
        tl::enums::InputPeer::Channel(_)
    ));
    assert!(matches!(
        mapping::advance(&mut request, &dialogs, &[], &[], &[]),
        Err(ChatError::IncompleteList)
    ));
}

#[tokio::test]
async fn isolates_accounts_restores_selection_and_replaces_atomically() {
    let path =
        std::env::temp_dir().join(format!("skopos-chats-test-{}.sqlite", std::process::id()));
    let repository = repository::ChatRepository::new(path.clone());
    let selected = mapping::chat(&fixtures::group(1).into()).unwrap();

    assert!(repository.get(1).await.unwrap().is_empty());

    repository.save(1, vec![selected.clone()]).await.unwrap();
    repository
        .save(
            2,
            vec![mapping::chat(&fixtures::channel(2).into()).unwrap()],
        )
        .await
        .unwrap();
    let reopened = repository::ChatRepository::new(path.clone());

    assert_eq!(reopened.get(1).await.unwrap()[0].id, "chat:1");
    assert_eq!(reopened.get(2).await.unwrap()[0].id, "channel:2");
    assert!(matches!(
        reopened.save(1, vec![selected.clone(), selected]).await,
        Err(ChatError::InvalidSelection)
    ));
    assert_eq!(reopened.get(1).await.unwrap().len(), 1);

    reopened.save(1, vec![]).await.unwrap();

    assert!(reopened.get(1).await.unwrap().is_empty());
    assert_eq!(reopened.get(2).await.unwrap().len(), 1);

    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn reports_authentication_and_network_errors_without_a_list() {
    let (context, fixture) = test_context().await;
    fixture
        .0
        .lock()
        .unwrap()
        .replies
        .push_back(Ok(vec![tl::enums::User::from(tl::types::UserEmpty {
            id: 1,
        })]
        .to_bytes()));

    let error = adapter::account_id(&context.client).await.unwrap_err();

    assert!(matches!(error, ChatError::Auth(_)));
    assert!(matches!(
        error.message(),
        crate::app_message::AppMessage::RestartLogin
    ));

    fixture.0.lock().unwrap().scenario.chats_error = true;

    assert!(matches!(
        adapter::list(&context.client).await,
        Err(ChatError::Telegram(_))
    ));
    assert!(matches!(
        ChatError::Telegram(InvocationError::Dropped).message(),
        crate::app_message::AppMessage::ChatLoadFailed
    ));
}

#[test]
fn paginates_past_private_users_and_inaccessible_channels() {
    let mut request = tl::functions::messages::GetDialogs {
        exclude_pinned: false,
        folder_id: Some(0),
        offset_date: 0,
        offset_id: 0,
        offset_peer: tl::enums::InputPeer::Empty,
        limit: 100,
        hash: 0,
    };
    let user_peer: tl::enums::Peer = tl::types::PeerUser { user_id: 3 }.into();
    let dialogs = [fixtures::dialog(user_peer.clone())];
    let messages = [fixtures::message(user_peer)];
    let mut user = fixtures::account(3);

    mapping::advance(
        &mut request,
        &dialogs,
        &messages,
        &[user.clone().into()],
        &[],
    )
    .unwrap();

    assert!(matches!(
        request.offset_peer,
        tl::enums::InputPeer::PeerSelf
    ));

    user.is_self = false;
    user.access_hash = Some(42);
    mapping::advance(
        &mut request,
        &dialogs,
        &messages,
        &[user.clone().into()],
        &[],
    )
    .unwrap();

    assert!(
        matches!(&request.offset_peer, tl::enums::InputPeer::User(peer) if peer.user_id == 3 && peer.access_hash == 42)
    );

    user.access_hash = None;

    assert!(matches!(
        mapping::advance(&mut request, &dialogs, &messages, &[user.into()], &[]),
        Err(ChatError::IncompleteList)
    ));
    assert!(mapping::advance(
        &mut request,
        &dialogs,
        &messages,
        &[tl::types::UserEmpty { id: 3 }.into()],
        &[]
    )
    .is_err());

    let channel_peer: tl::enums::Peer = tl::types::PeerChannel { channel_id: 4 }.into();
    let dialogs = [fixtures::dialog(channel_peer.clone())];
    let messages = [fixtures::message(channel_peer)];
    let forbidden: tl::enums::Chat = tl::types::ChannelForbidden {
        broadcast: true,
        megagroup: false,
        monoforum: false,
        id: 4,
        access_hash: 99,
        title: "Unavailable".into(),
        until_date: None,
    }
    .into();
    mapping::advance(
        &mut request,
        &dialogs,
        &messages,
        &[],
        std::slice::from_ref(&forbidden),
    )
    .unwrap();

    assert!(
        matches!(&request.offset_peer, tl::enums::InputPeer::Channel(peer) if peer.channel_id == 4 && peer.access_hash == 99)
    );
    assert!(mapping::chat(&forbidden).is_none());
    assert!(mapping::advance(
        &mut request,
        &dialogs,
        &messages,
        &[],
        &[fixtures::group(4).into()]
    )
    .is_err());
    assert!(mapping::advance(&mut request, &[], &messages, &[], &[]).is_err());
}

#[test]
fn advances_using_the_matching_message_and_peer() {
    let peer: tl::enums::Peer = tl::types::PeerChat { chat_id: 7 }.into();
    let dialogs = [fixtures::dialog(peer.clone())];
    let mut request = tl::functions::messages::GetDialogs {
        exclude_pinned: false,
        folder_id: Some(0),
        offset_date: 0,
        offset_id: 0,
        offset_peer: tl::enums::InputPeer::Empty,
        limit: 100,
        hash: 0,
    };
    let message: tl::enums::Message = tl::types::Message {
        out: false,
        mentioned: false,
        media_unread: false,
        silent: false,
        post: false,
        from_scheduled: false,
        legacy: false,
        edit_hide: false,
        pinned: false,
        noforwards: false,
        invert_media: false,
        offline: false,
        video_processing_pending: false,
        paid_suggested_post_stars: false,
        paid_suggested_post_ton: false,
        id: 10,
        from_id: None,
        from_boosts_applied: None,
        from_rank: None,
        peer_id: peer,
        saved_peer_id: None,
        fwd_from: None,
        via_bot_id: None,
        via_business_bot_id: None,
        guestchat_via_from: None,
        reply_to: None,
        date: 5678,
        message: "Promotion".into(),
        media: None,
        reply_markup: None,
        entities: None,
        views: None,
        forwards: None,
        replies: None,
        edit_date: None,
        post_author: None,
        grouped_id: None,
        reactions: None,
        restriction_reason: None,
        ttl_period: None,
        quick_reply_shortcut_id: None,
        effect: None,
        factcheck: None,
        report_delivery_until_date: None,
        paid_message_stars: None,
        suggested_post: None,
        schedule_repeat_period: None,
        summary_from_language: None,
        rich_message: None,
    }
    .into();
    let unrelated = fixtures::message(tl::types::PeerChat { chat_id: 8 }.into());

    let cursor = mapping::advance(&mut request, &dialogs, &[unrelated, message], &[], &[]).unwrap();

    assert_eq!(cursor, ("chat:7".into(), 10, 5678));
    assert!(matches!(request.offset_peer, tl::enums::InputPeer::Chat(peer) if peer.chat_id == 7));
    assert!(mapping::contains(&dialogs[0], "chat:7"));
    assert!(!mapping::contains(&dialogs[0], "chat:8"));
}

#[tokio::test]
async fn rejects_unexpected_pages_and_collects_available_photos() {
    let (context, fixture) = test_context().await;
    fixture
        .0
        .lock()
        .unwrap()
        .replies
        .push_back(Ok(tl::enums::messages::Dialogs::from(
            tl::types::messages::DialogsNotModified { count: 2 },
        )
        .to_bytes()));

    assert!(matches!(
        adapter::list(&context.client).await,
        Err(ChatError::IncompleteList)
    ));

    let mut group = fixtures::group(1);
    group.photo = tl::types::ChatPhoto {
        has_video: false,
        photo_id: 42,
        stripped_thumb: None,
        dc_id: 2,
    }
    .into();
    let page: tl::enums::messages::Dialogs = tl::types::messages::Dialogs {
        dialogs: vec![fixtures::dialog(tl::types::PeerChat { chat_id: 1 }.into())],
        messages: vec![],
        chats: vec![
            group.into(),
            fixtures::group(3).into(),
            tl::types::ChatEmpty { id: 5 }.into(),
        ],
        users: vec![],
    }
    .into();
    fixture
        .0
        .lock()
        .unwrap()
        .replies
        .push_back(Ok(page.to_bytes()));

    let (chats, locations) = adapter::list(&context.client).await.unwrap();

    assert_eq!(chats.len(), 2);
    assert_eq!(locations.len(), 1);
    assert!(locations.contains_key("chat:1"));
}
