use crate::storage::utils::Timestamp;
use serde::{Deserialize, Serialize};

// Represents an event that occurred in the storage system for auditing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageEvent {
    pub event_type: String, // e.g., "write", "read", "delete", "permission_change"
    pub user_id: String,
    pub namespace: String,
    pub key: String,
    pub timestamp: Timestamp,
    pub details: String, // e.g., size of data written, permission granted
}
