//! Shared application state.

use std::sync::{Arc, Mutex, MutexGuard};

use forge_core::ForgeError;
use forge_sdk::Forge;

/// Owns the database. A single connection behind a mutex, as decided in the R1
/// server ADR: SQLite serialises writers anyway, and the browser host has exactly
/// one connection.
pub struct AppState {
    forge: Mutex<Forge>,
}

/// What axum hands to handlers (`State<SharedState>`).
pub type SharedState = Arc<AppState>;

impl AppState {
    pub fn new(forge: Forge) -> SharedState {
        Arc::new(Self { forge: Mutex::new(forge) })
    }

    pub(crate) fn lock(&self) -> Result<MutexGuard<'_, Forge>, ForgeError> {
        self.forge
            .lock()
            .map_err(|_| ForgeError::Internal("database lock poisoned".into()))
    }
}
