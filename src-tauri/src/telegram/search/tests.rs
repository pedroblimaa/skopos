use super::*;
use crate::telegram::{
    chats::Chat,
    e2e::{history, test_context},
    AuthState,
};
use grammers_client::{sender::RpcError, tl};
use std::collections::HashSet;
use tl::Serializable;

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
            &chat("group:1", None)
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
