use super::{credentials::parse_credentials, status_for};
use crate::app_message::AppMessage;
use crate::telegram::e2e::test_context;
use grammers_session::storages::SqliteSession;

#[tokio::test]
async fn missing_local_identity_is_resolved_once_and_reused_without_a_login_error() {
    let (context, fixture) = test_context().await;
    fixture.0.lock().unwrap().scenario.authorized = true;
    fixture.0.lock().unwrap().scenario.account_id = Some(77);

    assert_eq!(context.local_account_id().await.unwrap(), 77);
    fixture
        .0
        .lock()
        .unwrap()
        .replies
        .push_back(Err(grammers_client::InvocationError::Dropped));

    assert_eq!(context.local_account_id().await.unwrap(), 77);
    assert_eq!(fixture.0.lock().unwrap().replies.len(), 1);
}

#[tokio::test]
async fn persisted_identity_needs_no_telegram_call() {
    use grammers_session::{
        types::{PeerAuth, PeerInfo},
        Session,
    };
    let (context, fixture) = test_context().await;
    context
        .client
        .session
        .reset(Some(super::SessionStatus::signed_in("Pedro".into())));
    context
        .session
        .cache_peer(&PeerInfo::User {
            id: 88,
            auth: Some(PeerAuth::from_hash(42)),
            bot: Some(false),
            is_self: Some(true),
        })
        .await
        .unwrap();
    fixture
        .0
        .lock()
        .unwrap()
        .replies
        .push_back(Err(grammers_client::InvocationError::Dropped));

    assert_eq!(context.local_account_id().await.unwrap(), 88);
    assert_eq!(fixture.0.lock().unwrap().replies.len(), 1);
}

#[tokio::test]
async fn failed_identity_resolution_can_retry() {
    let (context, fixture) = test_context().await;
    fixture.0.lock().unwrap().scenario.authorized = true;
    fixture
        .0
        .lock()
        .unwrap()
        .replies
        .push_back(Err(grammers_client::InvocationError::Dropped));

    assert!(context.local_account_id().await.is_err());
    assert_eq!(*context.account_id.lock().await, None);

    assert_eq!(context.local_account_id().await.unwrap(), 1);
}

#[tokio::test]
async fn identity_lookup_requires_a_self_user() {
    use grammers_client::tl::Serializable;
    let (context, fixture) = test_context().await;
    context
        .client
        .session
        .reset(Some(super::SessionStatus::signed_in("Pedro".into())));
    fixture
        .0
        .lock()
        .unwrap()
        .replies
        .push_back(Ok(Vec::<grammers_client::tl::enums::User>::new().to_bytes()));

    assert_eq!(
        context.local_account_id().await.unwrap_err().message(),
        AppMessage::RestartLogin
    );
}

#[test]
fn credentials_require_both_values_and_a_numeric_id() {
    for (id, hash) in [
        (None, Some("hash")),
        (Some(""), Some("hash")),
        (Some("1"), None),
        (Some("1"), Some("")),
    ] {
        assert_eq!(
            parse_credentials(id, hash).unwrap_err().message(),
            AppMessage::MissingCredentials
        );
    }

    assert_eq!(
        parse_credentials(Some("wrong"), Some("hash"))
            .unwrap_err()
            .message(),
        AppMessage::InvalidApiId
    );
    assert_eq!(
        parse_credentials(Some("1"), Some("hash")).unwrap(),
        (1, "hash")
    );
}

#[tokio::test]
async fn status_handles_signed_out_signed_in_and_api_failure() {
    let (context, fixture) = test_context().await;

    assert!(!status_for(&context.client).await.unwrap().authorized());

    fixture.0.lock().unwrap().scenario.authorized = true;
    context.client.session.reset(None);

    let status = status_for(&context.client).await.unwrap();

    assert_eq!(
        serde_json::to_value(status).unwrap()["displayName"],
        "Pedro"
    );

    fixture.0.lock().unwrap().scenario.status_error = Some("network".into());
    context.client.session.reset(None);

    assert!(status_for(&context.client).await.is_err());
}

#[tokio::test]
async fn sqlite_session_persists_home_dc_across_reopen() {
    use grammers_session::Session;

    let directory =
        std::env::temp_dir().join(format!("skopos-session-test-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("session.sqlite");

    {
        let session = SqliteSession::open(&path).await.unwrap();
        session.set_home_dc_id(4).await.unwrap();
    }

    let session = SqliteSession::open(&path).await.unwrap();

    assert_eq!(session.home_dc_id().unwrap(), 4);

    drop(session);
    std::fs::remove_dir_all(&directory).unwrap();
}

#[tokio::test]
async fn concurrent_restoration_validates_once_and_local_reads_reuse_the_session() {
    let (context, fixture) = test_context().await;
    fixture.0.lock().unwrap().scenario.authorized = true;
    fixture.0.lock().unwrap().scenario.account_id = Some(77);
    let validation = context.client.session.validation.lock().await;
    let first = status_for(&context.client);
    let second = status_for(&context.client);

    tokio::pin!(first, second);
    std::future::poll_fn(|task| {
        use std::future::Future;

        assert!(first.as_mut().poll(task).is_pending());
        assert!(second.as_mut().poll(task).is_pending());
        std::task::Poll::Ready(())
    })
    .await;
    drop(validation);

    let (first, second) = tokio::join!(first, second);

    assert!(first.unwrap().authorized());
    assert!(second.unwrap().authorized());
    assert_eq!(fixture.0.lock().unwrap().authorization_checks, 1);

    fixture.0.lock().unwrap().scenario.status_error = Some("network".into());

    assert_eq!(context.local_account_id().await.unwrap(), 77);
    assert_eq!(context.local_account_id().await.unwrap(), 77);
    assert!(status_for(&context.client).await.unwrap().authorized());
    assert_eq!(fixture.0.lock().unwrap().authorization_checks, 1);
}

#[test]
fn session_generations_reject_stale_validation_and_revocation_after_a_new_login() {
    let cache = super::SessionCache::default();
    let initial = cache.snapshot().generation;

    assert!(cache.store(initial, super::SessionStatus::signed_in("First".into())));
    cache.reset(None);

    assert!(!cache.store(initial, super::SessionStatus::signed_in("Stale".into())));
    assert!(!cache.revoke(initial));
    assert!(cache.snapshot().status.is_none());

    let current = cache.snapshot().generation;

    assert!(cache.store(current, super::SessionStatus::signed_in("Second".into())));
    assert!(cache.revoke(current));
    assert!(!cache.snapshot().status.unwrap().authorized());
    assert!(!cache.revoke(current));
}

#[tokio::test]
async fn refreshing_after_login_discards_the_previous_account_identity() {
    let (context, fixture) = test_context().await;
    context
        .client
        .session
        .reset(Some(super::SessionStatus::signed_out()));
    *context.account_id.lock().await = Some(77);
    fixture.0.lock().unwrap().scenario.authorized = true;
    fixture.0.lock().unwrap().scenario.account_id = Some(88);

    assert!(context.refresh_status().await.unwrap().authorized());
    assert_eq!(context.local_account_id().await.unwrap(), 88);
    assert_eq!(fixture.0.lock().unwrap().authorization_checks, 1);
}

#[tokio::test]
async fn identity_resolution_waiting_for_the_cache_cannot_publish_after_revocation() {
    let (context, fixture) = test_context().await;
    fixture.0.lock().unwrap().scenario.authorized = true;
    assert!(status_for(&context.client).await.unwrap().authorized());
    let account = context.account_id.lock().await;
    let lookup = context.local_account_id();
    tokio::pin!(lookup);

    std::future::poll_fn(|task| {
        use std::future::Future;

        assert!(lookup.as_mut().poll(task).is_pending());
        std::task::Poll::Ready(())
    })
    .await;
    let generation = context.client.session.snapshot().generation;

    assert!(context.client.session.revoke(generation));
    drop(account);

    assert_eq!(
        lookup.await.unwrap_err().message(),
        AppMessage::AuthCancelled
    );
    assert_eq!(*context.account_id.lock().await, None);
}
