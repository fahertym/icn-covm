use serde::{Serialize, Deserialize};
use did_key::{generate, DidKey, Ed25519KeyPair, CONFIG_LD_PUBLIC};
use ed25519_dalek::{SigningKey, Signature, VerifyingKey, Signer, Verifier};
use rand::rngs::OsRng;
use std::collections::HashMap;

// Error type for identity operations
#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    #[error("Key generation failed: {0}")]
    KeyGeneration(String),
    #[error("DID generation failed: {0}")]
    DidGeneration(String),
    #[error("Signing failed: {0}")]
    SigningError(String),
    #[error("Verification failed: {0}")]
    VerificationError(String),
    #[error("Invalid key material")]
    InvalidKeyMaterial,
    #[error("Serialization error: {0}")]
    Serialization(String),
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

/// Represents a digital identity based on did:key with associated profile.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Identity {
    /// The decentralized identifier (did:key:...). Required.
    pub did: String,
    /// The keypair (keep private key secret!). Required.
    keypair: Ed25519KeyPair, 
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
        let mut csprng = OsRng{};
        let signing_key: SigningKey = SigningKey::generate(&mut csprng);
        let verifying_key: VerifyingKey = signing_key.verifying_key();

        let keypair = Ed25519KeyPair {
            public_key: verifying_key.to_bytes().to_vec(),
            private_key: Some(signing_key.to_bytes().to_vec()),
        };

        let did_key = generate::<Ed25519KeyPair>(&keypair);
        let did = did_key.to_string();

        let profile = Profile {
            public_username,
            full_name,
            other_fields: other_profile_fields.unwrap_or_default(),
        };

        Ok(Self {
            did,
            keypair,
            profile,
            identity_type,
        })
    }

    /// Returns the DID string.
    pub fn did(&self) -> &str {
        &self.did
    }

    /// Returns the public key bytes.
    pub fn public_key_bytes(&self) -> &[u8] {
        &self.keypair.public_key
    }

    /// Signs a message using the identity's private key.
    /// Returns the signature as bytes.
    pub fn sign(&self, message: &[u8]) -> Result<Signature, IdentityError> {
        let private_bytes = self.keypair.private_key.as_ref()
            .ok_or(IdentityError::InvalidKeyMaterial)?;
        let signing_key = SigningKey::from_bytes(
                private_bytes.as_slice().try_into().map_err(|_| IdentityError::InvalidKeyMaterial)?
            );
        Ok(signing_key.sign(message))
    }

    /// Verifies a signature against the identity's public key.
    pub fn verify(&self, message: &[u8], signature: &Signature) -> Result<(), IdentityError> {
        let verifying_key = VerifyingKey::from_bytes(
            &self.keypair.public_key.as_slice().try_into().map_err(|_| IdentityError::InvalidKeyMaterial)?
        ).map_err(|e| IdentityError::VerificationError(e.to_string()))?;
        
        verifying_key.verify(message, signature)
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
            public_key_multibase: String,
            profile: &'a Profile,
            identity_type: &'a str,
        }

        let public_key_multibase = multibase::encode(multibase::Base::Base58Btc, &self.keypair.public_key);

        let public_id = PublicIdentity {
            did: &self.did,
            public_key_multibase,
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
        let identity = Identity::new("test_user".to_string(), None, "member".to_string(), None).unwrap();
        assert_eq!(identity.profile.public_username, "test_user");
        assert!(identity.did.starts_with("did:key:"));
        assert!(identity.public_key_multibase.starts_with("z")); // z is the prefix for Ed25519 multibase
        // Private key is not stored directly, so we can't assert on it
    }

    #[test]
    fn test_sign_verify_ok() {
        let identity = Identity::new("signer".to_string(), None, "member".to_string(), None).unwrap();
        let message = b"This is a test message";

        let signature_multibase = identity.sign(message).unwrap();
        assert!(signature_multibase.starts_with("z")); // Signature should also be multibase encoded

        // Verification should succeed with correct message and signature
        let verification_result = identity.verify(message, &signature_multibase);
        assert!(verification_result.is_ok());
    }

    #[test]
    fn test_verify_bad_signature() {
        let identity = Identity::new("verifier".to_string(), None, "member".to_string(), None).unwrap();
        let message = b"Another test message";

        // Create a signature from a *different* identity
        let other_identity = Identity::new("bad_signer".to_string(), None, "member".to_string(), None).unwrap();
        let bad_signature = other_identity.sign(message).unwrap();

        // Verification should fail with the wrong signature
        let verification_result = identity.verify(message, &bad_signature);
        assert!(verification_result.is_err());
        // Optionally check the error type
        match verification_result.unwrap_err() {
            IdentityError::DidKeyError(_) => {} // Expected error type
            _ => panic!("Expected DidKeyError for bad signature verification"),
        }
    }

    #[test]
    fn test_verify_wrong_message() {
        let identity = Identity::new("msg_verifier".to_string(), None, "member".to_string(), None).unwrap();
        let message1 = b"Original message";
        let message2 = b"Tampered message";

        let signature = identity.sign(message1).unwrap();

        // Verification should fail with the correct signature but wrong message
        let verification_result = identity.verify(message2, &signature);
        assert!(verification_result.is_err());
        match verification_result.unwrap_err() {
             IdentityError::DidKeyError(_) => {} // Expected error type
             _ => panic!("Expected DidKeyError for wrong message verification"),
         }
    }

    // Optional: Test serialization/deserialization if needed
    // #[test]
    // fn test_identity_serde() {
    //     let identity = Identity::new("serde_user".to_string(), None, "member".to_string(), None).unwrap();
    //     let json = identity.to_json().unwrap();
    //     let deserialized_identity = Identity::from_json(&json).unwrap();
    //     assert_eq!(identity.did, deserialized_identity.did);
    //     assert_eq!(identity.profile, deserialized_identity.profile);
    //     assert_eq!(identity.public_key_multibase, deserialized_identity.public_key_multibase);
    //     assert_eq!(identity.identity_type, deserialized_identity.identity_type);
    //     assert_eq!(identity.metadata, deserialized_identity.metadata);
    // }
} 