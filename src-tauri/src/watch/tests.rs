use super::repository::{CreateWatch, WatchError, WatchRepository};

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
        })
        .await
        .unwrap();
    assert_eq!(saved.phrases, ["Laptop Vivobook S14", "Asus Vivobook 14"]);
    assert_eq!(saved.max_price_cents, Some(350_000));

    let second = repository
        .create(CreateWatch {
            phrases: vec!["RTX 5070".into()],
            max_price_cents: None,
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
            })
            .await;
        assert!(matches!(result, Err(WatchError::InvalidPhrase)));
    }
    for price in [0, -1] {
        let result = repository
            .create(CreateWatch {
                phrases: vec!["Valid".into()],
                max_price_cents: Some(price),
            })
            .await;
        assert!(matches!(result, Err(WatchError::InvalidPrice)));
    }
    assert_eq!(
        WatchError::InvalidPhrase.message(),
        "Enter every search phrase before saving"
    );
    assert_eq!(
        WatchError::InvalidPrice.message(),
        "Enter a valid price greater than zero"
    );
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
            })
            .await,
        Err(WatchError::Storage)
    ));
    assert_eq!(
        WatchError::Storage.message(),
        "Could not save or load products on this device"
    );
}

#[tokio::test]
async fn updates_names_and_price_without_changing_identity_or_other_products() {
    let path = test_path("update");
    let repository = WatchRepository::new(path.clone());
    let original = repository
        .create(CreateWatch {
            phrases: vec!["Laptop".into()],
            max_price_cents: Some(350_000),
        })
        .await
        .unwrap();
    let other = repository
        .create(CreateWatch {
            phrases: vec!["RTX 5070".into()],
            max_price_cents: None,
        })
        .await
        .unwrap();

    let updated = repository
        .update(
            original.id,
            CreateWatch {
                phrases: vec![" Laptop OLED ".into(), "Asus S14".into()],
                max_price_cents: Some(325_099),
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
                }
            )
            .await,
        Err(WatchError::NotFound)
    ));
    assert_eq!(
        WatchError::NotFound.message(),
        "This product no longer exists"
    );
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
