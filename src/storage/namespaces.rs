/// Namespace helper constants and functions

/// Constants for common namespace prefixes
pub const SYSTEM_NAMESPACE: &str = "system";
pub const GOVERNANCE_NAMESPACE: &str = "governance";
pub const USER_NAMESPACE: &str = "user";

/// Governance namespace helper for common key patterns
pub struct GovernanceNamespace;

impl GovernanceNamespace {
    /// Create a key in the proposals namespace
    pub fn proposals(proposal_id: &str) -> String {
        format!("{}/proposals/{}", GOVERNANCE_NAMESPACE, proposal_id)
    }
    
    /// Create a key in the votes namespace
    pub fn votes(proposal_id: &str, voter_id: &str) -> String {
        format!("{}/votes/{}/{}", GOVERNANCE_NAMESPACE, proposal_id, voter_id)
    }
    
    /// Create a key in the members namespace
    pub fn members(member_id: &str) -> String {
        format!("{}/members/{}", GOVERNANCE_NAMESPACE, member_id)
    }
    
    /// Create a key in the delegations namespace
    pub fn delegations(from: &str, to: &str) -> String {
        format!("{}/delegations/{}/{}", GOVERNANCE_NAMESPACE, from, to)
    }
    
    /// Create a key for proposal results
    pub fn results(proposal_id: &str) -> String {
        format!("{}/results/{}", GOVERNANCE_NAMESPACE, proposal_id)
    }
}

/// System namespace helper for internal VM state
pub struct SystemNamespace;

impl SystemNamespace {
    /// Create a key for storing VM configuration
    pub fn config(name: &str) -> String {
        format!("{}/config/{}", SYSTEM_NAMESPACE, name)
    }
    
    /// Create a key for storing account information
    pub fn accounts(account_id: &str) -> String {
        format!("{}/accounts/{}", SYSTEM_NAMESPACE, account_id)
    }
    
    /// Create a key for audit log entries
    pub fn audit_log(timestamp: u64) -> String {
        format!("{}/audit/{}", SYSTEM_NAMESPACE, timestamp)
    }
}

/// User namespace helper for user-specific data
pub struct UserNamespace;

impl UserNamespace {
    /// Create a key for user profile data
    pub fn profile(user_id: &str) -> String {
        format!("{}/{}/profile", USER_NAMESPACE, user_id)
    }
    
    /// Create a key for user settings
    pub fn settings(user_id: &str) -> String {
        format!("{}/{}/settings", USER_NAMESPACE, user_id)
    }
    
    /// Create a key for user-specific application data
    pub fn app_data(user_id: &str, app_id: &str) -> String {
        format!("{}/{}/apps/{}", USER_NAMESPACE, user_id, app_id)
    }
}

/// Check if a key belongs to a namespace
pub fn is_in_namespace(key: &str, namespace: &str) -> bool {
    key.starts_with(&format!("{}/", namespace))
}

/// Extract the namespace from a key
pub fn extract_namespace(key: &str) -> Option<&str> {
    key.split('/').next()
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_governance_namespace() {
        assert_eq!(GovernanceNamespace::proposals("prop1"), "governance/proposals/prop1");
        assert_eq!(GovernanceNamespace::votes("prop1", "user1"), "governance/votes/prop1/user1");
        assert_eq!(GovernanceNamespace::members("user1"), "governance/members/user1");
        assert_eq!(GovernanceNamespace::delegations("user1", "user2"), "governance/delegations/user1/user2");
    }
    
    #[test]
    fn test_system_namespace() {
        assert_eq!(SystemNamespace::config("max_memory"), "system/config/max_memory");
        assert_eq!(SystemNamespace::accounts("user1"), "system/accounts/user1");
        assert_eq!(SystemNamespace::audit_log(123456), "system/audit/123456");
    }
    
    #[test]
    fn test_namespace_checks() {
        assert!(is_in_namespace("governance/proposals/123", "governance"));
        assert!(!is_in_namespace("user/alice/profile", "governance"));
        
        assert_eq!(extract_namespace("governance/proposals/123"), Some("governance"));
        assert_eq!(extract_namespace("invalid"), Some("invalid"));
    }
}
