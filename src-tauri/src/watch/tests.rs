use super::repository::{CreateWatch, WatchError, WatchRepository};
use crate::app_message::AppMessage;

fn test_path(name: &str) -> std::path::PathBuf {
    let id = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("skopos-{name}-{}-{id}.sqlite", std::process::id()))
}

#[tokio::test]
async fn saves_and_reloads_ordered_phrases_and_price() {
    let path = test_path("watches");
    let repository = WatchRepository::new(path.clone());
    assert!(repository.list().await.unwrap().is_empty());

    let saved = repository
        .create(CreateWatch {
            phrases: vec![" Laptop Vivobook S14 ".into(), "Asus Vivobook 14".into()],
            max_price_cents: Some(350_000),
            min_price_cents: None,
        })
        .await
        .unwrap();
    assert_eq!(saved.phrases, ["Laptop Vivobook S14", "Asus Vivobook 14"]);
    assert_eq!(saved.max_price_cents, Some(350_000));

    let second = repository
        .create(CreateWatch {
            phrases: vec!["RTX 5070".into()],
            max_price_cents: None,
            min_price_cents: None,
        })
        .await
        .unwrap();
    assert!(second.id > saved.id);

    let reopened = WatchRepository::new(path.clone());
    assert_eq!(reopened.list().await.unwrap(), [second, saved]);
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn rejects_invalid_inputs_before_opening_storage() {
    let repository = WatchRepository::new(test_path("validation"));
    for phrases in [vec![], vec!["  ".into()], vec!["Valid".into(), "".into()]] {
        let result = repository
            .create(CreateWatch {
                phrases,
                max_price_cents: None,
                min_price_cents: None,
            })
            .await;
        assert!(matches!(result, Err(WatchError::InvalidPhrase)));
    }
    for price in [0, -1] {
        let result = repository
            .create(CreateWatch {
                phrases: vec!["Valid".into()],
                max_price_cents: Some(price),
                min_price_cents: None,
            })
            .await;
        assert!(matches!(result, Err(WatchError::InvalidPrice)));
    }
    assert_eq!(
        WatchError::InvalidPhrase.message(),
        AppMessage::InvalidPhrase
    );
    assert_eq!(WatchError::InvalidPrice.message(), AppMessage::InvalidPrice);
}

#[tokio::test]
async fn reports_storage_failures_without_exposing_database_errors() {
    let repository = WatchRepository::new(test_path("missing").join("watches.sqlite"));
    assert!(matches!(repository.list().await, Err(WatchError::Storage)));
    assert!(matches!(
        repository
            .create(CreateWatch {
                phrases: vec!["Valid".into()],
                max_price_cents: None,
                min_price_cents: None,
            })
            .await,
        Err(WatchError::Storage)
    ));
    assert_eq!(WatchError::Storage.message(), AppMessage::WatchStorage);
}

#[tokio::test]
async fn updates_names_and_price_without_changing_identity_or_other_products() {
    let path = test_path("update");
    let repository = WatchRepository::new(path.clone());
    let original = repository
        .create(CreateWatch {
            phrases: vec!["Laptop".into()],
            max_price_cents: Some(350_000),
            min_price_cents: None,
        })
        .await
        .unwrap();
    let other = repository
        .create(CreateWatch {
            phrases: vec!["RTX 5070".into()],
            max_price_cents: None,
            min_price_cents: None,
        })
        .await
        .unwrap();

    let updated = repository
        .update(
            original.id,
            CreateWatch {
                phrases: vec![" Laptop OLED ".into(), "Asus S14".into()],
                max_price_cents: Some(325_099),
                min_price_cents: None,
            },
        )
        .await
        .unwrap();

    assert_eq!(updated.id, original.id);
    assert_eq!(updated.phrases, ["Laptop OLED", "Asus S14"]);
    assert_eq!(updated.max_price_cents, Some(325_099));
    let reopened = WatchRepository::new(path.clone());
    assert_eq!(reopened.list().await.unwrap(), [other, updated]);

    let uncapped = reopened
        .update(
            original.id,
            CreateWatch {
                phrases: vec!["Vivobook".into()],
                max_price_cents: None,
                min_price_cents: None,
            },
        )
        .await
        .unwrap();

    assert_eq!(uncapped.max_price_cents, None);
    assert_eq!(repository.list().await.unwrap()[1], uncapped);
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn invalid_updates_preserve_the_saved_product() {
    let path = test_path("invalid-update");
    let repository = WatchRepository::new(path.clone());
    let saved = repository
        .create(CreateWatch {
            phrases: vec!["RTX 5070".into()],
            max_price_cents: Some(400_000),
            min_price_cents: None,
        })
        .await
        .unwrap();

    for phrases in [vec![], vec!["  ".into()], vec!["Valid".into(), "".into()]] {
        assert!(matches!(
            repository
                .update(
                    saved.id,
                    CreateWatch {
                        phrases,
                        max_price_cents: None,
                        min_price_cents: None,
                    }
                )
                .await,
            Err(WatchError::InvalidPhrase)
        ));
    }
    for price in [0, -1] {
        assert!(matches!(
            repository
                .update(
                    saved.id,
                    CreateWatch {
                        phrases: vec!["Changed".into()],
                        max_price_cents: Some(price),
                        min_price_cents: None,
                    }
                )
                .await,
            Err(WatchError::InvalidPrice)
        ));
    }

    assert_eq!(repository.list().await.unwrap(), [saved]);
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn deletion_persists_and_updating_a_deleted_product_reports_not_found() {
    let path = test_path("delete");
    let repository = WatchRepository::new(path.clone());
    let saved = repository
        .create(CreateWatch {
            phrases: vec!["RTX 5070".into()],
            max_price_cents: None,
            min_price_cents: None,
        })
        .await
        .unwrap();

    repository.delete(saved.id).await.unwrap();
    repository.delete(saved.id).await.unwrap();
    let reopened = WatchRepository::new(path.clone());

    assert!(reopened.list().await.unwrap().is_empty());
    assert!(matches!(
        reopened
            .update(
                saved.id,
                CreateWatch {
                    phrases: vec!["RTX 5080".into()],
                    max_price_cents: None,
                    min_price_cents: None,
                }
            )
            .await,
        Err(WatchError::NotFound)
    ));
    assert_eq!(WatchError::NotFound.message(), AppMessage::ProductNotFound);
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn update_and_delete_report_inaccessible_storage() {
    let repository = WatchRepository::new(test_path("missing-update").join("watches.sqlite"));

    assert!(matches!(
        repository
            .update(
                1,
                CreateWatch {
                    phrases: vec!["RTX 5070".into()],
                    max_price_cents: None,
                    min_price_cents: None,
                }
            )
            .await,
        Err(WatchError::Storage)
    ));
    assert!(matches!(
        repository.delete(1).await,
        Err(WatchError::Storage)
    ));
}

#[tokio::test]
async fn migrates_existing_products_to_automatic_minimum_without_losing_data() {
    let path = test_path("minimum-migration");
    let database = libsql::Builder::new_local(&path).build().await.unwrap();
    let connection = database.connect().unwrap();
    connection.execute("CREATE TABLE watches (id INTEGER PRIMARY KEY, phrases TEXT NOT NULL, max_price_cents INTEGER)", ()).await.unwrap();
    connection.execute(r#"INSERT INTO watches VALUES (7, '["Dishwasher"]', 500003), (8, '["Controller"]', NULL)"#, ()).await.unwrap();
    drop(connection);
    drop(database);

    let repository = WatchRepository::new(path.clone());
    let watches = repository.list().await.unwrap();

    assert_eq!(watches[1].id, 7);
    assert_eq!(watches[1].phrases, ["Dishwasher"]);
    assert_eq!(watches[1].min_price_cents, None);
    assert_eq!(watches[1].minimum_price_cents(), 100000);
    assert_eq!(watches[0].minimum_price_cents(), 0);
    assert_eq!(
        WatchRepository::new(path.clone()).list().await.unwrap(),
        watches
    );
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn persists_custom_minimum_zero_and_restored_automatic_mode() {
    let path = test_path("minimum-modes");
    let repository = WatchRepository::new(path.clone());
    let saved = repository
        .create(CreateWatch {
            phrases: vec!["Dishwasher".into()],
            max_price_cents: Some(500000),
            min_price_cents: Some(120000),
        })
        .await
        .unwrap();

    for (minimum, maximum, effective) in [
        (Some(120000), Some(600000), 120000),
        (Some(0), Some(600000), 0),
        (None, Some(600003), 120000),
        (None, None, 0),
        (Some(1800), None, 1800),
    ] {
        let updated = repository
            .update(
                saved.id,
                CreateWatch {
                    phrases: vec!["Dishwasher".into()],
                    max_price_cents: maximum,
                    min_price_cents: minimum,
                },
            )
            .await
            .unwrap();

        assert_eq!(updated.minimum_price_cents(), effective);
        assert_eq!(
            WatchRepository::new(path.clone()).list().await.unwrap(),
            [updated]
        );
    }
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn rejects_negative_or_inverted_minimum_without_changing_the_product() {
    let path = test_path("minimum-invalid");
    let repository = WatchRepository::new(path.clone());
    let input = || CreateWatch {
        phrases: vec!["Dishwasher".into()],
        max_price_cents: Some(500000),
        min_price_cents: Some(500000),
    };
    let saved = repository.create(input()).await.unwrap();

    for minimum in [-1, 500001] {
        let mut invalid = input();
        invalid.min_price_cents = Some(minimum);
        assert!(matches!(
            repository.create(invalid).await,
            Err(WatchError::InvalidPrice)
        ));

        let mut invalid = input();
        invalid.min_price_cents = Some(minimum);
        assert!(matches!(
            repository.update(saved.id, invalid).await,
            Err(WatchError::InvalidPrice)
        ));
    }

    assert_eq!(repository.list().await.unwrap(), [saved]);
    std::fs::remove_file(path).unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn concurrent_reads_do_not_lock_an_already_migrated_database() {
    let path = test_path("minimum-concurrent");
    let repository = WatchRepository::new(path.clone());
    repository
        .create(CreateWatch {
            phrases: vec!["Dishwasher".into()],
            max_price_cents: Some(500000),
            min_price_cents: None,
        })
        .await
        .unwrap();
    let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(17));
    let readers: Vec<_> = (0..16)
        .map(|_| {
            let path = path.clone();
            let barrier = barrier.clone();
            tokio::spawn(async move {
                barrier.wait().await;
                WatchRepository::new(path).list().await
            })
        })
        .collect();

    barrier.wait().await;
    for reader in readers {
        let watches = reader.await.unwrap().unwrap();
        assert_eq!(watches.len(), 1);
    }
    std::fs::remove_file(path).unwrap();
}
