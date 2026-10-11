use super::{
    content::{caption, separator},
    delivery::{add_matches, next_delivery, record_telegram_result, Delivery},
    repository::{decode, Record, Repository},
    Settings,
};
use crate::{
    app_message::AppMessage,
    promotion::{ProductMatch, SourceMessage},
};
use serde_json::json;
use std::sync::atomic::{AtomicU64, Ordering};

fn result(id: i32) -> ProductMatch {
    ProductMatch {
        watch_id: 1,
        price_cents: Some(20100),
        message: SourceMessage {
            chat_id: "deals".into(),
            chat_title: "Deals".into(),
            message_id: id,
            posted_at: 100,
            text: format!("\nControle <Blue> & Dock\nhttps://shop.example/item?a={id}&b=2\nCoupon"),
            message_link: Some("https://t.me/deals/1".into()),
            image: None,
        },
    }
}

#[test]
fn defaults_and_deduplication_keep_deliveries_separate_from_search_results() {
    let mut record = Record::default();

    assert!(record.settings.telegram_enabled && record.settings.desktop_enabled);

    add_matches(&mut record, &[result(1), result(1)]);

    assert_eq!(record.items.len(), 1);
    assert_eq!(record.status(None).pending, 1);
    assert_eq!(record.items[0].title, "Controle <Blue> & Dock");

    record.items[0].telegram = "sent".into();

    add_matches(&mut record, &[result(1), result(2)]);

    assert_eq!(record.items.len(), 2);
    assert_eq!(record.items[0].telegram, "sent");

    add_matches(&mut record, &[]);

    assert_eq!(record.items[1].telegram, "pending");

    record.items[1].telegram = "uncertain".into();

    assert_eq!(
        record
            .status(Some(AppMessage::NotificationFailed))
            .uncertain,
        1
    );
}

#[test]
fn cancelled_and_disabled_deliveries_can_resume_without_resending_successes() {
    let mut record = Record::default();

    add_matches(&mut record, &[result(1)]);
    record.items[0].telegram = "cancelled".into();

    add_matches(&mut record, &[result(1)]);

    assert_eq!(record.items[0].telegram, "pending");

    record.items[0].telegram = "disabled".into();
    record.items[0].telegram_signatures.clear();

    add_matches(&mut record, &[result(1)]);

    assert_eq!(record.items[0].telegram, "pending");
}

#[test]
fn disabled_channels_and_safe_captions_handle_missing_metadata() {
    let mut record = Record {
        settings: Settings {
            telegram_enabled: false,
            desktop_enabled: false,
            ..Settings::default()
        },
        ..Record::default()
    };
    let mut matched = result(1);
    matched.message.text = " ".into();
    matched.price_cents = None;
    add_matches(&mut record, &[matched]);

    assert_eq!(record.items[0].telegram, "disabled");
    assert!(record.items[0].desktop);
    assert!(caption(&record.items[0], "en").contains("Price unavailable"));
    assert!(caption(&record.items[0], "pt-BR").contains("Preço não informado"));

    record.items[0].message.message_link = Some("javascript:alert(1)".into());

    assert!(!caption(&record.items[0], "en").contains("href"));

    let mut record = Record::default();

    add_matches(&mut record, &[result(2)]);
    let text = caption(&record.items[0], "en");

    assert!(text.contains("Controle &lt;Blue&gt; &amp; Dock"));
    assert!(text.contains("R$ 201,00"));
    assert!(text.contains("https://shop.example/item?a=2&amp;b=2"));
    assert!(!text.contains("Coupon"));
}

#[test]
fn cash_prices_feed_the_notification_pipeline_without_installment_confusion() {
    let mut message = result(1).message;
    message.text = "Controle R$ 201 em dinheiro".into();
    let watch = crate::watch::Watch {
        id: 1,
        phrases: vec!["Controle".into()],
        max_price_cents: Some(50000),
        min_price_cents: None,
    };
    let matches = crate::promotion::match_messages(&[message], &[watch]);

    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].price_cents, Some(20100));

    let mut record = Record::default();

    add_matches(&mut record, &matches);

    assert!(caption(&record.items[0], "pt-BR").contains("R$ 201,00"));
}

#[tokio::test]
async fn repository_is_account_scoped_and_preserves_delivery_history() {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let path = std::env::temp_dir().join(format!(
        "skopos-notification-test-{}-{}.sqlite",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    ));
    let repository = Repository(path.clone());

    assert!(repository.load(77).await.unwrap().items.is_empty());

    let mut record = Record::default();

    add_matches(&mut record, &[result(1)]);
    repository.save(77, &record).await.unwrap();

    assert_eq!(repository.load(77).await.unwrap().items.len(), 1);
    assert!(repository.load(88).await.unwrap().items.is_empty());

    record.items[0].telegram = "sent".into();

    repository.save(77, &record).await.unwrap();

    assert_eq!(repository.load(77).await.unwrap().items[0].telegram, "sent");
    std::fs::remove_file(path).unwrap();

    let invalid = Repository(std::env::temp_dir().join("skopos-missing-dir/deeper/db.sqlite"));

    assert!(matches!(
        invalid.load(1).await,
        Err(AppMessage::NotificationStorage)
    ));
}

#[test]
fn existing_bot_settings_preserve_preferences_and_sent_history() {
    let mut record = Record::default();
    record.settings.telegram_enabled = false;
    add_matches(&mut record, &[result(1)]);
    record.items[0].telegram = "sent".into();
    let mut data = serde_json::to_value(&record).unwrap();
    data["settings"]["botUsername"] = json!("old_bot");
    data["failure"] = json!({"code": "notificationToken"});

    let restored = decode(&data.to_string()).unwrap();

    assert!(!restored.settings.telegram_enabled);
    assert!(restored.settings.desktop_enabled);
    assert_eq!(restored.items[0].telegram, "sent");
    assert!(restored.failure.is_none());

    data["failure"] = json!({"code": "notificationStartBot"});

    assert!(decode(&data.to_string()).unwrap().failure.is_none());
    assert!(matches!(
        decode("bad json"),
        Err(AppMessage::NotificationStorage)
    ));

    assert!(matches!(decode("{}"), Err(AppMessage::NotificationStorage)));
}

#[test]
fn separate_dividers_precede_offers_and_survive_reopen_and_new_days() {
    let mut record = Record {
        delivery_day: Some("05/10/2026".into()),
        ..Record::default()
    };

    assert!(next_delivery(&mut record).unwrap().is_none());

    add_matches(&mut record, &[result(1), result(2)]);

    let delivery = next_delivery(&mut record).unwrap().unwrap();

    assert!(matches!(&delivery, Delivery::Separator(day) if day == "05/10/2026"));
    assert_eq!(record.items[0].telegram, "pending");
    assert!(!caption(&record.items[0], "pt-BR").contains("05/10/2026"));

    record_telegram_result(&mut record, &delivery, &Ok(()));

    record = decode(&serde_json::to_string(&record).unwrap()).unwrap();
    let delivery = next_delivery(&mut record).unwrap().unwrap();

    assert!(matches!(delivery, Delivery::Promotion(0)));

    record_telegram_result(&mut record, &delivery, &Ok(()));
    let delivery = next_delivery(&mut record).unwrap().unwrap();

    assert!(matches!(delivery, Delivery::Promotion(1)));

    record_telegram_result(&mut record, &delivery, &Ok(()));

    assert!(next_delivery(&mut record).unwrap().is_none());

    add_matches(&mut record, &[result(1), result(2), result(3)]);

    let delivery = next_delivery(&mut record).unwrap().unwrap();

    assert!(matches!(delivery, Delivery::Promotion(2)));

    record_telegram_result(&mut record, &delivery, &Ok(()));

    record.delivery_day = Some("06/10/2026".into());
    add_matches(&mut record, &[result(4)]);
    let delivery = next_delivery(&mut record).unwrap().unwrap();

    assert!(matches!(&delivery, Delivery::Separator(day) if day == "06/10/2026"));
}

#[test]
fn old_caption_history_loads_without_resending_offers_and_dates_are_escaped() {
    let mut record = Record::default();

    add_matches(&mut record, &[result(1)]);
    record.items[0].telegram = "sent".into();
    let mut old = serde_json::to_value(&record).unwrap();
    old.as_object_mut().unwrap().remove("delivery_day");
    old.as_object_mut().unwrap().remove("separator");
    old["last_delivery_day"] = json!("05/10/2026");
    old["items"][0]["separator_day"] = json!("05/10/2026");
    let mut restored = decode(&old.to_string()).unwrap();

    assert!(restored.separator.is_none());
    assert!(next_delivery(&mut restored).unwrap().is_none());

    add_matches(&mut restored, &[result(1), result(2)]);

    assert!(matches!(
        next_delivery(&mut restored).unwrap(),
        Some(Delivery::Promotion(1))
    ));

    assert!(separator("<date>").contains("&lt;date&gt;"));
}

#[test]
fn rejected_dividers_retry_but_uncertain_dividers_block_offers_until_confirmation() {
    let mut record = Record {
        delivery_day: Some("05/10/2026".into()),
        ..Record::default()
    };

    add_matches(&mut record, &[result(1)]);
    let delivery = next_delivery(&mut record).unwrap().unwrap();

    record_telegram_result(&mut record, &delivery, &Err(AppMessage::NotificationFailed));

    assert_eq!(record.separator.as_ref().unwrap().telegram, "pending");
    assert_eq!(record.status(None).pending, 1);

    let delivery = next_delivery(&mut record).unwrap().unwrap();

    assert!(matches!(delivery, Delivery::Separator(_)));

    record_telegram_result(
        &mut record,
        &delivery,
        &Err(AppMessage::NotificationUncertain),
    );
    record = decode(&serde_json::to_string(&record).unwrap()).unwrap();

    assert_eq!(record.status(None).uncertain, 1);
    assert!(matches!(
        next_delivery(&mut record),
        Err(AppMessage::NotificationUncertain)
    ));

    assert_eq!(record.items[0].telegram, "pending");

    record.separator.as_mut().unwrap().telegram = "pending".into();

    let delivery = next_delivery(&mut record).unwrap().unwrap();

    record_telegram_result(&mut record, &delivery, &Ok(()));

    assert!(matches!(
        next_delivery(&mut record).unwrap(),
        Some(Delivery::Promotion(0))
    ));
}

#[test]
fn uncertain_offers_preserve_source_and_retry_without_repeating_sent_dividers() {
    let mut record = Record {
        delivery_day: Some("05/10/2026".into()),
        ..Record::default()
    };
    let mut offer = result(1);
    offer.message.image = Some("cached photo".into());
    add_matches(&mut record, &[offer, result(2)]);
    let delivery = next_delivery(&mut record).unwrap().unwrap();

    record_telegram_result(&mut record, &delivery, &Ok(()));

    let delivery = next_delivery(&mut record).unwrap().unwrap();
    record_telegram_result(
        &mut record,
        &delivery,
        &Err(AppMessage::NotificationUncertain),
    );

    assert_eq!(record.items[0].telegram, "uncertain");
    assert!(!record.items[0].message.text.is_empty());
    assert!(record.items[0].message.image.is_some());
    assert!(matches!(
        next_delivery(&mut record).unwrap(),
        Some(Delivery::Promotion(1))
    ));

    record.items[0].telegram = "pending".into();

    let delivery = next_delivery(&mut record).unwrap().unwrap();

    assert!(matches!(delivery, Delivery::Promotion(0)));

    record_telegram_result(&mut record, &delivery, &Ok(()));

    assert!(!record.items[0].message.text.is_empty());
    assert!(record.items[0].message.image.is_none());
    assert_eq!(record.separator.as_ref().unwrap().telegram, "sent");
}

#[test]
fn suppresses_reposts_but_sends_when_either_price_or_offer_link_changes() {
    let mut record = Record::default();
    let first = result(1);

    add_matches(&mut record, std::slice::from_ref(&first));
    record_telegram_result(&mut record, &Delivery::Promotion(0), &Ok(()));
    let mut repost = first.clone();
    repost.message.message_id = 2;
    repost.message.chat_id = "another-chat".into();
    repost.message.message_link = Some("https://t.me/other/2".into());

    add_matches(&mut record, std::slice::from_ref(&repost));

    assert_eq!(record.items[1].telegram, "suppressed");
    assert!(record.items[1].desktop);

    repost.message.message_id = 3;

    repost.price_cents = Some(20000);
    add_matches(&mut record, std::slice::from_ref(&repost));

    assert_eq!(record.items[2].telegram, "pending");

    repost.message.message_id = 4;
    repost.message.text = "Controle https://shop.example/different".into();
    add_matches(&mut record, &[repost]);

    assert_eq!(record.items[3].telegram, "pending");
}

#[test]
fn unknown_price_and_link_reservations_survive_restart_and_uncertain_sends() {
    let mut record = Record::default();
    let mut first = result(1);
    first.price_cents = None;
    first.message.text = "Controle without price or offer link".into();
    add_matches(&mut record, std::slice::from_ref(&first));
    record_telegram_result(
        &mut record,
        &Delivery::Promotion(0),
        &Err(AppMessage::NotificationUncertain),
    );
    let mut reopened = decode(&serde_json::to_string(&record).unwrap()).unwrap();

    first.message.message_id = 2;

    add_matches(&mut reopened, &[first]);

    assert_eq!(reopened.items[0].telegram, "uncertain");
    assert_eq!(reopened.items[1].telegram, "suppressed");
    assert_eq!(reopened.status(None).uncertain, 1);
}
