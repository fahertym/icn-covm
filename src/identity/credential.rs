use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::storage::versioning::VersionInfo;

/// Credential that can be issued to identities
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Credential {
    /// Unique identifier for this credential
    pub id: String,

    /// Type of credential (e.g., "membership", "voting_right", "admin_access")
    pub credential_type: String,

    /// Identity ID that issued this credential
    pub issuer_id: String,

    /// Identity ID that holds this credential
    pub holder_id: String,

    /// Timestamp when issued
    pub issued_at: u64,

    /// Optional expiration timestamp
    pub expires_at: Option<u64>,

    /// Cryptographic signature from the issuer
    pub signature: Option<Vec<u8>>,

    /// Claims associated with this credential
    pub claims: HashMap<String, String>,

    /// Version information for this credential
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version_info: Option<VersionInfo>,
}

impl Credential {
    /// Create a new credential
    pub fn new(
        id: &str,
        credential_type: &str,
        issuer_id: &str,
        holder_id: &str,
        issued_at: u64,
    ) -> Self {
        Self {
            id: id.to_string(),
            credential_type: credential_type.to_string(),
            issuer_id: issuer_id.to_string(),
            holder_id: holder_id.to_string(),
            issued_at,
            expires_at: None,
            signature: None,
            claims: HashMap::new(),
            version_info: None,
        }
    }

    /// Set expiration timestamp
    pub fn with_expiration(&mut self, expires_at: u64) -> &mut Self {
        self.expires_at = Some(expires_at);
        self
    }

    /// Add a claim to this credential
    pub fn add_claim(&mut self, key: &str, value: &str) -> &mut Self {
        self.claims.insert(key.to_string(), value.to_string());
        self
    }

    /// Set the signature after all claims are added
    pub fn sign(&mut self, signature: Vec<u8>) -> &mut Self {
        self.signature = Some(signature);
        self
    }

    /// Check if the credential is expired
    pub fn is_expired(&self, current_time: u64) -> bool {
        match self.expires_at {
            Some(expires) => current_time > expires,
            None => false,
        }
    }

    /// Check if the credential has a valid signature
    pub fn is_signed(&self) -> bool {
        self.signature.is_some()
    }

    /// Get the namespace for this credential
    pub fn get_namespace(&self) -> String {
        format!("credentials/{}/{}", self.credential_type, self.id)
    }
}
