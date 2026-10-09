use super::{
    workflow::{self, SearchRun},
    SearchState,
};
use crate::{
    app_message::AppMessage,
    promotion::{match_messages, ResultRepository, SearchResults},
    telegram::{
        chats::repository::ChatRepository,
        monitoring::{self, MonitoringState, Record},
        AuthState,
    },
    watch::WatchRepository,
};
use chrono::Timelike;
use std::{collections::HashSet, sync::atomic::Ordering};
use tauri::{AppHandle, Emitter, Manager};

pub(crate) async fn manual(
    app: &AppHandle,
    notification_day: String,
) -> Result<SearchResults, AppMessage> {
    execute(app, Some(notification_day))
        .await?
        .ok_or(AppMessage::SearchFailed)
}

pub(crate) async fn automatic(app: &AppHandle) -> Result<(), AppMessage> {
    let auth = app.state::<AuthState>();
    let context = auth.client(app).await.map_err(|error| error.message())?;

    if !super::super::client::status_for(&context.client)
        .await
        .map_err(|error| error.message())?
        .authorized()
    {
        return Ok(());
    }

    match execute(app, None).await {
        Ok(_)
        | Err(
            AppMessage::SearchBusy
            | AppMessage::SearchNeedsProducts
            | AppMessage::SearchNeedsChats
            | AppMessage::AuthCancelled,
        ) => Ok(()),
        Err(error) => Err(error),
    }
}

async fn execute(
    app: &AppHandle,
    notification_day: Option<String>,
) -> Result<Option<SearchResults>, AppMessage> {
    let auth = app.state::<AuthState>();
    let state = app.state::<SearchState>();
    let monitoring = app.state::<MonitoringState>();
    let _operation = state
        .operation
        .try_lock()
        .map_err(|_| AppMessage::SearchBusy)?;
    let _monitoring = monitoring.operation.lock().await;
    let generation = auth.login.lock().await.generation;
    let cancellation = state.cancellation.load(Ordering::SeqCst);
    let directory = monitoring::directory(app)?;
    let context = auth.client(app).await.map_err(|error| error.message())?;
    let account = context
        .local_account_id()
        .await
        .map_err(|error| error.message())?;
    let monitor_repository = monitoring::repository(app)?;
    let mut record = monitor_repository.load(account).await?;
    let watches = WatchRepository::new(directory.join("watches.sqlite"))
        .list()
        .await
        .map_err(|error| error.message())?;
    let now = monitoring::now(app);
    let day = now.format("%Y-%m-%d").to_string();
    let is_automatic = notification_day.is_none();

    if !monitoring.can_run() {
        return Err(AppMessage::AuthCancelled);
    }

    flush_pending(app, account, &mut record, &watches).await?;
    crate::notification::resume(app.clone(), account);

    if is_automatic && (!monitoring.can_run() || !record.is_due(&day, now.hour())) {
        return Ok(None);
    }

    if watches.is_empty() {
        return Err(AppMessage::SearchNeedsProducts);
    }

    let selected = ChatRepository::new(directory.join("chats.sqlite"))
        .get(account)
        .await
        .map_err(|error| error.message())?;

    if selected.is_empty() {
        return Err(AppMessage::SearchNeedsChats);
    }

    let repository = ResultRepository::new(directory.join("results.sqlite"));
    let (saved, _) = repository
        .load(account)
        .await
        .map_err(|_| AppMessage::SearchStorage)?;
    let since = search_since(&record, &selected, now.timestamp(), is_automatic);
    let run = SearchRun {
        api: &context.client,
        auth: &auth,
        state: &state,
        generation,
        cancellation,
        since,
        until: now.timestamp(),
        cached_photos: saved
            .iter()
            .filter_map(|message| {
                message
                    .image
                    .as_ref()
                    .map(|image| ((message.chat_id.clone(), message.message_id), image.clone()))
            })
            .collect(),
        saved_messages: saved
            .into_iter()
            .map(|message| (message.chat_id, message.message_id))
            .collect(),
        checkpoints: if is_automatic {
            record.checkpoints.clone()
        } else {
            Default::default()
        },
    };
    run.ensure_current()
        .await
        .map_err(|error| error.message())?;

    if is_automatic {
        record.claim(day, now.hour(), now.timestamp());
        monitor_repository.save(account, &record).await?;
        monitoring.is_running.store(true, Ordering::SeqCst);
        *monitoring.failure.lock().await = None;
        monitoring::publish(app, account).await;
    }

    let inputs = SearchInputs {
        selected,
        watches,
        notification_day,
    };
    let result = search_and_save(app, &run, inputs, &mut record).await;

    if is_automatic {
        finish_attempt(app, &run, &mut record, &result).await?;
    }

    result.map(Some)
}

async fn finish_attempt(
    app: &AppHandle,
    run: &SearchRun<'_>,
    record: &mut Record,
    result: &Result<SearchResults, AppMessage>,
) -> Result<(), AppMessage> {
    let monitoring = app.state::<MonitoringState>();
    monitoring.is_running.store(false, Ordering::SeqCst);
    let account = run.auth.account_id(app).await?;

    if let Err(error) = result {
        record.failure = Some(error.clone());
        monitoring::repository(app)?.save(account, record).await?;
    }

    if run.ensure_current().await.is_ok() {
        monitoring::publish(app, account).await;
    }

    Ok(())
}

fn search_since(
    record: &Record,
    selected: &[super::super::chats::Chat],
    now: i64,
    automatic: bool,
) -> i64 {
    if !automatic {
        return now - 86_400;
    }
    selected
        .iter()
        .map(|chat| {
            record
                .checkpoints
                .get(&chat.id)
                .map_or(now - 86_400, |checkpoint| checkpoint.checked_at)
        })
        .min()
        .unwrap_or(now - 86_400)
}

struct SearchInputs {
    selected: Vec<super::super::chats::Chat>,
    watches: Vec<crate::watch::Watch>,
    notification_day: Option<String>,
}

async fn search_and_save(
    app: &AppHandle,
    run: &SearchRun<'_>,
    inputs: SearchInputs,
    record: &mut Record,
) -> Result<SearchResults, AppMessage> {
    let is_automatic = inputs.notification_day.is_none();
    let account = run
        .auth
        .client(app)
        .await
        .map_err(|error| error.message())?
        .local_account_id()
        .await
        .map_err(|error| error.message())?;
    let (messages, summary, checkpoints) = workflow::search(run, inputs.selected, &inputs.watches)
        .await
        .map_err(|error| error.message())?;
    run.ensure_current()
        .await
        .map_err(|error| error.message())?;
    let directory = monitoring::directory(app)?;
    let current_watches = WatchRepository::new(directory.join("watches.sqlite"))
        .list()
        .await
        .map_err(|error| error.message())?;
    let notifications = match_messages(&messages, &current_watches);
    let mut committed = Record {
        enabled: record.enabled,
        days: record.days.clone(),
        checkpoints: record.checkpoints.clone(),
        pending: notifications,
        pending_day: inputs
            .notification_day
            .unwrap_or_else(|| monitoring::now(app).format("%d/%m/%Y").to_string()),
        last_attempt: record.last_attempt,
        failure: summary.failure.clone(),
    };

    if is_automatic {
        committed.checkpoints.extend(checkpoints);
    }

    let repository = ResultRepository::new(directory.join("results.sqlite"));
    let monitoring_data =
        serde_json::to_string(&committed).map_err(|_| AppMessage::MonitoringStorage)?;
    repository
        .merge_monitoring(account, messages, summary, &monitoring_data)
        .await
        .map_err(|_| AppMessage::SearchStorage)?;
    *record = committed;
    let (messages, summary) = repository
        .load(account)
        .await
        .map_err(|_| AppMessage::SearchStorage)?;
    let results = SearchResults {
        matches: match_messages(&messages, &current_watches),
        summary,
    };
    run.ensure_current()
        .await
        .map_err(|error| error.message())?;

    if let Err(error) = flush_pending(app, account, record, &current_watches).await {
        let _ = app.emit(
            "notifications:status",
            crate::notification::DeliveryStatus {
                failure: Some(error),
                ..Default::default()
            },
        );
    }

    run.ensure_current()
        .await
        .map_err(|error| error.message())?;

    if is_automatic {
        // Consumers reload local results, keeping event payloads small and account scoped.
        let _ = app.emit("search:updated", account);
    }

    Ok(results)
}

async fn flush_pending(
    app: &AppHandle,
    account: i64,
    record: &mut Record,
    watches: &[crate::watch::Watch],
) -> Result<(), AppMessage> {
    if record.pending.is_empty() {
        return Ok(());
    }

    let messages: Vec<_> = record
        .pending
        .iter()
        .map(|result| result.message.clone())
        .collect();
    let valid = match_messages(&messages, watches);
    let ids: HashSet<_> = record
        .pending
        .iter()
        .map(|result| {
            (
                result.watch_id,
                result.message.chat_id.clone(),
                result.message.message_id,
            )
        })
        .collect();
    let candidates: Vec<_> = valid
        .into_iter()
        .filter(|result| {
            ids.contains(&(
                result.watch_id,
                result.message.chat_id.clone(),
                result.message.message_id,
            ))
        })
        .collect();
    crate::notification::enqueue(app, account, &candidates, record.pending_day.clone()).await?;
    record.pending.clear();
    monitoring::repository(app)?.save(account, record).await
}
