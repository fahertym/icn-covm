use icn_covm::storage::auth::AuthContext;
use icn_covm::storage::implementations::in_memory::InMemoryStorage;
use icn_covm::storage::traits::StorageBackend;
use icn_covm::storage::errors::StorageResult;

/// Creates an admin AuthContext with full permissions
pub fn create_admin_auth() -> AuthContext {
    let mut auth = AuthContext::with_roles("admin_user", vec!["admin".to_string()]);
    auth.add_role("global", "admin");
    auth.add_role("governance", "admin");
    auth.add_role("governance/config", "admin");
    auth.add_role("governance/proposals", "admin");
    auth.add_role("governance/votes", "admin");
    auth.add_role("governance/members", "admin");
    auth.add_role("governance/delegations", "admin");
    auth
}

/// Creates a member AuthContext with standard member permissions
pub fn create_member_auth(user_id: &str) -> AuthContext {
    let mut auth = AuthContext::with_roles(user_id, vec!["member".to_string()]);
    auth.add_role("governance", "member");
    // Add read permissions to all governance namespaces
    auth.add_role("governance/config", "reader");
    auth.add_role("governance/proposals", "reader");
    auth.add_role("governance/votes", "reader");
    auth.add_role("governance/members", "reader");
    auth.add_role("governance/delegations", "reader");
    // Add writer permission to specific member namespaces
    auth.add_role(&format!("governance/votes/{}", user_id), "writer");
    auth
}

/// Creates an observer AuthContext with read-only permissions
pub fn create_observer_auth(user_id: &str) -> AuthContext {
    let mut auth = AuthContext::with_roles(user_id, vec!["observer".to_string()]);
    auth.add_role("governance", "observer");
    auth.add_role("governance/proposals", "reader");
    auth
}

/// Sets up a standard InMemoryStorage with initialized namespaces
pub fn setup_test_storage() -> StorageResult<InMemoryStorage> {
    let mut storage = InMemoryStorage::new();
    let admin = create_admin_auth();
    
    // Create standard namespaces
    storage.create_namespace(Some(&admin), "governance", 1024*1024, None)?;
    storage.create_namespace(Some(&admin), "governance/config", 1024*1024, Some("governance"))?;
    storage.create_namespace(Some(&admin), "governance/proposals", 1024*1024, Some("governance"))?;
    storage.create_namespace(Some(&admin), "governance/votes", 1024*1024, Some("governance"))?;
    storage.create_namespace(Some(&admin), "governance/members", 1024*1024, Some("governance"))?;
    storage.create_namespace(Some(&admin), "governance/delegations", 1024*1024, Some("governance"))?;
    
    // Create member-specific vote namespaces
    storage.create_namespace(Some(&admin), "governance/votes/member1", 1024*1024, Some("governance/votes"))?;
    storage.create_namespace(Some(&admin), "governance/votes/member2", 1024*1024, Some("governance/votes"))?;
    
    // Create resource accounts
    storage.create_account(Some(&admin), "admin_user", 1024*1024)?;
    storage.create_account(Some(&admin), "member1", 1024*1024)?;
    storage.create_account(Some(&admin), "member2", 1024*1024)?;
    storage.create_account(Some(&admin), "observer_user", 1024*100)?;
    
    Ok(storage)
}

/// Converts a string to bytes for storage
pub fn to_bytes(value: &str) -> Vec<u8> {
    value.as_bytes().to_vec()
}

/// Converts bytes from storage to a string (with error handling)
pub fn from_bytes(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).to_string()
} 