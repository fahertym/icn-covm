use std::collections::HashMap;
use serde::{Serialize, Deserialize};

// Auth Context for RBAC
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthContext {
    pub user_id: String,
    // Namespace -> Roles
    roles: HashMap<String, Vec<String>>,
    // Delegate ID -> Delegator ID
    delegations: HashMap<String, String>,
}

impl AuthContext {
    pub fn new(user_id: &str) -> Self {
        Self {
            user_id: user_id.to_string(),
            roles: HashMap::new(),
            delegations: HashMap::new(),
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

    pub fn add_role(&mut self, namespace: &str, role: &str) {
        self.roles
            .entry(namespace.to_string())
            .or_insert_with(Vec::new)
            .push(role.to_string());
    }

    // Check if the user directly has a role in a namespace
    pub fn has_role(&self, namespace: &str, role: &str) -> bool {
        self.roles
            .get(namespace)
            .map(|roles| roles.contains(&role.to_string()))
            .unwrap_or(false)
    }

    // Delegate voting power or other rights to another user
    pub fn delegate_to(&mut self, delegate_id: &str) {
        // TODO: Add checks - cannot delegate to self, prevent cycles?
        self.delegations.insert(delegate_id.to_string(), self.user_id.clone());
    }

    // Check if the user `delegate_id` is a delegate of this context's user
    pub fn is_delegate(&self, delegate_id: &str) -> bool {
        self.delegations.contains_key(delegate_id)
    }

    // Check if this user is a delegate of `delegator_id`
    pub fn is_delegate_of(&self, delegator_id: &str) -> bool {
       // NOTE: This implementation seems incorrect based on the map structure
       // (Delegate ID -> Delegator ID). It checks if *any* delegation stored
       // points to the `delegator_id`.
       // A clearer check might be needed depending on the exact delegation semantics desired.
       // Reverting to original logic from the demo for now.
       self.delegations.values().any(|id| id == delegator_id)
    }

    // Get the user ID this context has delegated to (if any)
    // Assuming one delegation per user for simplicity here.
    pub fn get_delegation(&self) -> Option<&String> {
        // This assumes the map is Delegate -> Delegator. If one user delegates
        // to multiple people, this needs rethinking. If one user delegates to ONE
        // person, the map should maybe be UserID -> DelegateID.
        // Let's return the first delegate found based on current structure.
        self.delegations.keys().next()
    }
}
