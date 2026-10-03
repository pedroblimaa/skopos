use super::*;
use crate::watch::Watch;

fn message(text: &str) -> SourceMessage {
    SourceMessage {
        chat_id: "channel:1".into(),
        chat_title: "Deals".into(),
        message_id: 7,
        posted_at: 100,
        text: text.into(),
        message_link: Some("https://t.me/deals/7".into()),
    }
}

fn watch(id: i64, name: &str, ceiling: Option<i64>) -> Watch {
    Watch {
        id,
        phrases: vec![name.into()],
        max_price_cents: ceiling,
    }
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
        .merge(1, vec![message("Controle R$ 201")], summary.clone())
        .await
        .unwrap();
    let mut newer = message("Controle R$ 202");
    newer.posted_at = 150;
    repository
        .merge(1, vec![newer.clone()], summary.clone())
        .await
        .unwrap();
    let mut other_chat = message("Controle R$ 300");
    other_chat.chat_id = "channel:2".into();
    repository
        .merge(1, vec![other_chat], summary.clone())
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
        .merge(1, vec![], SearchSummary::default())
        .await
        .is_err());
}
