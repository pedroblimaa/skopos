use super::FixtureState;

pub(in crate::telegram) async fn test_context() -> (
    crate::telegram::client::ClientContext,
    std::sync::Arc<FixtureState>,
) {
    let fixture = std::sync::Arc::new(FixtureState::default());
    let session = std::sync::Arc::new(
        grammers_session::storages::SqliteSession::open(":memory:")
            .await
            .unwrap(),
    );

    let pool = grammers_client::sender::SenderPool::new(std::sync::Arc::clone(&session), 1);
    let context = crate::telegram::client::ClientContext {
        client: crate::telegram::api::TelegramApi {
            client: grammers_client::Client::new(pool.handle),
            session: std::sync::Arc::new(crate::telegram::client::SessionCache::default()),
            app: None,
            fixture: Some(std::sync::Arc::clone(&fixture)),
        },
        session,
        account_id: std::sync::Arc::new(tokio::sync::Mutex::new(None)),
    };

    (context, fixture)
}
