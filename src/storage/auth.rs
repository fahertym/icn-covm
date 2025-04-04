use crate::identity::{Credential, DelegationLink, Identity, MemberProfile};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Auth Context for RBAC and identity-aware execution
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AuthContext {
    pub user_id: String,
    // Namespace -> Roles
    roles: HashMap<String, Vec<String>>,
    // Delegate ID -> Delegator ID
    delegations: HashMap<String, String>,
    // Identity information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_identity: Option<Identity>,
    // Known identities cache
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity_registry: Option<HashMap<String, Identity>>,
    // Known delegations cache
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delegation_registry: Option<HashMap<String, DelegationLink>>,
    // Member profiles cache
    #[serde(skip_serializing_if = "Option::is_none")]
    pub member_registry: Option<HashMap<String, MemberProfile>>,
    // Credentials cache
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential_registry: Option<HashMap<String, Credential>>,
    // Execution context information for federation
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executing_cooperative_id: Option<String>,
}

impl AuthContext {
    pub fn new(user_id: &str) -> Self {
        Self {
            user_id: user_id.to_string(),
            roles: HashMap::new(),
            delegations: HashMap::new(),
            current_identity: None,
            identity_registry: None,
            delegation_registry: None,
            member_registry: None,
            credential_registry: None,
            executing_cooperative_id: None,
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

    /// Create a new AuthContext with identity information
    pub fn with_identity(user_id: &str, identity: Identity) -> Self {
        let mut auth = Self::new(user_id);
        auth.current_identity = Some(identity);

        // Set cooperative ID if it exists in the identity's metadata
        if let Some(coop_id) = auth
            .current_identity
            .as_ref()
            .and_then(|id| id.get_metadata("coop_id"))
        {
            auth.executing_cooperative_id = Some(coop_id.clone());
        }

        auth
    }

    /// Set the current identity
    pub fn set_identity(&mut self, identity: Identity) -> &mut Self {
        self.current_identity = Some(identity);

        // Update the user ID to match the identity ID for consistency
        self.user_id = self.current_identity.as_ref().unwrap().id.clone();

        // Update cooperative ID if available
        if let Some(coop_id) = self
            .current_identity
            .as_ref()
            .and_then(|id| id.get_metadata("coop_id"))
        {
            self.executing_cooperative_id = Some(coop_id.clone());
        }

        self
    }

    /// Set the executing cooperative ID
    pub fn set_cooperative_id(&mut self, coop_id: &str) -> &mut Self {
        self.executing_cooperative_id = Some(coop_id.to_string());
        self
    }

    /// Register a known identity
    pub fn register_identity(&mut self, identity: Identity) -> &mut Self {
        if self.identity_registry.is_none() {
            self.identity_registry = Some(HashMap::new());
        }

        let registry = self.identity_registry.as_mut().unwrap();
        registry.insert(identity.id.clone(), identity);
        self
    }

    /// Register a known delegation
    pub fn register_delegation(&mut self, delegation: DelegationLink) -> &mut Self {
        if self.delegation_registry.is_none() {
            self.delegation_registry = Some(HashMap::new());
        }

        let registry = self.delegation_registry.as_mut().unwrap();
        registry.insert(delegation.id.clone(), delegation);
        self
    }

    /// Register a known member profile
    pub fn register_member(&mut self, member: MemberProfile) -> &mut Self {
        if self.member_registry.is_none() {
            self.member_registry = Some(HashMap::new());
        }

        let registry = self.member_registry.as_mut().unwrap();
        registry.insert(member.identity.id.clone(), member);
        self
    }

    /// Register a known credential
    pub fn register_credential(&mut self, credential: Credential) -> &mut Self {
        if self.credential_registry.is_none() {
            self.credential_registry = Some(HashMap::new());
        }

        let registry = self.credential_registry.as_mut().unwrap();
        registry.insert(credential.id.clone(), credential);
        self
    }

    /// Get an identity by ID
    pub fn get_identity(&self, id: &str) -> Option<&Identity> {
        self.identity_registry.as_ref().and_then(|reg| reg.get(id))
    }

    /// Get a delegation by ID
    pub fn get_delegation(&self, id: &str) -> Option<&DelegationLink> {
        self.delegation_registry
            .as_ref()
            .and_then(|reg| reg.get(id))
    }

    /// Get a member profile by identity ID
    pub fn get_member(&self, identity_id: &str) -> Option<&MemberProfile> {
        self.member_registry
            .as_ref()
            .and_then(|reg| reg.get(identity_id))
    }

    /// Get a credential by ID
    pub fn get_credential(&self, id: &str) -> Option<&Credential> {
        self.credential_registry
            .as_ref()
            .and_then(|reg| reg.get(id))
    }

    /// Check if an identity is a member of a cooperative or namespace
    pub fn is_member_of(&self, identity_id: &str, namespace: &str) -> bool {
        // First check if we have a member profile
        if let Some(member) = self.get_member(identity_id) {
            // If this is a cooperative membership check
            if namespace.starts_with("coops/") {
                // Extract cooperative ID from namespace (e.g., "coops/coop123")
                let parts: Vec<&str> = namespace.split('/').collect();
                if parts.len() >= 2 {
                    let coop_id = parts[1];
                    return member
                        .get_cooperative_id()
                        .map_or(false, |id| id == coop_id);
                }
            }

            // For other namespaces, check if member has any roles there
            return self.roles.get(namespace).is_some();
        }

        // If we don't have a member profile, check credentials
        if let Some(registry) = &self.credential_registry {
            for credential in registry.values() {
                if credential.holder_id == identity_id
                    && credential.credential_type == "membership"
                    && !credential.is_expired(crate::storage::utils::now())
                {
                    // Check if credential has a claim for this namespace
                    if let Some(ns) = credential.claims.get("namespace") {
                        if ns == namespace {
                            return true;
                        }
                    }

                    // Check if credential has a claim for this cooperative
                    if namespace.starts_with("coops/") {
                        let parts: Vec<&str> = namespace.split('/').collect();
                        if parts.len() >= 2 {
                            let coop_id = parts[1];
                            if let Some(claim_coop) = credential.claims.get("cooperative_id") {
                                if claim_coop == coop_id {
                                    return true;
                                }
                            }
                        }
                    }
                }
            }
        }

        false
    }

    /// Check if one identity has delegated to another
    pub fn has_delegation(&self, delegator_id: &str, delegate_id: &str) -> bool {
        // Check direct delegations in the registry
        if let Some(registry) = &self.delegation_registry {
            for delegation in registry.values() {
                if delegation.delegator_id == delegator_id
                    && delegation.delegate_id == delegate_id
                    && !delegation.is_expired(crate::storage::utils::now())
                {
                    return true;
                }
            }
        }

        // Also check the old delegations system for backward compatibility
        if let Some(delegator) = self.delegations.get(delegate_id) {
            return delegator == delegator_id;
        }

        false
    }

    /// Verify an identity's signature
    pub fn verify_signature(&self, identity_id: &str, message: &str, signature: &str) -> bool {
        if let Some(identity) = self.get_identity(identity_id) {
            if let Some(public_key) = &identity.public_key {
                if let Some(_crypto_scheme) = &identity.crypto_scheme {
                    // This is a placeholder - in a real implementation, we would:
                    // 1. Decode the signature from base64
                    // 2. Use the appropriate crypto library based on crypto_scheme
                    // 3. Verify the signature against the message and public key

                    // For now, simulate a successful verification if everything is present
                    return !public_key.is_empty() && !signature.is_empty() && !message.is_empty();
                }
            }
        }

        false
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
        self.delegations
            .insert(delegate_id.to_string(), self.user_id.clone());
    }

    // Check if the user `delegate_id` is a delegate of this context's user
    pub fn is_delegate(&self, delegate_id: &str) -> bool {
        self.delegations.contains_key(delegate_id)
    }

    // Check if this user is a delegate of `delegator_id`
    pub fn is_delegate_of(&self, delegator_id: &str) -> bool {
        // Check if the map has an entry where delegator_id is the value
        self.delegations.values().any(|id| id == delegator_id)
    }

    // Get delegations for this user
    pub fn get_delegations(&self) -> Vec<(&String, &String)> {
        self.delegations.iter().collect()
    }
}
