use super::*;
use crate::watch::Watch;

fn monitoring_data() -> String {
    serde_json::to_string(&crate::telegram::monitoring::Record::default()).unwrap()
}

fn message(text: &str) -> SourceMessage {
    SourceMessage {
        chat_id: "channel:1".into(),
        chat_title: "Deals".into(),
        message_id: 7,
        posted_at: 100,
        text: text.into(),
        message_link: Some("https://t.me/deals/7".into()),
        image: None,
    }
}

fn watch(id: i64, name: &str, ceiling: Option<i64>) -> Watch {
    Watch {
        id,
        phrases: vec![name.into()],
        max_price_cents: ceiling,
        min_price_cents: None,
    }
}

#[test]
fn older_saved_messages_without_images_still_load() {
    let mut old = serde_json::to_value(message("Controller R$ 201")).unwrap();
    old.as_object_mut().unwrap().remove("image");

    let restored: SourceMessage = serde_json::from_value(old).unwrap();

    assert_eq!(restored.text, "Controller R$ 201");
    assert!(restored.image.is_none());
}

#[tokio::test]
async fn cached_images_survive_reopen_and_a_later_failed_download() {
    let directory = std::env::temp_dir().join(format!("skopos-image-test-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("results.sqlite");
    let mut original = message("Controller R$ 201");
    original.image = Some("data:image/jpeg;base64,/9j/2Q==".into());
    let repository = ResultRepository::new(path.clone());
    repository
        .merge_monitoring(
            77,
            vec![original.clone()],
            SearchSummary::default(),
            &monitoring_data(),
        )
        .await
        .unwrap();

    let reopened = ResultRepository::new(path);

    assert_eq!(reopened.load(77).await.unwrap().0[0].image, original.image);

    reopened
        .merge_monitoring(
            77,
            vec![message("Controller R$ 200")],
            SearchSummary::default(),
            &monitoring_data(),
        )
        .await
        .unwrap();

    let saved = reopened.load(77).await.unwrap().0;

    assert_eq!(saved[0].image, original.image);
    assert_eq!(saved[0].text, "Controller R$ 200");

    reopened.clear(77, None).await.unwrap();

    assert!(reopened.load(77).await.unwrap().0.is_empty());
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn matches_all_tokens_in_any_name_with_boundaries_and_case_folding() {
    let mut product = watch(1, "RTX 5070", None);
    product.phrases.push("GPU Azul".into());

    for text in ["5070 nova rtx", "GPU disponível AZUL"] {
        assert_eq!(
            match_messages(&[message(text)], std::slice::from_ref(&product)).len(),
            1
        );
    }

    for text in ["RTX 5070Ti", "RTX 50700", "GPU", "Azul", "unrelated"] {
        assert!(match_messages(&[message(text)], std::slice::from_ref(&product)).is_empty());
    }
}

#[test]
fn compares_full_brl_prices_without_installments_or_old_prices() {
    let product = watch(1, "Controle", Some(50_000));

    for (text, expected) in [
        ("Controle R$ 201", Some(20_100)),
        ("Controle R$ 499,99", Some(49_999)),
        ("Controle de R$ 800 por R$ 200", Some(20_000)),
        ("Controle R$ 1.201,50", None),
        ("Controle 10x R$ 50", None),
        ("Controle 10 x R$ 50", None),
        ("Controle R$ 50/mês", None),
        ("Controle R$ 50 por parcela", None),
        ("Controle parcelas de R$ 50", None),
        ("Controle R$ 0", None),
        ("Controle R$ 12,3", None),
        ("Controle R$ 1.20", None),
        ("Controle R$ 999999999999999999999", None),
    ] {
        let matches = match_messages(&[message(text)], std::slice::from_ref(&product));

        assert_eq!(
            matches.first().and_then(|found| found.price_cents),
            expected,
            "{text}"
        );
        assert_eq!(matches.len(), usize::from(expected.is_some()), "{text}");
    }
}

#[test]
fn overlapping_watches_share_a_product_price_and_keep_independent_ceilings() {
    let products = [
        watch(1, "RTX 5070", Some(400_000)),
        watch(2, "RTX 5070 ASUS", Some(400_000)),
        watch(3, "RTX 5070", Some(380_000)),
    ];

    for text in ["RTX 5070 ASUS R$ 3.900", "RTX 5070 ASUS\nR$ 3.900"] {
        let matches = match_messages(&[message(text)], &products);

        assert_eq!(
            matches
                .iter()
                .map(|found| (found.watch_id, found.price_cents))
                .collect::<Vec<_>>(),
            [(1, Some(390_000)), (2, Some(390_000))],
            "{text}"
        );
    }
}

#[test]
fn shipping_charges_do_not_replace_the_product_price() {
    let product = watch(1, "Controle", Some(50_000));

    for (text, expected) in [
        ("Controle R$ 600 + frete R$ 20", None),
        ("Controle R$ 400 + frete: R$ 20", Some(40_000)),
        ("Controle R$ 20 de frete", None),
    ] {
        let matches = match_messages(&[message(text)], std::slice::from_ref(&product));

        assert_eq!(matches.len(), usize::from(expected.is_some()), "{text}");
        assert_eq!(
            matches.first().and_then(|found| found.price_cents),
            expected,
            "{text}"
        );
    }
}

#[test]
fn installments_after_the_amount_do_not_replace_the_full_price() {
    let product = watch(1, "Controle", Some(50_000));

    for (text, expected) in [
        ("Controle R$ 600 ou R$ 60 em 10x", None),
        ("Controle R$ 600 ou R$ 60 em 10 x", None),
        ("Controle R$ 600 ou R$ 60 em 10 parcelas", None),
        ("Controle R$ 400 ou R$ 40 em 10x", Some(40_000)),
    ] {
        let matches = match_messages(&[message(text)], std::slice::from_ref(&product));

        assert_eq!(matches.len(), usize::from(expected.is_some()), "{text}");
        assert_eq!(
            matches.first().and_then(|found| found.price_cents),
            expected,
            "{text}"
        );
    }
}

#[test]
fn requires_unambiguous_price_association_for_multiple_products() {
    let products = [
        watch(1, "Controle", Some(50_000)),
        watch(2, "Monitor", Some(100_000)),
    ];

    let matches = match_messages(&[message("Controle R$ 201\nMonitor R$ 800")], &products);

    assert_eq!(
        matches
            .iter()
            .map(|found| (found.watch_id, found.price_cents))
            .collect::<Vec<_>>(),
        [(1, Some(20_100)), (2, Some(80_000))]
    );

    assert!(match_messages(&[message("Controle e Monitor\nR$ 201")], &products).is_empty());
    assert!(match_messages(&[message("Controle e Monitor R$ 201")], &products).is_empty());
    assert!(match_messages(&[message("Controle\nR$ 201\nR$ 800")], &products[..1]).is_empty());

    let any_price = watch(3, "Controle", None);

    assert_eq!(
        match_messages(&[message("Controle sem preço")], &[any_price])[0].price_cents,
        None
    );
}

#[tokio::test]
async fn persists_deduplicates_updates_and_clears_per_account() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "skopos-results-{}-{nonce}.sqlite",
        std::process::id()
    ));
    let repository = ResultRepository::new(path.clone());

    assert!(repository.load(1).await.unwrap().0.is_empty());

    let summary = SearchSummary {
        started_at: 200,
        since: 100,
        completed_chats: 1,
        total_chats: 1,
        ..SearchSummary::default()
    };

    repository
        .merge_monitoring(
            1,
            vec![message("Controle R$ 201")],
            summary.clone(),
            &monitoring_data(),
        )
        .await
        .unwrap();
    let mut newer = message("Controle R$ 202");
    newer.posted_at = 150;
    repository
        .merge_monitoring(1, vec![newer.clone()], summary.clone(), &monitoring_data())
        .await
        .unwrap();
    let mut other_chat = message("Controle R$ 300");
    other_chat.chat_id = "channel:2".into();
    repository
        .merge_monitoring(1, vec![other_chat], summary.clone(), &monitoring_data())
        .await
        .unwrap();

    let reopened = ResultRepository::new(path.clone());
    let (saved, metadata) = reopened.load(1).await.unwrap();

    assert_eq!(saved.len(), 2);
    assert_eq!(saved[0].text, newer.text);
    assert_eq!(metadata.unwrap().started_at, 200);
    assert!(reopened.load(2).await.unwrap().0.is_empty());

    reopened.clear(1, Some(150)).await.unwrap();

    assert_eq!(reopened.load(1).await.unwrap().0.len(), 1);
    assert!(reopened.load(1).await.unwrap().1.is_some());

    reopened.clear(1, None).await.unwrap();

    let (saved, metadata) = reopened.load(1).await.unwrap();

    assert!(saved.is_empty());
    assert!(metadata.is_none());
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn reports_storage_failure_without_publishing_database_details() {
    let repository = ResultRepository::new(std::env::temp_dir());

    assert!(repository.load(1).await.is_err());
    assert!(repository.clear(1, None).await.is_err());
    assert!(repository
        .merge_monitoring(1, vec![], SearchSummary::default(), &monitoring_data())
        .await
        .is_err());
}

#[test]
fn minimum_price_filters_accessories_and_keeps_inclusive_boundaries() {
    let mut product = watch(1, "Dishwasher", Some(500003));

    for (text, expected) in [
        ("Detergent for Dishwasher R$ 18", false),
        ("Dishwasher R$ 999,99", false),
        ("Dishwasher R$ 1.000", true),
        ("Dishwasher R$ 5.000,03", true),
        ("Dishwasher R$ 5.000,04", false),
        ("Dishwasher without price", false),
    ] {
        assert_eq!(
            !match_messages(&[message(text)], std::slice::from_ref(&product)).is_empty(),
            expected,
            "{text}"
        );
    }

    product.min_price_cents = Some(0);

    assert_eq!(
        match_messages(
            &[message("Detergent for Dishwasher R$ 18")],
            std::slice::from_ref(&product)
        )
        .len(),
        1
    );

    product.max_price_cents = None;

    product.min_price_cents = Some(1800);

    assert!(match_messages(
        &[message("Dishwasher without price")],
        std::slice::from_ref(&product)
    )
    .is_empty());

    assert!(match_messages(
        &[message("Dishwasher R$ 17,99")],
        std::slice::from_ref(&product)
    )
    .is_empty());

    assert_eq!(
        match_messages(
            &[message("Dishwasher R$ 18")],
            std::slice::from_ref(&product)
        )
        .len(),
        1
    );

    product.min_price_cents = None;

    assert_eq!(
        match_messages(&[message("Dishwasher without price")], &[product]).len(),
        1
    );
}

#[tokio::test]
async fn saved_messages_rematch_after_minimum_edits_without_deleting_sources() {
    let path =
        std::env::temp_dir().join(format!("skopos-min-rematch-{}.sqlite", std::process::id()));
    let repository = ResultRepository::new(path.clone());
    let mut cheap = message("Detergent for Dishwasher R$ 18");
    cheap.message_id = 2;
    repository
        .merge_monitoring(
            77,
            vec![cheap, message("Dishwasher R$ 1.526,88")],
            SearchSummary::default(),
            &monitoring_data(),
        )
        .await
        .unwrap();
    let messages = repository.load(77).await.unwrap().0;
    let mut product = watch(1, "Dishwasher", Some(500000));

    assert_eq!(
        match_messages(&messages, std::slice::from_ref(&product)).len(),
        1
    );

    product.min_price_cents = Some(0);

    assert_eq!(match_messages(&messages, &[product]).len(), 2);
    assert_eq!(repository.load(77).await.unwrap().0.len(), 2);
    std::fs::remove_file(path).unwrap();
}
