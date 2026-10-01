use super::{credentials::parse_credentials, status_for};
use crate::app_message::AppMessage;
use crate::telegram::e2e::test_context;
use grammers_session::storages::SqliteSession;

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

    let status = status_for(&context.client).await.unwrap();

    assert_eq!(
        serde_json::to_value(status).unwrap()["displayName"],
        "Pedro"
    );

    fixture.0.lock().unwrap().scenario.status_error = Some("network".into());

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
