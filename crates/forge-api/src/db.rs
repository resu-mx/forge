//! Run a closure against the database connection from an async handler.

use rusqlite::Connection;

use crate::error::ApiError;
use crate::state::SharedState;
use forge_core::ForgeError;

/// Run `f` with the connection locked.
///
/// - With the `tokio` feature (native server) the closure runs on the blocking
///   pool so SQLite never stalls an async worker thread.
/// - Without it (browser host) it runs inline: the Worker has one thread and no
///   runtime, and the OPFS-backed database is synchronous.
///
/// `FnOnce` because handlers move request bodies into the closure.
#[cfg(feature = "tokio")]
pub async fn with_conn<T, F>(state: &SharedState, f: F) -> Result<T, ApiError>
where
    F: FnOnce(&Connection) -> Result<T, ForgeError> + Send + 'static,
    T: Send + 'static,
{
    let state = state.clone();
    tokio::task::spawn_blocking(move || {
        let forge = state.lock()?;
        f(forge.conn())
    })
    .await
    .map_err(|e| ForgeError::Internal(format!("blocking task failed: {e}")))?
    .map_err(ApiError::from)
}

#[cfg(not(feature = "tokio"))]
pub async fn with_conn<T, F>(state: &SharedState, f: F) -> Result<T, ApiError>
where
    F: FnOnce(&Connection) -> Result<T, ForgeError> + Send + 'static,
    T: Send + 'static,
{
    let forge = state.lock()?;
    f(forge.conn()).map_err(ApiError::from)
}
