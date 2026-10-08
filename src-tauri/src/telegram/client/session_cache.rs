use super::SessionStatus;
use std::sync::Mutex;

#[derive(Default)]
pub(in crate::telegram) struct SessionCache {
    pub validation: tokio::sync::Mutex<()>,
    value: Mutex<Snapshot>,
}

#[derive(Clone, Default)]
pub(in crate::telegram) struct Snapshot {
    pub generation: u64,
    pub status: Option<SessionStatus>,
}

impl SessionCache {
    pub fn snapshot(&self) -> Snapshot {
        self.value
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .clone()
    }

    pub fn store(&self, generation: u64, status: SessionStatus) -> bool {
        let mut value = self.value.lock().unwrap_or_else(|error| error.into_inner());

        if value.generation != generation {
            return false;
        }
        value.status = Some(status);
        true
    }

    pub fn reset(&self, status: Option<SessionStatus>) {
        let mut value = self.value.lock().unwrap_or_else(|error| error.into_inner());
        value.generation = value.generation.wrapping_add(1);
        value.status = status;
    }

    pub fn revoke(&self, generation: u64) -> bool {
        let mut value = self.value.lock().unwrap_or_else(|error| error.into_inner());

        if value.generation != generation
            || !value.status.as_ref().is_some_and(SessionStatus::authorized)
        {
            return false;
        }
        value.generation = value.generation.wrapping_add(1);
        value.status = Some(SessionStatus::signed_out());
        true
    }
}
