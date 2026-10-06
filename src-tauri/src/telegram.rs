mod api;
pub mod chats;
mod client;
#[cfg(any(test, feature = "e2e"))]
pub(crate) mod e2e;
mod error;
pub mod phone;
pub mod qr;
pub(crate) mod saved;
pub mod search;
pub mod session;
mod state;

pub use state::AuthState;
