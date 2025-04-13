use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Error type for identity operations
#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    #[error("Key generation failed: {0}")]
    KeyGeneration(String),
    #[error("DID generation failed: {0}")]
    DidGeneration(String),
    #[error("Signing error: {0}")]
    SigningError(String),
    #[error("Verification error: {0}")]
    VerificationError(String),
    #[error("Invalid key material")]
    InvalidKeyMaterial,
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Multibase error: {0}")]
    MultibaseError(String),
    #[error("Profile field missing: {0}")]
    ProfileFieldMissing(String),
}

/// Represents profile information associated with an identity.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Profile {
    /// Publicly visible username (required).
    pub public_username: String,
    /// Optional private full name.
    pub full_name: Option<String>,
    /// Other optional profile fields.
    #[serde(flatten)]
    pub other_fields: HashMap<String, serde_json::Value>,
}

/// Represents a digital identity with associated profile.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Identity {
    /// The decentralized identifier (did:key:...). Required.
    pub did: String,
    /// Public key bytes (stored for verification).
    #[serde(with = "serde_bytes")]
    pub public_key_bytes: Vec<u8>,
    /// Private key bytes (should be kept secret).
    #[serde(with = "serde_bytes", skip_serializing_if = "Option::is_none")]
    pub private_key_bytes: Option<Vec<u8>>,
    /// Public key in multibase format
    pub public_key_multibase: String,
    /// Associated profile information. Required.
    pub profile: Profile,
    /// Type of identity (e.g., "member", "cooperative", "service"). Required.
    pub identity_type: String,
}

impl Identity {
    /// Creates a new Identity with a generated Ed25519 keypair and did:key.
    pub fn new(
        public_username: String,
        full_name: Option<String>,
        identity_type: String,
        other_profile_fields: Option<HashMap<String, serde_json::Value>>,
    ) -> Result<Self, IdentityError> {
        // Generate new Ed25519 keypair
        let mut csprng = OsRng {};
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();

        let public_key_bytes = verifying_key.to_bytes().to_vec();
        let private_key_bytes = Some(signing_key.to_bytes().to_vec());

        // Create multibase encoded public key
        let public_key_multibase = multibase::encode(multibase::Base::Base58Btc, &public_key_bytes);

        // Generate did:key identifier
        // Format: did:key:z + multibase encoded public key
        let did = format!("did:key:{}", public_key_multibase);

        let profile = Profile {
            public_username,
            full_name,
            other_fields: other_profile_fields.unwrap_or_default(),
        };

        Ok(Self {
            did,
            public_key_bytes,
            private_key_bytes,
            public_key_multibase,
            profile,
            identity_type,
        })
    }

    /// Returns the DID string.
    pub fn did(&self) -> &str {
        &self.did
    }

    /// Signs a message using the identity's private key.
    /// Returns the signature as a multibase encoded string.
    pub fn sign(&self, message: &[u8]) -> Result<String, IdentityError> {
        let private_bytes = self
            .private_key_bytes
            .as_ref()
            .ok_or(IdentityError::InvalidKeyMaterial)?;

        let signing_key = SigningKey::from_bytes(
            private_bytes
                .as_slice()
                .try_into()
                .map_err(|_| IdentityError::InvalidKeyMaterial)?,
        );

        let signature = signing_key.sign(message);
        Ok(multibase::encode(
            multibase::Base::Base58Btc,
            signature.to_bytes(),
        ))
    }

    /// Verifies a multibase-encoded signature against the identity's public key.
    pub fn verify(&self, message: &[u8], signature_multibase: &str) -> Result<(), IdentityError> {
        let verifying_key = VerifyingKey::from_bytes(
            &self
                .public_key_bytes
                .as_slice()
                .try_into()
                .map_err(|_| IdentityError::InvalidKeyMaterial)?,
        )
        .map_err(|e| IdentityError::VerificationError(e.to_string()))?;

        // Decode the multibase signature
        let (_, sig_bytes) = multibase::decode(signature_multibase).map_err(|e| {
            IdentityError::MultibaseError(format!("Invalid signature format: {}", e))
        })?;

        // Convert to ed25519 Signature
        let signature = Signature::from_bytes(
            &sig_bytes
                .try_into()
                .map_err(|_| IdentityError::InvalidKeyMaterial)?,
        );

        verifying_key
            .verify(message, &signature)
            .map_err(|e| IdentityError::VerificationError(e.to_string()))
    }

    /// Returns the public username.
    pub fn public_username(&self) -> &str {
        &self.profile.public_username
    }

    /// Serializes the identity (excluding private key) to JSON.
    pub fn to_public_json(&self) -> Result<String, IdentityError> {
        #[derive(Serialize)]
        struct PublicIdentity<'a> {
            did: &'a str,
            public_key_multibase: &'a str,
            profile: &'a Profile,
            identity_type: &'a str,
        }

        let public_id = PublicIdentity {
            did: &self.did,
            public_key_multibase: &self.public_key_multibase,
            profile: &self.profile,
            identity_type: &self.identity_type,
        };
        serde_json::to_string(&public_id).map_err(|e| IdentityError::Serialization(e.to_string()))
    }

    // Add methods to load from storage, update profile etc. as needed
}

#[cfg(test)]
mod tests {
    use super::*; // Import everything from the parent module (Identity, etc.)

    #[test]
    fn test_create_identity() {
        let identity =
            Identity::new("test_user".to_string(), None, "member".to_string(), None).unwrap();
        assert_eq!(identity.profile.public_username, "test_user");
        assert!(identity.did.starts_with("did:key:z")); // z is the prefix for base58btc multibase
        assert!(identity.public_key_multibase.starts_with("z")); // z is the prefix for base58btc multibase
    }

    #[test]
    fn test_sign_verify_ok() {
        let identity =
            Identity::new("signer".to_string(), None, "member".to_string(), None).unwrap();
        let message = b"This is a test message";

        let signature_multibase = identity.sign(message).unwrap();
        assert!(signature_multibase.starts_with("z")); // Signature should also be multibase encoded

        // Verification should succeed with correct message and signature
        let verification_result = identity.verify(message, &signature_multibase);
        assert!(verification_result.is_ok());
    }

    #[test]
    fn test_verify_bad_signature() {
        let identity =
            Identity::new("verifier".to_string(), None, "member".to_string(), None).unwrap();
        let message = b"Another test message";

        // Create a signature from a *different* identity
        let other_identity =
            Identity::new("bad_signer".to_string(), None, "member".to_string(), None).unwrap();
        let bad_signature = other_identity.sign(message).unwrap();

        // Verification should fail with the wrong signature
        let verification_result = identity.verify(message, &bad_signature);
        assert!(verification_result.is_err());
        // Check the error type
        match verification_result.unwrap_err() {
            IdentityError::VerificationError(_) => {} // Expected error type
            err => panic!(
                "Expected VerificationError for bad signature verification, got: {:?}",
                err
            ),
        }
    }

    #[test]
    fn test_verify_wrong_message() {
        let identity =
            Identity::new("msg_verifier".to_string(), None, "member".to_string(), None).unwrap();
        let message1 = b"Original message";
        let message2 = b"Tampered message";

        let signature = identity.sign(message1).unwrap();

        // Verification should fail with the correct signature but wrong message
        let verification_result = identity.verify(message2, &signature);
        assert!(verification_result.is_err());
        match verification_result.unwrap_err() {
            IdentityError::VerificationError(_) => {} // Expected error type
            err => panic!(
                "Expected VerificationError for wrong message verification, got: {:?}",
                err
            ),
        }
    }

    #[test]
    fn test_to_public_json() {
        let identity = Identity::new(
            "json_user".to_string(),
            Some("Full Name".to_string()),
            "member".to_string(),
            None,
        )
        .unwrap();
        let json = identity.to_public_json().unwrap();

        // Basic validation that the JSON contains expected fields
        assert!(json.contains(&identity.did));
        assert!(json.contains(&identity.public_key_multibase));
        assert!(json.contains("json_user"));
        assert!(json.contains("Full Name"));
        assert!(json.contains("member"));

        // Ensure private key is not included
        assert!(!json.contains("private_key"));
    }
}
