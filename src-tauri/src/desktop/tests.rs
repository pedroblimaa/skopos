use super::*;
use std::sync::atomic::AtomicU64;

static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct Directory(std::path::PathBuf);

impl Directory {
    fn new() -> Self {
        let id = NEXT_DIRECTORY.fetch_add(1, Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!("skopos-startup-{}-{id}", std::process::id()));
        std::fs::create_dir_all(&path).unwrap();

        Self(path)
    }
}

impl Drop for Directory {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn startup_defaults_and_saved_preferences_survive_reopening() {
    let directory = Directory::new();
    let path = directory.0.join("startup.json");
    let mut settings = load_startup(&path).unwrap();
    let mut registrations = Vec::new();

    assert!(settings.enabled);
    assert!(settings.failure.is_none());

    let saved = save_startup(&directory.0, &mut settings, false, |enabled| {
        registrations.push(enabled);
        Ok(())
    })
    .unwrap();

    assert!(!saved.enabled);
    assert!(!settings.enabled);
    assert!(!load_startup(&path).unwrap().enabled);
    assert_eq!(registrations, [false]);
    assert!(!directory.0.join("startup.tmp").exists());
}

#[test]
fn failed_registration_preserves_the_file_and_cached_preference() {
    let directory = Directory::new();
    let path = directory.0.join("startup.json");
    let mut settings = StartupSettings::default();
    save_startup(&directory.0, &mut settings, true, |_| Ok(())).unwrap();
    let original = std::fs::read(&path).unwrap();

    let error = save_startup(&directory.0, &mut settings, false, |_| {
        Err(AppMessage::StartupFailed)
    })
    .err()
    .unwrap();

    assert_eq!(error, AppMessage::StartupFailed);
    assert!(settings.enabled);
    assert_eq!(std::fs::read(&path).unwrap(), original);
}

#[test]
fn failed_persistence_rolls_back_registration_even_when_rollback_fails() {
    let directory = Directory::new();
    std::fs::create_dir(directory.0.join("startup.json")).unwrap();
    let mut settings = StartupSettings::default();
    let mut registrations = Vec::new();

    let error = save_startup(&directory.0, &mut settings, false, |enabled| {
        registrations.push(enabled);
        if enabled {
            Err(AppMessage::StartupFailed)
        } else {
            Ok(())
        }
    })
    .err()
    .unwrap();

    assert_eq!(error, AppMessage::MonitoringStorage);
    assert!(settings.enabled);
    assert_eq!(registrations, [false, true]);
}

#[test]
fn unreadable_and_corrupt_startup_files_report_storage_failure() {
    let directory = Directory::new();
    let path = directory.0.join("startup.json");
    std::fs::write(&path, b"{broken").unwrap();

    assert_eq!(
        load_startup(&path).err(),
        Some(AppMessage::MonitoringStorage)
    );

    std::fs::remove_file(&path).unwrap();
    std::fs::create_dir(&path).unwrap();

    assert_eq!(
        load_startup(&path).err(),
        Some(AppMessage::MonitoringStorage)
    );
}

#[tokio::test]
async fn shutdown_suspends_monitoring_wakes_the_worker_and_is_idempotent() {
    let state = MonitoringState::default();

    assert!(state.can_run());
    assert!(begin_shutdown(&state));
    assert!(state.is_suspended.load(Ordering::SeqCst));
    assert!(!state.can_run());
    tokio::time::timeout(std::time::Duration::from_secs(1), state.wake.notified())
        .await
        .unwrap();

    assert!(!begin_shutdown(&state));
}

#[tokio::test]
async fn shutdown_waits_for_active_operations_and_keeps_them_reserved_until_exit() {
    let search = crate::telegram::search::SearchState::default();
    let monitoring = MonitoringState::default();
    let notifications = crate::notification::NotificationState::default();
    let search_guard = search.cancel_and_wait().await;
    let monitoring_guard = monitoring.operation.lock().await;
    let notification_guard = notifications.cancel_and_wait().await;
    let drain = drain_operations(&search, &monitoring, &notifications);
    tokio::pin!(drain);
    let wait = std::time::Duration::from_millis(10);

    assert!(tokio::time::timeout(wait, &mut drain).await.is_err());

    drop(search_guard);

    assert!(tokio::time::timeout(wait, &mut drain).await.is_err());

    drop(monitoring_guard);

    assert!(tokio::time::timeout(wait, &mut drain).await.is_err());

    drop(notification_guard);
    let reserved = tokio::time::timeout(std::time::Duration::from_secs(1), &mut drain)
        .await
        .unwrap();

    assert!(monitoring.operation.try_lock().is_err());
    assert!(tokio::time::timeout(wait, search.cancel_and_wait())
        .await
        .is_err());
    assert!(tokio::time::timeout(wait, notifications.cancel_and_wait())
        .await
        .is_err());

    drop(reserved);

    assert!(monitoring.operation.try_lock().is_ok());
    drop(
        tokio::time::timeout(wait, search.cancel_and_wait())
            .await
            .unwrap(),
    );
    drop(
        tokio::time::timeout(wait, notifications.cancel_and_wait())
            .await
            .unwrap(),
    );
}
