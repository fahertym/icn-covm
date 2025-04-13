use crate::storage::errors::StorageError;
use crate::storage::utils::{now, Timestamp};
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
            last_updated: now(),
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
                account_id: self.owner_id.clone(),
                requested: bytes,
                available: self
                    .storage_quota_bytes
                    .saturating_sub(self.storage_used_bytes),
            });
        }
        self.storage_used_bytes = self.storage_used_bytes.saturating_add(bytes);
        self.last_updated = now();
        Ok(())
    }

    // Reduce storage usage (e.g., when data is deleted)
    pub fn reduce_usage(&mut self, bytes: u64) {
        self.storage_used_bytes = self.storage_used_bytes.saturating_sub(bytes);
        self.last_updated = now();
    }
}
