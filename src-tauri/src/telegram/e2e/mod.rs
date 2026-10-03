//! Telegram API responses and controls available only in test builds.
pub(in crate::telegram) mod chats;
#[cfg(feature = "e2e")]
pub(crate) mod commands;
#[cfg(test)]
mod context;
mod fixture;
pub(in crate::telegram) mod history;
mod responses;

#[cfg(test)]
pub(in crate::telegram) use context::test_context;
pub(crate) use fixture::FixtureState;
#[cfg(feature = "e2e")]
pub(crate) use fixture::Scenario;
#[cfg(test)]
pub(in crate::telegram) use responses::{authorization, password};
