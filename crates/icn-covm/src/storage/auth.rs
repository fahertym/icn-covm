use crate::identity::Identity;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Display};

/// Represents a role assignment for an identity in a specific namespace
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RoleAssignment {
    /// The namespace this role applies to (e.g., "coop/my-coop")
    pub namespace: String,
    /// The role name (e.g., "admin", "member")
    pub role: String,
}

/// Represents a membership relationship between an identity and a namespace (typically a cooperative)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Membership {
    /// The identity DID that is a member
    pub identity_did: String,
    /// The namespace (cooperative ID) they are a member of
    pub namespace: String,
    /// Optional metadata about the membership
    pub metadata: HashMap<String, String>,
}

impl Display for Membership {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Member {} of {}", self.identity_did, self.namespace)
    }
}

/// Represents a delegation from one identity to another
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Delegation {
    /// The delegator's identity DID
    pub delegator_did: String,
    /// The delegate's identity DID
    pub delegate_did: String,
    /// The type of delegation (e.g., "voting", "representation")
    pub delegation_type: String,
    /// Optional metadata about the delegation
    pub metadata: HashMap<String, String>,
}

/// Provides authentication and authorization context for the VM
///
/// The auth context contains information about the current user, their roles,
/// and the identities they can act on behalf of.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthContext {
    /// DID of the current identity (user)
    pub current_identity_did: String,

    /// Map of identity DIDs to their full identity information
    pub identity_registry: HashMap<String, Identity>,

    /// Map of namespace to set of identity DIDs with that role
    pub roles: HashMap<String, HashMap<String, HashSet<String>>>,

    /// List of memberships
    pub memberships: Vec<Membership>,

    /// List of delegations
    pub delegations: Vec<Delegation>,
}

impl AuthContext {
    /// Create a new auth context with the specified identity as the current user
    pub fn new(current_identity_did: &str) -> Self {
        Self {
            current_identity_did: current_identity_did.to_string(),
            identity_registry: HashMap::new(),
            roles: HashMap::new(),
            memberships: Vec::new(),
            delegations: Vec::new(),
        }
    }

    /// Get the cooperative ID for an identity if available
    pub fn get_coop_id(&self, identity_did: &str) -> Option<String> {
        // Look for a membership in a cooperative namespace
        self.memberships
            .iter()
            .find(|m| m.identity_did == identity_did && m.namespace.starts_with("coops/"))
            .map(|m| m.namespace.trim_start_matches("coops/").to_string())
    }

    /// Register an identity in the identity registry
    pub fn register_identity(&mut self, identity: Identity) {
        let did = identity.did.clone();
        self.identity_registry.insert(did, identity);
    }

    /// Add a role to the current identity
    pub fn add_role(&mut self, namespace: &str, role: &str) {
        let current_did = self.current_identity_did.clone();
        self.add_role_to_identity(&current_did, namespace, role);
    }

    /// Add a role to a specific identity
    pub fn add_role_to_identity(&mut self, identity_did: &str, namespace: &str, role: &str) {
        let namespace_roles = self
            .roles
            .entry(namespace.to_string())
            .or_insert_with(HashMap::new);
        let role_identities = namespace_roles
            .entry(role.to_string())
            .or_insert_with(HashSet::new);
        role_identities.insert(identity_did.to_string());
    }

    /// Add a membership relationship between an identity and a namespace (cooperative)
    pub fn add_membership(&mut self, identity_did: &str, namespace: &str) {
        let membership = Membership {
            identity_did: identity_did.to_string(),
            namespace: namespace.to_string(),
            metadata: HashMap::new(),
        };
        self.memberships.push(membership);
    }

    /// Add a delegation from one identity to another
    pub fn add_delegation(
        &mut self,
        delegator_did: &str,
        delegate_did: &str,
        delegation_type: &str,
    ) {
        let delegation = Delegation {
            delegator_did: delegator_did.to_string(),
            delegate_did: delegate_did.to_string(),
            delegation_type: delegation_type.to_string(),
            metadata: HashMap::new(),
        };
        self.delegations.push(delegation);
    }

    /// Check if an identity has a specific role in a namespace
    pub fn has_role(&self, namespace: &str, role: &str) -> bool {
        self.has_role_for_identity(&self.current_identity_did, namespace, role)
    }

    /// Check if a specific identity has a specific role in a namespace
    pub fn has_role_for_identity(&self, identity_did: &str, namespace: &str, role: &str) -> bool {
        if let Some(namespace_roles) = self.roles.get(namespace) {
            if let Some(role_identities) = namespace_roles.get(role) {
                return role_identities.contains(identity_did);
            }
        }
        false
    }

    /// Check if an identity is a member of a namespace
    pub fn is_member(&self, identity_did: &str, namespace: &str) -> bool {
        self.memberships
            .iter()
            .any(|m| m.identity_did == identity_did && m.namespace == namespace)
    }

    /// Check if a delegation exists from delegator to delegate
    pub fn has_delegation(&self, delegator_did: &str, delegate_did: &str) -> bool {
        self.delegations
            .iter()
            .any(|d| d.delegator_did == delegator_did && d.delegate_did == delegate_did)
    }

    /// Get an identity by its DID
    pub fn get_identity(&self, identity_did: &str) -> Option<&Identity> {
        self.identity_registry.get(identity_did)
    }

    /// Verify a signature for an identity
    /// This is a simplified implementation that always returns true for registered identities
    /// In a real implementation, this would verify the signature using the identity's public key
    pub fn verify_signature(&self, identity_did: &str, _message: &[u8], _signature: &str) -> bool {
        if let Some(_identity) = self.get_identity(identity_did) {
            // In a real implementation, we would verify the signature using the identity's public key
            // For now, we'll assume all signatures from registered identities are valid for testing
            true
        } else {
            false
        }
    }

    /// Get the current user's DID
    pub fn identity_did(&self) -> &str {
        &self.current_identity_did
    }

    /// For backwards compatibility, alias to current_identity_did
    /// Returns the user ID as a string reference
    pub fn user_id(&self) -> &str {
        &self.current_identity_did
    }

    /// For backwards compatibility, returns the user ID as a cloned String
    /// Use this when you need to own the string for storage or other operations
    pub fn user_id_string(&self) -> String {
        self.current_identity_did.clone()
    }

    /// For backwards compatibility with code that expects to call .clone() on the result
    /// of user_id() to get a String
    pub fn user_id_cloneable(&self) -> String {
        self.current_identity_did.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::Profile;
    use std::collections::HashMap;

    fn create_test_identity(name: &str) -> Identity {
        Identity::new(name.to_string(), None, "member".to_string(), None)
            .expect("Failed to create test identity")
    }

    #[test]
    fn test_roles() {
        let alice = create_test_identity("alice");
        let bob = create_test_identity("bob");

        let mut auth = AuthContext::new(&alice.did);
        auth.register_identity(alice.clone());
        auth.register_identity(bob.clone());

        auth.add_role("coop1", "admin");
        auth.add_role_to_identity(&bob.did, "coop1", "member");

        assert!(auth.has_role_for_identity(&alice.did, "coop1", "admin"));
        assert!(!auth.has_role_for_identity(&alice.did, "coop1", "member"));
        assert!(auth.has_role_for_identity(&bob.did, "coop1", "member"));
        assert!(!auth.has_role_for_identity(&bob.did, "coop1", "admin"));
    }

    #[test]
    fn test_memberships() {
        let alice = create_test_identity("alice");
        let bob = create_test_identity("bob");

        let mut auth = AuthContext::new(&alice.did);
        auth.register_identity(alice.clone());
        auth.register_identity(bob.clone());

        auth.add_membership(&alice.did, "coops/coop1");
        auth.add_membership(&bob.did, "coops/coop2");

        assert!(auth.is_member(&alice.did, "coops/coop1"));
        assert!(!auth.is_member(&alice.did, "coops/coop2"));
        assert!(auth.is_member(&bob.did, "coops/coop2"));
        assert!(!auth.is_member(&bob.did, "coops/coop1"));
    }

    #[test]
    fn test_delegations() {
        let alice = create_test_identity("alice");
        let bob = create_test_identity("bob");
        let carol = create_test_identity("carol");

        let mut auth = AuthContext::new(&alice.did);
        auth.register_identity(alice.clone());
        auth.register_identity(bob.clone());
        auth.register_identity(carol.clone());

        auth.add_delegation(&bob.did, &alice.did, "voting");
        auth.add_delegation(&carol.did, &bob.did, "voting");

        assert!(auth.has_delegation(&bob.did, &alice.did));
        assert!(auth.has_delegation(&carol.did, &bob.did));
        assert!(!auth.has_delegation(&alice.did, &bob.did));
        assert!(!auth.has_delegation(&bob.did, &carol.did));
    }

    #[test]
    fn test_coop_id() {
        let alice = create_test_identity("alice");

        let mut auth = AuthContext::new(&alice.did);
        auth.register_identity(alice.clone());

        auth.add_membership(&alice.did, "coops/coop1");

        assert_eq!(auth.get_coop_id(&alice.did), Some("coop1".to_string()));
    }
}
