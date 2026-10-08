use super::*;
use crate::promotion::{ResultRepository, SearchSummary};

#[tokio::test]
async fn invalid_monitoring_storage_is_reported_without_overwriting_it() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "skopos-monitoring-invalid-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("results.sqlite");
    let repository = repository::Repository(path.clone());
    repository.load(77).await.unwrap();
    let database = libsql::Builder::new_local(&path).build().await.unwrap();
    let connection = database.connect().unwrap();

    connection
        .execute("INSERT INTO monitoring VALUES (77, 'invalid json')", ())
        .await
        .unwrap();

    assert!(matches!(
        repository.load(77).await,
        Err(AppMessage::MonitoringStorage)
    ));

    connection
        .execute("UPDATE monitoring SET data = X'FF'", ())
        .await
        .unwrap();

    assert!(matches!(
        repository.load(77).await,
        Err(AppMessage::MonitoringStorage)
    ));

    connection
        .execute("DROP TABLE monitoring", ())
        .await
        .unwrap();
    connection
        .execute(
            "CREATE TABLE monitoring (account_id INTEGER PRIMARY KEY)",
            (),
        )
        .await
        .unwrap();

    assert!(matches!(
        repository.load(77).await,
        Err(AppMessage::MonitoringStorage)
    ));
    assert!(matches!(
        repository.save(77, &Record::default()).await,
        Err(AppMessage::MonitoringStorage)
    ));

    drop(connection);
    drop(database);
    std::fs::remove_dir_all(&directory).unwrap();
    std::fs::write(&directory, b"unavailable directory").unwrap();

    assert!(matches!(
        repository.load(77).await,
        Err(AppMessage::MonitoringStorage)
    ));

    std::fs::remove_file(directory).unwrap();
}

#[tokio::test]
async fn settings_wait_for_a_short_concurrent_results_write() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "skopos-monitoring-contention-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("results.sqlite");
    let repository = repository::Repository(path.clone());
    let mut record = repository.load(77).await.unwrap();
    record.enabled = false;
    let database = libsql::Builder::new_local(path).build().await.unwrap();
    let writer = database.connect().unwrap();
    writer.execute("BEGIN IMMEDIATE", ()).await.unwrap();
    let release = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(100));
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(writer.execute("COMMIT", ()))
            .unwrap();
    });

    let saved = repository.save(77, &record).await;
    release.join().unwrap();

    assert!(saved.is_ok(), "settings write failed: {:?}", saved.err());
    assert!(!repository.load(77).await.unwrap().enabled);

    drop(database);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn daily_slots_survive_restart_and_allow_only_the_due_evening_attempt() {
    let mut record = Record::default();

    assert!(record.is_due("2026-10-07", 9));

    record.claim("2026-10-07".into(), 9, 100);

    let mut reopened: Record =
        serde_json::from_str(&serde_json::to_string(&record).unwrap()).unwrap();

    assert!(!reopened.is_due("2026-10-07", 17));
    assert!(reopened.is_due("2026-10-07", 18));

    reopened.claim("2026-10-07".into(), 20, 200);

    assert!(!reopened.is_due("2026-10-07", 23));
    assert!(reopened.is_due("2026-10-08", 0));
    assert!(!reopened.is_due("2026-10-07", 9));
}

#[test]
fn late_start_consumes_both_slots_and_failures_do_not_reopen_them() {
    let mut record = Record::default();
    record.claim("2026-10-07".into(), 19, 100);
    record.failure = Some(AppMessage::SearchFailed);

    assert!(!record.is_due("2026-10-07", 23));
    assert_eq!(record.last_attempt, Some(100));

    record.enabled = false;

    assert!(!record.is_due("2026-10-08", 9));

    record.enabled = true;

    assert!(record.is_due("2026-10-08", 9));
}

#[tokio::test]
async fn checkpoints_and_candidates_commit_with_results_and_survive_cleanup() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let directory = std::env::temp_dir().join(format!(
        "skopos-monitoring-test-{}-{nonce}",
        std::process::id()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("results.sqlite");
    let repository = repository::Repository(path.clone());
    let results = ResultRepository::new(path.clone());
    let mut record = repository.load(77).await.unwrap();
    record.claim("2026-10-07".into(), 9, 100);
    record.checkpoints.insert(
        "channel:1".into(),
        Checkpoint {
            message_id: 9,
            checked_at: 100,
        },
    );
    let message = crate::promotion::SourceMessage {
        chat_id: "channel:1".into(),
        chat_title: "Deals".into(),
        message_id: 9,
        posted_at: 90,
        text: "Controller R$ 201".into(),
        message_link: None,
        image: None,
    };

    record.pending.push(ProductMatch {
        watch_id: 1,
        price_cents: Some(20100),
        message: message.clone(),
    });

    results
        .merge_monitoring(
            77,
            vec![message],
            SearchSummary::default(),
            &serde_json::to_string(&record).unwrap(),
        )
        .await
        .unwrap();
    results.clear(77, None).await.unwrap();
    let reopened = repository::Repository(path).load(77).await.unwrap();

    assert_eq!(reopened.checkpoints["channel:1"].message_id, 9);
    assert_eq!(reopened.pending.len(), 1);
    assert!(!reopened.is_due("2026-10-07", 10));
    assert!(repository.load(88).await.unwrap().checkpoints.is_empty());
    assert!(results.load(77).await.unwrap().0.is_empty());

    std::fs::remove_dir_all(directory).unwrap();
}
