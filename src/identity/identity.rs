use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::storage::versioning::VersionInfo;

/// Core identity structure representing any entity in the cooperative system
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Identity {
    /// Unique identifier for the identity
    pub id: String,

    /// Public key for cryptographic verification (optional)
    pub public_key: Option<Vec<u8>>,

    /// Type of identity (e.g., "cooperative", "member", "service")
    pub identity_type: String,

    /// Cryptographic scheme used (e.g., "ed25519", "secp256k1")
    pub crypto_scheme: Option<String>,

    /// Additional metadata about this identity
    pub metadata: HashMap<String, String>,

    /// Version information tracking this identity's history
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_info: Option<VersionInfo>,
}

impl Identity {
    /// Create a new basic identity
    pub fn new(id: &str, identity_type: &str) -> Self {
        Self {
            id: id.to_string(),
            public_key: None,
            identity_type: identity_type.to_string(),
            crypto_scheme: None,
            metadata: HashMap::new(),
            version_info: None,
        }
    }

    /// Create an identity with a cryptographic public key
    pub fn with_public_key(
        id: &str,
        identity_type: &str,
        public_key: Vec<u8>,
        crypto_scheme: &str,
    ) -> Self {
        let mut identity = Self::new(id, identity_type);
        identity.public_key = Some(public_key);
        identity.crypto_scheme = Some(crypto_scheme.to_string());
        identity
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

    /// Get the namespace for this identity
    pub fn get_namespace(&self) -> String {
        match self.identity_type.as_str() {
            "cooperative" => format!("coops/{}", self.id),
            "member" => {
                // For members, if we know their coop, use a nested namespace
                if let Some(coop_id) = self.get_metadata("coop_id") {
                    format!("coops/{}/members/{}", coop_id, self.id)
                } else {
                    format!("members/{}", self.id)
                }
            }
            _ => format!("identities/{}/{}", self.identity_type, self.id),
        }
    }
}

/// Registry for managing identities
pub struct IdentityRegistry {
    // Map of ID -> Identity
    identities: HashMap<String, Identity>,
}

impl IdentityRegistry {
    /// Create a new empty identity registry
    pub fn new() -> Self {
        Self {
            identities: HashMap::new(),
        }
    }

    /// Register a new identity
    pub fn register(&mut self, identity: Identity) -> Result<(), String> {
        if self.identities.contains_key(&identity.id) {
            return Err(format!("Identity with ID {} already exists", identity.id));
        }
        self.identities.insert(identity.id.clone(), identity);
        Ok(())
    }

    /// Get an identity by ID
    pub fn get(&self, id: &str) -> Option<&Identity> {
        self.identities.get(id)
    }

    /// Update an existing identity
    pub fn update(&mut self, identity: Identity) -> Result<(), String> {
        if !self.identities.contains_key(&identity.id) {
            return Err(format!("Identity with ID {} does not exist", identity.id));
        }
        self.identities.insert(identity.id.clone(), identity);
        Ok(())
    }

    /// List identities of a specific type
    pub fn list_by_type(&self, identity_type: &str) -> Vec<&Identity> {
        self.identities
            .values()
            .filter(|identity| identity.identity_type == identity_type)
            .collect()
    }
}
