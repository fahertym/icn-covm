use std::time::{SystemTime, UNIX_EPOCH};
use crate::storage::errors::{StorageError, StorageResult};

/// Type alias for standard timestamps (seconds since UNIX epoch)
pub type Timestamp = u64;

/// Returns the current time as a `Timestamp`.
/// If time appears to have gone backwards (due to clock adjustment), returns an error.
pub fn now() -> StorageResult<Timestamp> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| StorageError::TimeError { 
            details: format!("Failed to get timestamp: {}", e) 
        })
        .map(|d| d.as_secs())
}

/// Returns the current time as a `Timestamp`, with a fallback value for error cases.
/// Logs error if time appears to have gone backwards before returning fallback.
///
/// This is a safe alternative to `now()` for cases where errors can be handled gracefully.
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

/// Returns the current time as a `Timestamp`, panicking if there is an error.
///
/// This function should only be used in test code or where a panic is acceptable.
/// For production code, prefer `now()` with proper error handling or `now_with_fallback()`.
#[cfg(test)]
pub fn now_or_panic() -> Timestamp {
    now().expect("Failed to get current timestamp")
}

/// Returns a default timestamp for testing purposes.
///
/// This ensures tests don't depend on the actual system time.
#[cfg(test)]
pub fn test_timestamp() -> Timestamp {
    // January 1, 2022 00:00:00 UTC
    1640995200
}

/// Returns the current time as a `Timestamp` or a default value if it fails.
///
/// This uses a hardcoded default timestamp if the system time cannot be retrieved.
/// The default timestamp is January 1, 2022 (1640995200).
/// This is a safe alternative for transitioning code that incorrectly used `now()` directly.
pub fn now_with_default() -> Timestamp {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => duration.as_secs(),
        Err(e) => {
            // Log the error
            log::warn!("Clock error when getting timestamp, using default: {}", e);
            // Use January 1, 2022 as the default timestamp
            1640995200
        }
    }
}
