use crate::storage::errors::StorageError;
use crate::storage::utils::{now_with_default, Timestamp};
use serde::{Deserialize, Serialize};

// Resource accounting
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceAccount {
    pub owner_id: String,
    pub storage_quota_bytes: u64,
    pub storage_used_bytes: u64,
    pub last_updated: Timestamp,
}

impl ResourceAccount {
    pub fn new(owner_id: &str, storage_quota_bytes: u64) -> Self {
        Self {
            owner_id: owner_id.to_string(),
            storage_quota_bytes,
            storage_used_bytes: 0,
            last_updated: now_with_default(),
        }
    }

    // Check if the account has enough quota for additional bytes
    pub fn can_store(&self, additional_bytes: u64) -> bool {
        self.storage_used_bytes.saturating_add(additional_bytes) <= self.storage_quota_bytes
    }

    // Add storage usage, returning error if quota exceeded
    pub fn add_usage(&mut self, bytes: u64) -> Result<(), StorageError> {
        if !self.can_store(bytes) {
            return Err(StorageError::QuotaExceeded {
                limit_type: format!("Storage for account '{}'", self.owner_id),
                current: self.storage_used_bytes,
                maximum: self.storage_quota_bytes,
            });
        }
        self.storage_used_bytes = self.storage_used_bytes.saturating_add(bytes);
        self.last_updated = now_with_default();
        Ok(())
    }

    // Reduce storage usage (e.g., when data is deleted)
    pub fn reduce_usage(&mut self, bytes: u64) {
        self.storage_used_bytes = self.storage_used_bytes.saturating_sub(bytes);
        self.last_updated = now_with_default();
    }
}
