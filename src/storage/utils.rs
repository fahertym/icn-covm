use std::time::{SystemTime, UNIX_EPOCH};

/// Type alias for standard timestamps (seconds since UNIX epoch)
pub type Timestamp = u64;

/// Returns the current time as a `Timestamp`.
pub fn now() -> Timestamp {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards") // Or handle error more gracefully
        .as_secs()
} 