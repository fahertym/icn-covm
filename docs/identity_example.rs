// identity.rs - Example implementation of identity system in ICN-COVM

use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use ed25519_dalek::{Verifier, Signature, PublicKey};

/// Error types for identity operations
#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    #[error("Invalid identity: {0}")]
    InvalidIdentity(String),
    
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),
    
    #[error("Signature verification failed: {0}")]
    SignatureVerificationFailed(String),
    
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    
    #[error("Crypto error: {0}")]
    CryptoError(String),
}

/// Core identity structure
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Identity {
    /// Unique identifier for the identity
    pub id: String,
    
    /// Public key for verification (optional for some identity types)
    #[serde(with = "base64_option")]
    pub public_key: Option<Vec<u8>>,
    
    /// Cryptographic scheme used (e.g., "ed25519", "secp256k1")
    pub scheme: String,
    
    /// Additional metadata
    pub metadata: HashMap<String, String>,
    
    /// Roles associated with this identity
    pub roles: Vec<String>,
}

/// Helper module for base64 serialization of Option<Vec<u8>>
mod base64_option {
    use serde::{Deserialize, Deserializer, Serializer, Serialize};
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

    pub fn serialize<S>(bytes: &Option<Vec<u8>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match bytes {
            Some(bytes) => {
                let b64 = BASE64.encode(bytes);
                String::serialize(&b64, serializer)
            },
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Vec<u8>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let b64: Option<String> = Option::deserialize(deserializer)?;
        match b64 {
            Some(b64) => {
                BASE64.decode(b64.as_bytes())
                    .map(Some)
                    .map_err(serde::de::Error::custom)
            },
            None => Ok(None),
        }
    }
}

/// Authentication context for VM execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthContext {
    /// The identity performing the current operation
    pub caller: Identity,
    
    /// Optional delegation chain (for delegated actions)
    pub delegation_chain: Vec<Identity>,
    
    /// Timestamp when this context was created
    pub timestamp: u64,
    
    /// Random nonce to prevent replay attacks
    #[serde(with = "base64_serde")]
    pub nonce: Vec<u8>,
}

/// Helper module for base64 serialization of Vec<u8>
mod base64_serde {
    use serde::{Deserialize, Deserializer, Serializer, Serialize};
    use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

    pub fn serialize<S>(bytes: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let b64 = BASE64.encode(bytes);
        String::serialize(&b64, serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let b64: String = String::deserialize(deserializer)?;
        BASE64.decode(b64.as_bytes())
            .map_err(serde::de::Error::custom)
    }
}

impl Identity {
    /// Create a new identity with basic information
    pub fn new(id: &str, scheme: &str) -> Self {
        Identity {
            id: id.to_string(),
            public_key: None,
            scheme: scheme.to_string(),
            metadata: HashMap::new(),
            roles: Vec::new(),
        }
    }
    
    /// Create a cryptographic identity with a public key
    pub fn with_public_key(id: &str, scheme: &str, public_key: Vec<u8>) -> Self {
        let mut identity = Identity::new(id, scheme);
        identity.public_key = Some(public_key);
        identity
    }
    
    /// Add a role to this identity
    pub fn add_role(&mut self, role: &str) -> &mut Self {
        if !self.roles.contains(&role.to_string()) {
            self.roles.push(role.to_string());
        }
        self
    }
    
    /// Check if this identity has a specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.contains(&role.to_string())
    }
    
    /// Add metadata to this identity
    pub fn add_metadata(&mut self, key: &str, value: &str) -> &mut Self {
        self.metadata.insert(key.to_string(), value.to_string());
        self
    }
    
    /// Get metadata for a specific key
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }
    
    /// Verify a signature made by this identity
    pub fn verify_signature(&self, message: &[u8], signature: &[u8]) -> Result<bool, IdentityError> {
        match self.scheme.as_str() {
            "ed25519" => {
                let public_key = match &self.public_key {
                    Some(pk) => pk,
                    None => return Err(IdentityError::InvalidIdentity(
                        "Identity has no public key".to_string()
                    )),
                };
                
                let pk = PublicKey::from_bytes(public_key)
                    .map_err(|e| IdentityError::CryptoError(e.to_string()))?;
                    
                let sig = Signature::from_bytes(signature)
                    .map_err(|e| IdentityError::CryptoError(e.to_string()))?;
                    
                pk.verify(message, &sig)
                    .map(|_| true)
                    .map_err(|e| IdentityError::SignatureVerificationFailed(e.to_string()))
            },
            "none" => {
                // For testing, allow "none" scheme that always verifies
                Ok(true)
            },
            _ => Err(IdentityError::InvalidIdentity(
                format!("Unsupported signature scheme: {}", self.scheme)
            )),
        }
    }
}

impl AuthContext {
    /// Create a new authentication context with a caller identity
    pub fn new(caller: Identity, timestamp: u64, nonce: Vec<u8>) -> Self {
        AuthContext {
            caller,
            delegation_chain: Vec::new(),
            timestamp,
            nonce,
        }
    }
    
    /// Add a delegation to the chain
    pub fn add_delegation(&mut self, delegator: Identity) -> &mut Self {
        self.delegation_chain.push(delegator);
        self
    }
    
    /// Check if this context includes delegation
    pub fn is_delegated(&self) -> bool {
        !self.delegation_chain.is_empty()
    }
    
    /// Get the original delegator (the first in the chain)
    pub fn original_delegator(&self) -> Option<&Identity> {
        self.delegation_chain.first()
    }
    
    /// Check if the caller has a specific role
    pub fn caller_has_role(&self, role: &str) -> bool {
        self.caller.has_role(role)
    }
    
    /// Check if the caller is a specific identity
    pub fn is_caller(&self, id: &str) -> bool {
        self.caller.id == id
    }
}

// VM Operation additions
impl crate::vm::VM {
    /// GetCaller operation: Push the current caller's ID onto the stack
    pub fn op_getcaller(&mut self) -> Result<(), crate::vm::VMError> {
        if let Some(auth) = &self.auth_context {
            // For simplicity, we're storing the ID as a number
            // In a real implementation, you might want to use a TypedValue enum
            self.stack.push(auth.caller.id.parse::<f64>().unwrap_or(0.0));
            Ok(())
        } else {
            Err(crate::vm::VMError::AuthenticationRequired("No identity context available".to_string()))
        }
    }
    
    /// HasRole operation: Check if the caller has a specific role
    pub fn op_hasrole(&mut self, role: &str) -> Result<(), crate::vm::VMError> {
        if let Some(auth) = &self.auth_context {
            let has_role = auth.caller_has_role(role);
            // Push 0.0 for true (has role) or 1.0 for false (doesn't have role)
            // This matches the VM's convention for conditional values
            self.stack.push(if has_role { 0.0 } else { 1.0 });
            Ok(())
        } else {
            Err(crate::vm::VMError::AuthenticationRequired("No identity context available".to_string()))
        }
    }
    
    /// RequireRole operation: Abort if the caller lacks a specific role
    pub fn op_requirerole(&mut self, role: &str) -> Result<(), crate::vm::VMError> {
        if let Some(auth) = &self.auth_context {
            if auth.caller_has_role(role) {
                Ok(())
            } else {
                Err(crate::vm::VMError::PermissionDenied(format!(
                    "Role '{}' required, but caller has roles: {:?}", 
                    role, 
                    auth.caller.roles
                )))
            }
        } else {
            Err(crate::vm::VMError::AuthenticationRequired("No identity context available".to_string()))
        }
    }
    
    /// RequireIdentity operation: Abort if the caller isn't the specified identity
    pub fn op_requireidentity(&mut self, id: &str) -> Result<(), crate::vm::VMError> {
        if let Some(auth) = &self.auth_context {
            if auth.is_caller(id) {
                Ok(())
            } else {
                Err(crate::vm::VMError::PermissionDenied(format!(
                    "Identity '{}' required, but caller is '{}'", 
                    id, 
                    auth.caller.id
                )))
            }
        } else {
            Err(crate::vm::VMError::AuthenticationRequired("No identity context available".to_string()))
        }
    }
    
    /// VerifySignature operation: Verify a cryptographic signature against a message
    pub fn op_verifysignature(&mut self) -> Result<(), crate::vm::VMError> {
        let scheme = self.pop_string("VerifySignature scheme")?;
        let public_key_b64 = self.pop_string("VerifySignature public_key")?;
        let signature_b64 = self.pop_string("VerifySignature signature")?;
        let message = self.pop_string("VerifySignature message")?;
        
        // Decode base64 inputs
        let public_key = BASE64.decode(public_key_b64.as_bytes())
            .map_err(|e| crate::vm::VMError::InvalidArgument(format!(
                "Invalid base64 public key: {}", e
            )))?;
            
        let signature = BASE64.decode(signature_b64.as_bytes())
            .map_err(|e| crate::vm::VMError::InvalidArgument(format!(
                "Invalid base64 signature: {}", e
            )))?;
        
        // Create a temporary identity for verification
        let temp_identity = Identity::with_public_key("temp", &scheme, public_key);
        
        // Verify the signature
        match temp_identity.verify_signature(message.as_bytes(), &signature) {
            Ok(true) => {
                self.stack.push(0.0); // Truthy value (signature valid)
                Ok(())
            },
            Ok(false) => {
                self.stack.push(1.0); // Falsey value (signature invalid)
                Ok(())
            },
            Err(e) => {
                Err(crate::vm::VMError::InvalidArgument(format!(
                    "Signature verification error: {}", e
                )))
            }
        }
    }
    
    /// GetCallerMetadata operation: Access caller metadata for the specified key
    pub fn op_getcallermetadata(&mut self, key: &str) -> Result<(), crate::vm::VMError> {
        if let Some(auth) = &self.auth_context {
            if let Some(value) = auth.caller.get_metadata(key) {
                // For simplicity, we're trying to convert the metadata to a number
                // In a real implementation, you might want to use a TypedValue enum
                if let Ok(num) = value.parse::<f64>() {
                    self.stack.push(num);
                    return Ok(());
                } else {
                    return Err(crate::vm::VMError::TypeMismatch {
                        expected: "number".to_string(),
                        found: "string".to_string(),
                        operation: format!("GetCallerMetadata({})", key),
                    });
                }
            } else {
                return Err(crate::vm::VMError::VariableNotFound(format!(
                    "Metadata key '{}' not found for caller", key
                )));
            }
        } else {
            Err(crate::vm::VMError::AuthenticationRequired("No identity context available".to_string()))
        }
    }
    
    /// Helper method to pop a string from the stack
    fn pop_string(&mut self, context: &str) -> Result<String, crate::vm::VMError> {
        // In a real implementation, this would handle TypedValue properly
        // For this example, we're assuming the stack contains string indices into a string table
        let value = self.pop_one(context)?;
        let string_id = value as usize;
        
        // This is a placeholder - in the real implementation you'd have a string table
        // For this example, just return a dummy string
        Ok(format!("string_{}", string_id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};
    
    fn create_test_identity() -> Identity {
        let mut identity = Identity::new("alice", "none");
        identity.add_role("user");
        identity.add_role("admin");
        identity.add_metadata("display_name", "Alice");
        identity.add_metadata("email", "alice@example.com");
        identity
    }
    
    #[test]
    fn test_identity_basic() {
        let identity = create_test_identity();
        
        assert_eq!(identity.id, "alice");
        assert_eq!(identity.scheme, "none");
        assert!(identity.has_role("user"));
        assert!(identity.has_role("admin"));
        assert!(!identity.has_role("guest"));
        assert_eq!(identity.get_metadata("display_name"), Some(&"Alice".to_string()));
        assert_eq!(identity.get_metadata("email"), Some(&"alice@example.com".to_string()));
        assert_eq!(identity.get_metadata("non_existent"), None);
    }
    
    #[test]
    fn test_auth_context() {
        let alice = create_test_identity();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let nonce = vec![1, 2, 3, 4];
        
        let mut auth = AuthContext::new(alice.clone(), now, nonce.clone());
        
        assert_eq!(auth.caller.id, "alice");
        assert!(auth.caller_has_role("admin"));
        assert!(!auth.is_delegated());
        
        // Add delegation
        let mut bob = Identity::new("bob", "none");
        bob.add_role("user");
        
        auth.add_delegation(bob.clone());
        
        assert!(auth.is_delegated());
        assert_eq!(auth.original_delegator().unwrap().id, "bob");
    }
    
    #[test]
    fn test_identity_serialization() {
        let identity = create_test_identity();
        
        let json = serde_json::to_string(&identity).unwrap();
        let deserialized: Identity = serde_json::from_str(&json).unwrap();
        
        assert_eq!(identity, deserialized);
    }
    
    #[test]
    fn test_auth_context_serialization() {
        let alice = create_test_identity();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let nonce = vec![1, 2, 3, 4];
        
        let auth = AuthContext::new(alice, now, nonce);
        
        let json = serde_json::to_string(&auth).unwrap();
        let deserialized: AuthContext = serde_json::from_str(&json).unwrap();
        
        assert_eq!(deserialized.caller.id, "alice");
        assert_eq!(deserialized.timestamp, now);
    }
} 