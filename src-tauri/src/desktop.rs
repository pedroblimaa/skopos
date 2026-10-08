#[cfg(feature = "e2e")]
pub(crate) mod fixture;
mod runtime;
pub(crate) mod startup;
#[cfg(test)]
mod tests;

pub(crate) use runtime::{setup, show, window_event};
pub(crate) use startup::{DesktopState, StartupSettings};
