use super::{
    commands::repository,
    content::{caption, separator},
    repository::{Item, Record, Separator, Signature},
    DeliveryStatus, NotificationState,
};
use crate::{
    app_message::AppMessage,
    promotion::{ProductMatch, SourceMessage},
};
use std::sync::atomic::Ordering;
use tauri::{AppHandle, Emitter, Manager};
#[cfg(not(feature = "e2e"))]
use tauri_plugin_notification::NotificationExt;

pub(crate) async fn enqueue(
    app: &AppHandle,
    account: i64,
    matches: &[ProductMatch],
    day: String,
) -> Result<(), AppMessage> {
    let state = app.state::<NotificationState>();
    let operation = state.operation.lock().await;
    let repository = repository(app)?;
    let mut record = repository.load(account).await?;
    record.delivery_day = Some(day);
    add_matches(&mut record, matches);
    repository.save(account, &record).await?;
    drop(operation);
    start(app.clone(), account);

    Ok(())
}

pub(super) fn add_matches(record: &mut Record, matches: &[ProductMatch]) {
    let mut ordered: Vec<_> = matches.iter().collect();
    ordered.sort_by_key(|result| {
        (
            result.message.posted_at,
            result.message.message_id,
            result.watch_id,
        )
    });

    for result in ordered {
        add_match(record, result);
    }
}

fn add_match(record: &mut Record, result: &ProductMatch) {
    let signature = Signature {
        watch_id: result.watch_id,
        price: result.price_cents,
        link: super::content::offer_link(&result.message.text),
    };
    let telegram_duplicate = is_duplicate(record, &signature, true);
    let desktop_duplicate = is_duplicate(record, &signature, false);

    if let Some(item) = record
        .items
        .iter_mut()
        .find(|item| same_message(&item.message, &result.message))
    {
        update_item(
            item,
            result,
            &signature,
            telegram_duplicate || !record.settings.telegram_enabled,
            desktop_duplicate,
        );

        return;
    }

    let title = result
        .message
        .text
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("Promoção")
        .chars()
        .take(300)
        .collect();
    let telegram = if !record.settings.telegram_enabled {
        "disabled"
    } else if telegram_duplicate {
        "suppressed"
    } else {
        "pending"
    };
    let desktop = !record.settings.desktop_enabled || desktop_duplicate;
    record.items.push(Item {
        message: result.message.clone(),
        price: result.price_cents,
        title,
        telegram: telegram.into(),
        desktop,
        telegram_signatures: if telegram == "pending" {
            vec![signature.clone()]
        } else {
            Vec::new()
        },
        desktop_signatures: if desktop { Vec::new() } else { vec![signature] },
    });
}

fn update_item(
    item: &mut Item,
    result: &ProductMatch,
    signature: &Signature,
    telegram_duplicate: bool,
    desktop_duplicate: bool,
) {
    if matches!(item.telegram.as_str(), "cancelled" | "disabled") && !telegram_duplicate {
        item.telegram = "pending".into();
        item.message = result.message.clone();
        item.price = result.price_cents;
    }

    if item.telegram == "pending"
        && !telegram_duplicate
        && !item.telegram_signatures.contains(signature)
    {
        item.telegram_signatures.push(signature.clone());
    }

    if !item.desktop && !desktop_duplicate && !item.desktop_signatures.contains(signature) {
        item.desktop_signatures.push(signature.clone());
    }
}

fn is_duplicate(record: &Record, signature: &Signature, telegram: bool) -> bool {
    let previous = if telegram {
        &record.last_telegram
    } else {
        &record.last_desktop
    };
    let reserved = record.items.iter().rev().find_map(|item| {
        let signatures = if telegram {
            if !matches!(item.telegram.as_str(), "pending" | "uncertain") {
                return None;
            }
            &item.telegram_signatures
        } else {
            if item.desktop {
                return None;
            }
            &item.desktop_signatures
        };
        signatures
            .iter()
            .find(|candidate| candidate.watch_id == signature.watch_id)
    });
    reserved.or_else(|| previous.get(&signature.watch_id)) == Some(signature)
}

pub(crate) fn start(app: AppHandle, account: i64) {
    let generation = app
        .state::<NotificationState>()
        .generation
        .load(Ordering::SeqCst);
    tauri::async_runtime::spawn(async move {
        let state = app.state::<NotificationState>();
        let Ok(_worker) = state.worker.try_lock() else {
            return;
        };

        if generation != state.generation.load(Ordering::SeqCst) {
            return;
        }

        let result = deliver(&app, account, generation).await;
        let status = finish(&app, account, result.err())
            .await
            .unwrap_or_else(|error| DeliveryStatus {
                failure: Some(error),
                ..DeliveryStatus::default()
            });

        if generation == state.generation.load(Ordering::SeqCst) {
            // Progress events are best-effort; persisted delivery status remains authoritative.
            let _ = app.emit("notifications:status", status);
        }
    });
}

async fn finish(
    app: &AppHandle,
    account: i64,
    failure: Option<AppMessage>,
) -> Result<DeliveryStatus, AppMessage> {
    let state = app.state::<NotificationState>();
    let _operation = state.operation.lock().await;
    let repository = repository(app)?;
    let mut record = repository.load(account).await?;
    record.failure = failure;

    if let Some(AppMessage::NotificationRateLimit { seconds }) = &record.failure {
        record.next_attempt_at = now().saturating_add(*seconds);
    }

    repository.save(account, &record).await?;

    Ok(record.status(None))
}

async fn deliver(app: &AppHandle, account: i64, generation: u64) -> Result<(), AppMessage> {
    let state = app.state::<NotificationState>();
    let repository = repository(app)?;
    let operation = state.operation.lock().await;
    let mut record = repository.load(account).await?;
    revalidate(app, &mut record).await?;
    repository.save(account, &record).await?;
    let desktop_failure = deliver_desktop(app, account, &mut record).await?;
    drop(operation);

    if !record.settings.telegram_enabled {
        return desktop_failure.map_or(Ok(()), Err);
    }

    if record.next_attempt_at > now() {
        return Err(AppMessage::NotificationRateLimit {
            seconds: record.next_attempt_at.saturating_sub(now()),
        });
    }

    loop {
        if generation != state.generation.load(Ordering::SeqCst) {
            return Ok(());
        }

        let operation = state.operation.lock().await;
        record = repository.load(account).await?;
        revalidate(app, &mut record).await?;
        let Some(delivery) = next_delivery(&mut record)? else {
            return desktop_failure.map_or(Ok(()), Err);
        };
        let (text, image) = match &delivery {
            Delivery::Separator(day) => (separator(day), None),
            Delivery::Promotion(index) => {
                let item = &record.items[*index];
                (
                    caption(item, &record.settings.language),
                    item.message.image.clone(),
                )
            }
        };
        // Persist uncertainty before sending: a crash must not silently cause a duplicate.
        repository.save(account, &record).await?;
        drop(operation);

        let interrupted = state.wake.notified();
        tokio::pin!(interrupted);
        interrupted.as_mut().enable();

        if generation != state.generation.load(Ordering::SeqCst) {
            return Ok(());
        }

        let result = tokio::select! {
            result = crate::telegram::saved::send(app, account, text, image.as_deref()) => result,
            _ = interrupted => return Ok(()),
        };
        let operation = state.operation.lock().await;
        record = repository.load(account).await?;
        record_telegram_result(&mut record, &delivery, &result);
        repository.save(account, &record).await?;
        drop(operation);
        result?;

        let interrupted = state.wake.notified();
        tokio::pin!(interrupted);
        interrupted.as_mut().enable();

        if generation != state.generation.load(Ordering::SeqCst) {
            return Ok(());
        }
        tokio::select! {
            _ = tokio::time::sleep(std::time::Duration::from_secs(1)) => {},
            _ = interrupted => return Ok(()),
        }
    }
}

async fn deliver_desktop(
    app: &AppHandle,
    account: i64,
    record: &mut Record,
) -> Result<Option<AppMessage>, AppMessage> {
    let count = record.items.iter().filter(|item| !item.desktop).count();

    if count == 0 || !record.settings.desktop_enabled {
        return Ok(None);
    }

    let result = desktop(app, count, &record.settings.language);

    if result.is_ok() {
        record_desktop_success(record);
        repository(app)?.save(account, record).await?;
    }

    Ok(result.err())
}

pub(super) enum Delivery {
    Separator(String),
    Promotion(usize),
}

pub(super) fn next_delivery(record: &mut Record) -> Result<Option<Delivery>, AppMessage> {
    let Some(index) = record
        .items
        .iter()
        .position(|item| item.telegram == "pending")
    else {
        return Ok(None);
    };

    if record
        .separator
        .as_ref()
        .is_some_and(|separator| separator.telegram == "uncertain")
    {
        return Err(AppMessage::NotificationUncertain);
    }

    if let Some(day) = &record.delivery_day {
        let already_sent = record
            .separator
            .as_ref()
            .is_some_and(|separator| separator.day == *day && separator.telegram == "sent");

        if !already_sent {
            record.separator = Some(Separator {
                day: day.clone(),
                telegram: "uncertain".into(),
            });

            return Ok(Some(Delivery::Separator(day.clone())));
        }
    }

    record.items[index].telegram = "uncertain".into();

    Ok(Some(Delivery::Promotion(index)))
}

pub(super) fn record_telegram_result(
    record: &mut Record,
    delivery: &Delivery,
    result: &Result<(), AppMessage>,
) {
    let status = match result {
        Ok(()) => "sent",
        Err(AppMessage::NotificationUncertain) => "uncertain",
        Err(_) => "pending",
    };
    match delivery {
        Delivery::Separator(_) => {
            if let Some(separator) = &mut record.separator {
                separator.telegram = status.into();
            }
        }

        Delivery::Promotion(index) => {
            let item = &mut record.items[*index];
            item.telegram = status.into();

            if result.is_ok() {
                for signature in &item.telegram_signatures {
                    record
                        .last_telegram
                        .insert(signature.watch_id, signature.clone());
                }
                item.message.image = None;

                if item.desktop {
                    item.message.text.clear();
                }
            }
        }
    }
}

async fn revalidate(app: &AppHandle, record: &mut Record) -> Result<(), AppMessage> {
    let directory = crate::telegram::monitoring::directory(app)?;
    let watches = crate::watch::WatchRepository::new(directory.join("watches.sqlite"))
        .list()
        .await
        .map_err(|error| error.message())?;

    for item in &mut record.items {
        if item.telegram != "pending" && item.desktop {
            continue;
        }

        let matches =
            crate::promotion::match_messages(std::slice::from_ref(&item.message), &watches);
        let is_valid = |signature: &Signature| {
            matches
                .iter()
                .any(|result| result.watch_id == signature.watch_id)
        };
        let is_legacy = item.telegram_signatures.is_empty() && item.desktop_signatures.is_empty();
        item.telegram_signatures.retain(is_valid);
        item.desktop_signatures.retain(is_valid);

        if item.telegram == "pending"
            && ((is_legacy && matches.is_empty())
                || (!is_legacy && item.telegram_signatures.is_empty()))
        {
            item.telegram = "cancelled".into();
        }

        if (is_legacy && matches.is_empty()) || (!is_legacy && item.desktop_signatures.is_empty()) {
            item.desktop = true;
        }
    }

    Ok(())
}

fn record_desktop_success(record: &mut Record) {
    for item in record.items.iter_mut().filter(|item| !item.desktop) {
        item.desktop = true;

        for signature in &item.desktop_signatures {
            record
                .last_desktop
                .insert(signature.watch_id, signature.clone());
        }
    }
}

fn desktop(app: &AppHandle, count: usize, language: &str) -> Result<(), AppMessage> {
    let body = if language == "en" {
        format!("{count} new promotions found")
    } else {
        format!("{count} novas promoções encontradas")
    };
    #[cfg(feature = "e2e")]
    {
        let state = app.state::<NotificationState>();
        let mut fixture = state.fixture.lock().unwrap();

        if fixture.desktop_error {
            return Err(AppMessage::NotificationDesktop);
        }

        fixture.desktop.push(body);

        Ok(())
    }
    #[cfg(not(feature = "e2e"))]
    app.notification()
        .builder()
        .title("Skopos")
        .body(body)
        .show()
        .map_err(|_| AppMessage::NotificationDesktop)
}

fn same_message(left: &SourceMessage, right: &SourceMessage) -> bool {
    left.chat_id == right.chat_id && left.message_id == right.message_id
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs())
}
