#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::{Identity, MemberProfile, Credential, DelegationLink};
    use crate::storage::utils::now;
    
    #[test]
    fn test_identity_creation() {
        let identity = Identity::new("alice", "member");
        
        assert_eq!(identity.id, "alice");
        assert_eq!(identity.identity_type, "member");
        assert!(identity.public_key.is_none());
        assert!(identity.crypto_scheme.is_none());
        assert!(identity.metadata.is_empty());
    }
    
    #[test]
    fn test_identity_with_public_key() {
        let public_key = vec![1, 2, 3, 4];
        let identity = Identity::with_public_key("bob", "member", public_key.clone(), "ed25519");
        
        assert_eq!(identity.id, "bob");
        assert_eq!(identity.identity_type, "member");
        assert_eq!(identity.public_key, Some(public_key));
        assert_eq!(identity.crypto_scheme, Some("ed25519".to_string()));
    }
    
    #[test]
    fn test_identity_metadata() {
        let mut identity = Identity::new("charlie", "member");
        identity.add_metadata("email", "charlie@example.com");
        identity.add_metadata("coop_id", "coop123");
        
        assert_eq!(identity.get_metadata("email"), Some(&"charlie@example.com".to_string()));
        assert_eq!(identity.get_metadata("coop_id"), Some(&"coop123".to_string()));
        assert_eq!(identity.get_metadata("nonexistent"), None);
    }
    
    #[test]
    fn test_identity_namespace() {
        // Cooperative namespace
        let coop = Identity::new("coop123", "cooperative");
        assert_eq!(coop.get_namespace(), "coops/coop123");
        
        // Member without coop
        let member1 = Identity::new("alice", "member");
        assert_eq!(member1.get_namespace(), "members/alice");
        
        // Member with coop
        let mut member2 = Identity::new("bob", "member");
        member2.add_metadata("coop_id", "coop123");
        assert_eq!(member2.get_namespace(), "coops/coop123/members/bob");
        
        // Other identity type
        let service = Identity::new("auth-svc", "service");
        assert_eq!(service.get_namespace(), "identities/service/auth-svc");
    }
    
    #[test]
    fn test_member_profile() {
        let identity = Identity::new("dave", "member");
        let timestamp = now();
        let mut profile = MemberProfile::new(identity, timestamp);
        
        assert_eq!(profile.joined_at, timestamp);
        assert!(profile.roles.is_empty());
        assert!(profile.reputation.is_none());
        
        profile.add_role("voter");
        profile.add_role("committee_member");
        
        assert!(profile.has_role("voter"));
        assert!(profile.has_role("committee_member"));
        assert!(!profile.has_role("admin"));
        
        profile.set_reputation(4.5);
        assert_eq!(profile.reputation, Some(4.5));
        
        profile.add_attribute("bio", "Cooperative developer");
        assert_eq!(profile.attributes.get("bio"), Some(&"Cooperative developer".to_string()));
    }
    
    #[test]
    fn test_credential() {
        let timestamp = now();
        let mut credential = Credential::new(
            "cred123",
            "membership",
            "coop123",
            "alice",
            timestamp,
        );
        
        assert_eq!(credential.id, "cred123");
        assert_eq!(credential.credential_type, "membership");
        assert_eq!(credential.issuer_id, "coop123");
        assert_eq!(credential.holder_id, "alice");
        assert_eq!(credential.issued_at, timestamp);
        assert!(credential.expires_at.is_none());
        
        // Set expiration
        let expiry = timestamp + 86400; // 1 day later
        credential.with_expiration(expiry);
        assert_eq!(credential.expires_at, Some(expiry));
        
        // Add claims
        credential.add_claim("role", "member");
        credential.add_claim("level", "full");
        
        assert_eq!(credential.claims.get("role"), Some(&"member".to_string()));
        assert_eq!(credential.claims.get("level"), Some(&"full".to_string()));
        
        // Sign credential
        let signature = vec![5, 6, 7, 8];
        credential.sign(signature);
        assert!(credential.is_signed());
        
        // Check expiration
        assert!(!credential.is_expired(timestamp));
        assert!(!credential.is_expired(timestamp + 43200)); // 12 hours later
        assert!(credential.is_expired(timestamp + 172800)); // 2 days later
    }
    
    #[test]
    fn test_delegation_link() {
        let timestamp = now();
        let mut delegation = DelegationLink::new(
            "del123",
            "alice",
            "bob",
            "voting",
            timestamp,
        );
        
        assert_eq!(delegation.id, "del123");
        assert_eq!(delegation.delegator_id, "alice");
        assert_eq!(delegation.delegate_id, "bob");
        assert_eq!(delegation.delegation_type, "voting");
        assert_eq!(delegation.created_at, timestamp);
        assert!(delegation.permissions.is_empty());
        
        // Add permissions
        delegation.add_permission("vote");
        delegation.add_permission("propose");
        
        assert!(delegation.has_permission("vote"));
        assert!(delegation.has_permission("propose"));
        assert!(!delegation.has_permission("veto"));
        
        // Set expiration
        let expiry = timestamp + 604800; // 1 week later
        delegation.with_expiration(expiry);
        assert_eq!(delegation.expires_at, Some(expiry));
        
        // Sign delegation
        let signature = vec![9, 10, 11, 12];
        delegation.sign(signature);
        assert!(delegation.is_signed());
        
        // Add attributes
        delegation.add_attribute("context", "annual_meeting");
        assert_eq!(delegation.attributes.get("context"), Some(&"annual_meeting".to_string()));
    }
} 