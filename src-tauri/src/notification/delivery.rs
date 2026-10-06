use super::{
    commands::repository,
    content::{caption, separator},
    repository::{Item, Record, Separator},
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
    // Pending matches no longer meeting current criteria must not be delivered later.
    for item in &mut record.items {
        if item.telegram == "pending"
            && !matches
                .iter()
                .any(|result| same_message(&item.message, &result.message))
        {
            item.telegram = "cancelled".into();
            item.desktop = true;
        }
    }
    for result in matches {
        if let Some(item) = record
            .items
            .iter_mut()
            .find(|item| same_message(&item.message, &result.message))
        {
            if matches!(item.telegram.as_str(), "cancelled" | "disabled")
                && record.settings.telegram_enabled
            {
                item.telegram = "pending".into();
                item.message = result.message.clone();
                item.price = result.price_cents;
            }
            continue;
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
        record.items.push(Item {
            message: result.message.clone(),
            price: result.price_cents,
            title,
            telegram: if record.settings.telegram_enabled {
                "pending"
            } else {
                "disabled"
            }
            .into(),
            desktop: !record.settings.desktop_enabled,
        });
    }
}

pub(super) fn start(app: AppHandle, account: i64) {
    let generation = app
        .state::<NotificationState>()
        .generation
        .load(Ordering::SeqCst);
    tauri::async_runtime::spawn(async move {
        let state = app.state::<NotificationState>();
        let _worker = state.worker.lock().await;
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
    let desktop_count = record.items.iter().filter(|item| !item.desktop).count();
    let desktop_failure = if desktop_count > 0 && record.settings.desktop_enabled {
        let result = desktop(app, desktop_count, &record.settings.language);
        if result.is_ok() {
            for item in &mut record.items {
                item.desktop = true;
            }
            repository.save(account, &record).await?;
        }
        result.err()
    } else {
        None
    };
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
                item.message.image = None;
                item.message.text.clear();
            }
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
