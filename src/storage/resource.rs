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

/// Represents an economic resource or token in the VM
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EconomicResource {
    /// Unique identifier for the resource
    pub id: String,

    /// Human-readable name of the resource
    pub name: String,

    /// Optional description of the resource
    pub description: Option<String>,

    /// Type of the resource (e.g., "currency", "labor_hour", "material")
    pub resource_type: String,

    /// Namespace/cooperative that issued this resource
    pub issuer_namespace: String,

    /// Timestamp when the resource was created
    pub created_at: Timestamp,

    /// Optional metadata for the resource as key-value pairs
    #[serde(default)]
    pub metadata: std::collections::HashMap<String, String>,

    /// Whether the resource is transferable between accounts
    #[serde(default = "default_transferable")]
    pub transferable: bool,

    /// Whether the resource can be divided into fractional amounts
    #[serde(default = "default_divisible")]
    pub divisible: bool,
}

/// Represents an account balance for a specific economic resource
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResourceBalance {
    /// ID of the resource this balance tracks
    pub resource_id: String,

    /// ID of the account/identity that owns this balance
    pub account_id: String,

    /// Current amount of the resource
    pub amount: f64,

    /// Timestamp of the last balance update
    pub last_updated: Timestamp,
}

/// Represents a transfer transaction of an economic resource
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ResourceTransfer {
    /// Unique identifier for the transfer
    pub id: String,

    /// ID of the resource being transferred
    pub resource_id: String,

    /// Source account/identity ID (None for minting)
    pub from_account: Option<String>,

    /// Destination account/identity ID (None for burning)
    pub to_account: Option<String>,

    /// Amount of the resource being transferred
    pub amount: f64,

    /// Timestamp when the transfer occurred
    pub timestamp: Timestamp,

    /// Optional memo or reason for the transfer
    pub memo: Option<String>,

    /// ID of the identity that authorized this transfer
    pub authorized_by: String,
}

// Default function for transferable field
fn default_transferable() -> bool {
    true
}

// Default function for divisible field
fn default_divisible() -> bool {
    true
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

impl EconomicResource {
    /// Create a new economic resource
    pub fn new(id: &str, name: &str, resource_type: &str, issuer_namespace: &str) -> Self {
        Self {
            id: id.to_string(),
            name: name.to_string(),
            description: None,
            resource_type: resource_type.to_string(),
            issuer_namespace: issuer_namespace.to_string(),
            created_at: now(),
            metadata: std::collections::HashMap::new(),
            transferable: default_transferable(),
            divisible: default_divisible(),
        }
    }

    /// Add metadata to the resource
    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }

    /// Set resource description
    pub fn with_description(mut self, description: &str) -> Self {
        self.description = Some(description.to_string());
        self
    }

    /// Set whether the resource is transferable
    pub fn transferable(mut self, transferable: bool) -> Self {
        self.transferable = transferable;
        self
    }

    /// Set whether the resource is divisible
    pub fn divisible(mut self, divisible: bool) -> Self {
        self.divisible = divisible;
        self
    }
}

impl ResourceBalance {
    /// Create a new resource balance
    pub fn new(resource_id: &str, account_id: &str, initial_amount: f64) -> Self {
        Self {
            resource_id: resource_id.to_string(),
            account_id: account_id.to_string(),
            amount: initial_amount,
            last_updated: now(),
        }
    }

    /// Add to the balance
    pub fn add(&mut self, amount: f64) -> Result<(), StorageError> {
        if amount < 0.0 {
            return Err(StorageError::InvalidOperation(format!(
                "Cannot add negative amount {} to balance",
                amount
            )));
        }
        self.amount += amount;
        self.last_updated = now();
        Ok(())
    }

    /// Subtract from the balance
    pub fn subtract(&mut self, amount: f64) -> Result<(), StorageError> {
        if amount < 0.0 {
            return Err(StorageError::InvalidOperation(format!(
                "Cannot subtract negative amount {} from balance",
                amount
            )));
        }
        if self.amount < amount {
            return Err(StorageError::InsufficientBalance {
                account_id: self.account_id.clone(),
                resource_id: self.resource_id.clone(),
                requested: amount,
                available: self.amount,
            });
        }
        self.amount -= amount;
        self.last_updated = now();
        Ok(())
    }
}

impl ResourceTransfer {
    /// Create a new resource transfer
    pub fn new(
        id: &str,
        resource_id: &str,
        from_account: Option<&str>,
        to_account: Option<&str>,
        amount: f64,
        authorized_by: &str,
    ) -> Result<Self, StorageError> {
        // Validate that either from or to is specified (or both for transfers)
        if from_account.is_none() && to_account.is_none() {
            return Err(StorageError::InvalidOperation(
                "Either source or destination account must be specified for a transfer".to_string(),
            ));
        }

        // Validate amount is positive
        if amount <= 0.0 {
            return Err(StorageError::InvalidOperation(format!(
                "Transfer amount must be positive, got {}",
                amount
            )));
        }

        Ok(Self {
            id: id.to_string(),
            resource_id: resource_id.to_string(),
            from_account: from_account.map(|s| s.to_string()),
            to_account: to_account.map(|s| s.to_string()),
            amount,
            timestamp: now(),
            memo: None,
            authorized_by: authorized_by.to_string(),
        })
    }

    /// Add a memo to the transfer
    pub fn with_memo(mut self, memo: &str) -> Self {
        self.memo = Some(memo.to_string());
        self
    }

    /// Check if this is a mint operation (no source account)
    pub fn is_mint(&self) -> bool {
        self.from_account.is_none() && self.to_account.is_some()
    }

    /// Check if this is a burn operation (no destination account)
    pub fn is_burn(&self) -> bool {
        self.from_account.is_some() && self.to_account.is_none()
    }

    /// Check if this is a transfer operation (both source and destination)
    pub fn is_transfer(&self) -> bool {
        self.from_account.is_some() && self.to_account.is_some()
    }
}
