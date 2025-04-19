use serde::{Serialize, Deserialize};
use std::collections::HashMap;

use crate::identity::Identity;
use crate::storage::versioning::VersionInfo;

/// Member profile for cooperative members
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MemberProfile {
    /// The core identity this profile is associated with
    pub identity: Identity,
    
    /// Member-specific roles within their cooperative
    pub roles: Vec<String>,
    
    /// Reputation score (if used by the cooperative)
    pub reputation: Option<f64>,
    
    /// Joined timestamp
    pub joined_at: u64,
    
    /// Additional profile attributes
    pub attributes: HashMap<String, String>,
    
    /// Version information for this profile
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_info: Option<VersionInfo>,
}

impl MemberProfile {
    /// Create a new member profile
    pub fn new(identity: Identity, joined_at: u64) -> Self {
        Self {
            identity,
            roles: Vec::new(),
            reputation: None,
            joined_at,
            attributes: HashMap::new(),
            version_info: None,
        }
    }
    
    /// Add a role to this member
    pub fn add_role(&mut self, role: &str) -> &mut Self {
        if !self.roles.contains(&role.to_string()) {
            self.roles.push(role.to_string());
        }
        self
    }
    
    /// Check if member has a specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.contains(&role.to_string())
    }
    
    /// Add profile attribute
    pub fn add_attribute(&mut self, key: &str, value: &str) -> &mut Self {
        self.attributes.insert(key.to_string(), value.to_string());
        self
    }
    
    /// Set reputation score
    pub fn set_reputation(&mut self, score: f64) -> &mut Self {
        self.reputation = Some(score);
        self
    }
    
    /// Get the cooperative ID this member belongs to
    pub fn get_cooperative_id(&self) -> Option<&String> {
        self.identity.get_metadata("coop_id")
    }
    
    /// Get member-specific namespace
    pub fn get_namespace(&self) -> String {
        self.identity.get_namespace()
    }
} 