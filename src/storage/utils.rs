use std::time::{SystemTime, UNIX_EPOCH};
use crate::storage::errors::{StorageError, StorageResult};

/// Type alias for standard timestamps (seconds since UNIX epoch)
pub type Timestamp = u64;

/// Returns the current time as a `Timestamp`.
/// If time appears to have gone backwards (due to clock adjustment), returns an error.
pub fn now() -> StorageResult<Timestamp> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| StorageError::Other { 
            details: format!("Failed to get timestamp: {}", e) 
        })
        .map(|d| d.as_secs())
}

/// Returns the current time as a `Timestamp`, with a fallback value for error cases.
/// Logs error if time appears to have gone backwards before returning fallback.
pub fn now_with_fallback(fallback: Timestamp) -> Timestamp {
    match now() {
        Ok(ts) => ts,
        Err(e) => {
            // Log the error when time appears to have gone backwards
            log::warn!("Clock error when getting timestamp: {}", e);
            fallback
        }
    }
}
