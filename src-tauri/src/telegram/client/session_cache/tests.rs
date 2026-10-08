use super::*;

#[test]
fn a_poisoned_cache_preserves_generation_checks_and_can_still_revoke() {
    let cache = std::sync::Arc::new(SessionCache::default());
    cache.reset(Some(SessionStatus::signed_in("Pedro".into())));
    let generation = cache.snapshot().generation;
    let worker = cache.clone();

    assert!(std::thread::spawn(move || {
        let _guard = worker.value.lock().unwrap();
        panic!("simulate an interrupted cache owner");
    })
    .join()
    .is_err());

    assert!(cache.snapshot().status.unwrap().authorized());
    assert!(!cache.store(generation.wrapping_sub(1), SessionStatus::signed_out()));
    assert!(cache.store(generation, SessionStatus::signed_in("Pedro".into())));
    assert!(cache.revoke(generation));
    assert!(!cache.snapshot().status.unwrap().authorized());
    assert!(!cache.revoke(generation));

    cache.reset(None);

    assert!(cache.snapshot().status.is_none());
    assert!(!cache.store(generation, SessionStatus::signed_in("Pedro".into())));
}
