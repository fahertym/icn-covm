use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::storage::versioning::VersionInfo;

/// Link representing a delegation from one identity to another
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DelegationLink {
    /// Unique identifier for this delegation
    pub id: String,

    /// Identity ID of the delegator
    pub delegator_id: String,

    /// Identity ID of the delegate
    pub delegate_id: String,

    /// Type of delegation (e.g., "voting", "admin", "full")
    pub delegation_type: String,

    /// Permissions granted through this delegation
    pub permissions: Vec<String>,

    /// When the delegation was created
    pub created_at: u64,

    /// When the delegation expires (if temporary)
    pub expires_at: Option<u64>,

    /// Cryptographic signature from the delegator
    pub signature: Option<Vec<u8>>,

    /// Additional attributes for this delegation
    pub attributes: HashMap<String, String>,

    /// Version information for this delegation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_info: Option<VersionInfo>,
}

impl DelegationLink {
    /// Create a new delegation link
    pub fn new(
        id: &str,
        delegator_id: &str,
        delegate_id: &str,
        delegation_type: &str,
        created_at: u64,
    ) -> Self {
        Self {
            id: id.to_string(),
            delegator_id: delegator_id.to_string(),
            delegate_id: delegate_id.to_string(),
            delegation_type: delegation_type.to_string(),
            permissions: Vec::new(),
            created_at,
            expires_at: None,
            signature: None,
            attributes: HashMap::new(),
            version_info: None,
        }
    }

    /// Add a permission to this delegation
    pub fn add_permission(&mut self, permission: &str) -> &mut Self {
        if !self.permissions.contains(&permission.to_string()) {
            self.permissions.push(permission.to_string());
        }
        self
    }

    /// Set expiration timestamp
    pub fn with_expiration(&mut self, expires_at: u64) -> &mut Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// Add attribute to this delegation
    pub fn add_attribute(&mut self, key: &str, value: &str) -> &mut Self {
        self.attributes.insert(key.to_string(), value.to_string());
        self
    }

    /// Sign the delegation (typically by the delegator)
    pub fn sign(&mut self, signature: Vec<u8>) -> &mut Self {
        self.signature = Some(signature);
        self
    }

    /// Check if the delegation is expired
    pub fn is_expired(&self, current_time: u64) -> bool {
        match self.expires_at {
            Some(expires) => current_time > expires,
            None => false,
        }
    }

    /// Check if the delegation has a specific permission
    pub fn has_permission(&self, permission: &str) -> bool {
        self.permissions.contains(&permission.to_string())
    }

    /// Check if the delegation is signed
    pub fn is_signed(&self) -> bool {
        self.signature.is_some()
    }

    /// Get the namespace for this delegation
    pub fn get_namespace(&self) -> String {
        format!("delegations/{}/{}", self.delegation_type, self.id)
    }
}
