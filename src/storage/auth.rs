use std::collections::{HashMap, HashSet};
use serde::{Serialize, Deserialize};
use crate::storage::utils::{Timestamp, now};

// Auth Context for RBAC
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthContext {
    /// The user ID performing the action
    pub user_id: String,
    
    /// Namespace -> Roles mapping (what roles the user has in each namespace)
    roles: HashMap<String, HashSet<String>>,
    
    /// Delegator ID -> Delegate ID mapping (who the user has delegated to)
    delegations_out: HashMap<String, String>,
    
    /// Delegate ID -> Delegator ID mapping (who has delegated to this user)
    delegations_in: HashMap<String, String>,
    
    /// Timestamp when this auth context was created
    pub timestamp: Timestamp,
}

impl AuthContext {
    /// Create a new AuthContext for a user with no roles
    pub fn new(user_id: &str) -> Self {
        Self {
            user_id: user_id.to_string(),
            roles: HashMap::new(),
            delegations_out: HashMap::new(),
            delegations_in: HashMap::new(),
            timestamp: now(),
        }
    }

    /// Create a new AuthContext with initial roles
    /// All roles are added to the "default" namespace
    pub fn with_roles(user_id: &str, roles: Vec<String>) -> Self {
        let mut auth = Self::new(user_id);
        let default_namespace = "default";
        
        // Add each role to the default namespace
        for role in roles {
            auth.add_role(default_namespace, &role);
        }
        
        auth
    }
    
    /// Create a new AuthContext with namespace-specific roles
    pub fn with_namespace_roles(user_id: &str, roles: Vec<(String, String)>) -> Self {
        let mut auth = Self::new(user_id);
        
        // Add each role to its specific namespace
        for (namespace, role) in roles {
            auth.add_role(&namespace, &role);
        }
        
        auth
    }

    /// Add a role to a namespace for this user
    pub fn add_role(&mut self, namespace: &str, role: &str) {
        self.roles
            .entry(namespace.to_string())
            .or_insert_with(HashSet::new)
            .insert(role.to_string());
    }
    
    /// Remove a role from a namespace for this user
    pub fn remove_role(&mut self, namespace: &str, role: &str) -> bool {
        if let Some(roles) = self.roles.get_mut(namespace) {
            roles.remove(role)
        } else {
            false
        }
    }

    /// Check if the user directly has a role in a namespace
    pub fn has_role(&self, namespace: &str, role: &str) -> bool {
        // Check global admin first (can access everything)
        if namespace != "global" && self.has_role("global", "admin") {
            return true;
        }
        
        self.roles
            .get(namespace)
            .map(|roles| roles.contains(role))
            .unwrap_or(false)
    }
    
    /// Check if the user has any role in a namespace
    pub fn has_any_role_in_namespace(&self, namespace: &str) -> bool {
        // Global admin can access everything
        if namespace != "global" && self.has_role("global", "admin") {
            return true;
        }
        
        self.roles
            .get(namespace)
            .map(|roles| !roles.is_empty())
            .unwrap_or(false)
    }
    
    /// Get all roles the user has in a specific namespace
    pub fn get_roles_in_namespace(&self, namespace: &str) -> Vec<String> {
        self.roles
            .get(namespace)
            .map(|roles| roles.iter().cloned().collect())
            .unwrap_or_default()
    }
    
    /// Get all namespaces the user has any role in
    pub fn get_namespaces(&self) -> Vec<String> {
        self.roles.keys().cloned().collect()
    }

    /// Delegate rights to another user
    pub fn delegate_to(&mut self, delegate_id: &str) -> Result<(), &'static str> {
        // Cannot delegate to self
        if delegate_id == self.user_id {
            return Err("Cannot delegate to self");
        }
        
        // Check for delegation cycles (we'd need to check the whole chain in practice)
        if self.is_delegate_of(delegate_id) {
            return Err("Delegation would create a cycle");
        }
        
        self.delegations_out.insert(self.user_id.clone(), delegate_id.to_string());
        Ok(())
    }
    
    /// Remove a delegation
    pub fn remove_delegation(&mut self) -> Option<String> {
        self.delegations_out.remove(&self.user_id)
    }
    
    /// Register that someone has delegated to this user
    pub fn register_delegation_from(&mut self, delegator_id: &str) {
        self.delegations_in.insert(delegator_id.to_string(), self.user_id.clone());
    }
    
    /// Remove a delegation that someone made to this user
    pub fn remove_delegation_from(&mut self, delegator_id: &str) -> bool {
        self.delegations_in.remove(delegator_id).is_some()
    }

    /// Check if the user `delegate_id` is a delegate of this user
    pub fn is_delegate(&self, delegate_id: &str) -> bool {
        self.delegations_out.get(&self.user_id)
            .map(|id| id == delegate_id)
            .unwrap_or(false)
    }

    /// Check if this user is a delegate of `delegator_id`
    pub fn is_delegate_of(&self, delegator_id: &str) -> bool {
        self.delegations_in.get(delegator_id)
            .map(|id| id == &self.user_id)
            .unwrap_or(false)
    }

    /// Get the ID of the user this context has delegated to (if any)
    pub fn get_delegation(&self) -> Option<&String> {
        self.delegations_out.get(&self.user_id)
    }
    
    /// Get all users who have delegated to this user
    pub fn get_delegators(&self) -> Vec<String> {
        self.delegations_in.keys().cloned().collect()
    }
    
    /// Check if this auth context has expired (based on timestamp)
    pub fn is_expired(&self, max_age: Timestamp) -> bool {
        now() - self.timestamp > max_age
    }
    
    /// Refresh the timestamp on this auth context
    pub fn refresh(&mut self) {
        self.timestamp = now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_basic_roles() {
        let mut auth = AuthContext::new("user1");
        
        // Add roles
        auth.add_role("governance", "voter");
        auth.add_role("governance", "proposer");
        auth.add_role("treasury", "spender");
        
        // Check roles
        assert!(auth.has_role("governance", "voter"));
        assert!(auth.has_role("governance", "proposer"));
        assert!(auth.has_role("treasury", "spender"));
        assert!(!auth.has_role("treasury", "admin"));
        
        // Check namespace awareness
        assert!(auth.has_any_role_in_namespace("governance"));
        assert!(!auth.has_any_role_in_namespace("nonexistent"));
        
        // Get roles
        let governance_roles = auth.get_roles_in_namespace("governance");
        assert_eq!(governance_roles.len(), 2);
        assert!(governance_roles.contains(&"voter".to_string()));
        assert!(governance_roles.contains(&"proposer".to_string()));
    }
    
    #[test]
    fn test_global_admin() {
        let mut auth = AuthContext::new("admin1");
        
        // Add global admin role
        auth.add_role("global", "admin");
        
        // Global admin should have access to everything
        assert!(auth.has_role("governance", "voter"));
        assert!(auth.has_role("treasury", "spender"));
        assert!(auth.has_any_role_in_namespace("any_namespace"));
        
        // Except explicit global roles they don't have
        assert!(!auth.has_role("global", "super_admin"));
    }
    
    #[test]
    fn test_delegations() {
        let mut user1 = AuthContext::new("user1");
        let mut user2 = AuthContext::new("user2");
        
        // User1 delegates to User2
        user1.delegate_to("user2").unwrap();
        user2.register_delegation_from("user1");
        
        // Check delegation relationships
        assert!(user1.is_delegate("user2"));
        assert!(user2.is_delegate_of("user1"));
        
        // Get delegation info
        assert_eq!(user1.get_delegation(), Some(&"user2".to_string()));
        assert_eq!(user2.get_delegators(), vec!["user1".to_string()]);
        
        // Remove delegation
        user1.remove_delegation();
        user2.remove_delegation_from("user1");
        
        // Verify removal
        assert_eq!(user1.get_delegation(), None);
        assert!(user2.get_delegators().is_empty());
    }
}
