use super::*;
use crate::telegram::{
    chats::Chat,
    e2e::{chats as chat_fixtures, history, test_context},
    AuthState,
};
use grammers_client::{sender::RpcError, tl};
use std::collections::HashSet;
use tl::Serializable;

#[tokio::test]
async fn skips_selected_chats_that_disappeared_before_the_search() {
    let (context, _) = test_context().await;
    let auth = AuthState::default();
    let state = SearchState::default();
    let run = workflow::SearchRun {
        api: &context.client,
        auth: &auth,
        state: &state,
        generation: 0,
        cancellation: 0,
        since: 100,
        until: 200,
        saved_messages: HashSet::new(),
    };

    let (messages, summary) = workflow::search(&run, vec![chat("channel:999", None)], &[])
        .await
        .unwrap();
    assert!(messages.is_empty());
    assert_eq!(summary.unavailable_chats, ["Deals"]);
    assert_eq!(summary.completed_chats, 0);
}

fn chat(id: &str, username: Option<&str>) -> Chat {
    Chat {
        id: id.into(),
        title: "Deals".into(),
        kind: crate::telegram::chats::ChatKind::Channel,
        username: username.map(str::to_owned),
        available: true,
    }
}

#[tokio::test]
async fn history_enforces_snapshot_bounds_and_original_links() {
    let (context, fixture) = test_context().await;
    let auth = AuthState::default();
    let state = SearchState::default();
    let run = workflow::SearchRun {
        api: &context.client,
        auth: &auth,
        state: &state,
        generation: 0,
        cancellation: 0,
        since: 100,
        until: 200,
        saved_messages: HashSet::new(),
    };
    let peer: tl::enums::Peer = tl::types::PeerChannel { channel_id: 2 }.into();
    fixture
        .0
        .lock()
        .unwrap()
        .replies
        .push_back(Ok(history::page(vec![
            history::message(peer.clone(), 10, 201, "future"),
            history::message(peer.clone(), 9, 200, "upper boundary"),
            history::message(peer.clone(), 8, 100, "lower boundary"),
            history::message(peer.clone(), 7, 150, " "),
            history::message(peer, 6, 99, "old"),
        ])
        .to_bytes()));

    let messages = adapter::history(
        &run,
        tl::types::InputPeerChannel {
            channel_id: 2,
            access_hash: 22,
        }
        .into(),
        &chat("channel:2", None),
        &[],
    )
    .await
    .unwrap();
    assert_eq!(
        messages
            .iter()
            .map(|message| message.message_id)
            .collect::<Vec<_>>(),
        [9, 8]
    );
    assert_eq!(
        messages[0].message_link.as_deref(),
        Some("https://t.me/c/2/9")
    );

    state.cancellation.fetch_add(1, Ordering::SeqCst);
    assert!(matches!(
        run.ensure_current().await,
        Err(SearchError::Cancelled)
    ));
}

#[tokio::test]
async fn pagination_handles_empty_pages_and_rejects_repeated_offsets() {
    let (context, fixture) = test_context().await;
    let auth = AuthState::default();
    let state = SearchState::default();
    let run = workflow::SearchRun {
        api: &context.client,
        auth: &auth,
        state: &state,
        generation: 0,
        cancellation: 0,
        since: 100,
        until: 200,
        saved_messages: HashSet::new(),
    };
    let peer: tl::enums::Peer = tl::types::PeerChat { chat_id: 1 }.into();
    let page = history::page(vec![history::message(peer, 10, 150, "Controller")]).to_bytes();
    fixture
        .0
        .lock()
        .unwrap()
        .replies
        .extend([Ok(page.clone()), Ok(history::page(vec![]).to_bytes())]);

    let messages = adapter::history(
        &run,
        tl::types::InputPeerChat { chat_id: 1 }.into(),
        &chat("group:1", Some("deals")),
        &[],
    )
    .await
    .unwrap();
    assert_eq!(
        messages[0].message_link.as_deref(),
        Some("https://t.me/deals/10")
    );

    fixture
        .0
        .lock()
        .unwrap()
        .replies
        .extend([Ok(page.clone()), Ok(page)]);
    assert!(matches!(
        adapter::history(
            &run,
            tl::types::InputPeerChat { chat_id: 1 }.into(),
            &chat("group:1", None),
            &[]
        )
        .await,
        Err(SearchError::Incomplete)
    ));
}

#[test]
fn maps_rate_limits_authorization_and_unavailable_sources() {
    let error = |code, name: &str, value| {
        SearchError::from(InvocationError::Rpc(RpcError {
            code,
            name: name.into(),
            value,
            caused_by: None,
        }))
    };

    assert_eq!(
        error(401, "AUTH_KEY_UNREGISTERED", None).message(),
        AppMessage::RestartLogin
    );
    assert_eq!(
        error(420, "FLOOD_WAIT", Some(30)).message(),
        AppMessage::ChatRateLimitSeconds { seconds: 30 }
    );
    assert_eq!(
        error(420, "FLOOD_WAIT", None).message(),
        AppMessage::ChatRateLimit
    );
    assert!(error(400, "CHANNEL_PRIVATE", None).is_unavailable());
    assert!(!SearchError::Incomplete.is_unavailable());
    assert_eq!(SearchError::Cancelled.message(), AppMessage::AuthCancelled);
    assert_eq!(SearchError::Incomplete.message(), AppMessage::SearchFailed);
}

#[tokio::test]
async fn logout_cancels_search_before_waiting_for_its_commit_lock() {
    let state = SearchState::default();
    let operation = state.operation.lock().await;
    let cancel = state.cancel_and_wait();
    tokio::pin!(cancel);

    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(1), &mut cancel)
            .await
            .is_err()
    );
    assert_eq!(state.cancellation.load(Ordering::SeqCst), 1);

    drop(operation);
    let _guard = cancel.await;
}

#[tokio::test]
async fn search_reports_dialog_failure_without_claiming_chats_completed() {
    let (context, fixture) = test_context().await;
    let auth = AuthState::default();
    let state = SearchState::default();
    let run = workflow::SearchRun {
        api: &context.client,
        auth: &auth,
        state: &state,
        generation: 0,
        cancellation: 0,
        since: 100,
        until: 200,
        saved_messages: HashSet::new(),
    };
    fixture
        .0
        .lock()
        .unwrap()
        .replies
        .push_back(Err(InvocationError::Dropped));

    let (messages, summary) = workflow::search(&run, vec![chat("group:1", None)], &[])
        .await
        .unwrap();

    assert!(messages.is_empty());
    assert_eq!(summary.total_chats, 1);
    assert_eq!(summary.completed_chats, 0);
    assert_eq!(summary.failure, Some(AppMessage::ChatLoadFailed));
}

#[tokio::test]
async fn search_skips_missing_and_private_chats_but_stops_on_rate_limits() {
    let (context, fixture) = test_context().await;
    let auth = AuthState::default();
    let state = SearchState::default();
    let run = workflow::SearchRun {
        api: &context.client,
        auth: &auth,
        state: &state,
        generation: 0,
        cancellation: 0,
        since: 100,
        until: 200,
        saved_messages: HashSet::new(),
    };
    let error = |code, name: &str, value| {
        InvocationError::Rpc(RpcError {
            code,
            name: name.into(),
            value,
            caused_by: None,
        })
    };
    fixture.0.lock().unwrap().replies.extend([
        Ok(chat_fixtures::page(false, false).to_bytes()),
        Ok(chat_fixtures::page(true, false).to_bytes()),
        Err(error(400, "CHANNEL_PRIVATE", None)),
        Err(error(420, "FLOOD_WAIT", Some(30))),
    ]);
    let mut missing = chat("channel:999", None);
    missing.title = "Removed channel".into();
    let mut private = chat("channel:2", None);
    private.title = "Private channel".into();

    let (messages, summary) =
        workflow::search(&run, vec![missing, private, chat("chat:1", None)], &[])
            .await
            .unwrap();

    assert!(messages.is_empty());
    assert_eq!(summary.total_chats, 3);
    assert_eq!(summary.completed_chats, 0);
    assert_eq!(summary.unavailable_chats, ["Removed channel", "Channel 2"]);
    assert_eq!(
        summary.failure,
        Some(AppMessage::ChatRateLimitSeconds { seconds: 30 })
    );
}

#[tokio::test]
async fn history_accepts_paginated_variants_and_skips_non_product_messages() {
    let (context, fixture) = test_context().await;
    let auth = AuthState::default();
    let state = SearchState::default();
    let run = workflow::SearchRun {
        api: &context.client,
        auth: &auth,
        state: &state,
        generation: 0,
        cancellation: 0,
        since: 100,
        until: 200,
        saved_messages: HashSet::new(),
    };
    let peer: tl::enums::Peer = tl::types::PeerChat { chat_id: 1 }.into();
    let tl::enums::Message::Service(mut service) = chat_fixtures::message(peer.clone()) else {
        panic!("expected service fixture");
    };
    service.id = 9;
    service.date = 150;
    let messages = vec![
        history::message(peer, 10, 150, "Controller R$ 200"),
        service.into(),
        tl::types::MessageEmpty {
            id: 8,
            peer_id: None,
        }
        .into(),
    ];
    let slice: tl::enums::messages::Messages = tl::types::messages::MessagesSlice {
        inexact: false,
        count: 3,
        next_rate: None,
        offset_id_offset: None,
        search_flood: None,
        messages,
        topics: vec![],
        chats: vec![],
        users: vec![],
    }
    .into();
    fixture
        .0
        .lock()
        .unwrap()
        .replies
        .extend([Ok(slice.to_bytes()), Ok(history::page(vec![]).to_bytes())]);

    let messages = adapter::history(
        &run,
        tl::types::InputPeerChat { chat_id: 1 }.into(),
        &chat("group:1", None),
        &[],
    )
    .await
    .unwrap();

    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].message_id, 10);

    let unchanged: tl::enums::messages::Messages =
        tl::types::messages::MessagesNotModified { count: 0 }.into();
    fixture
        .0
        .lock()
        .unwrap()
        .replies
        .push_back(Ok(unchanged.to_bytes()));

    assert!(matches!(
        adapter::history(
            &run,
            tl::types::InputPeerChat { chat_id: 1 }.into(),
            &chat("group:1", None),
            &[]
        )
        .await,
        Err(SearchError::Incomplete)
    ));
}
