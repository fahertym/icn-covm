use icn_covm::storage::{AuthContext, InMemoryStorage, StorageBackend, StorageError};

#[test]
fn test_auth_context() {
    // Create different auth contexts
    let admin = AuthContext::with_roles("admin_user", vec!["admin".to_string()]);
    let member = AuthContext::with_roles("member_user", vec!["member".to_string()]);
    let observer = AuthContext::with_roles("observer_user", vec!["observer".to_string()]);
    
    // Test role checking
    assert!(admin.has_role("global", "admin"));
    assert!(!admin.has_role("global", "member"));
    assert!(member.has_role("global", "member"));
    assert!(!member.has_role("global", "admin"));
    assert!(observer.has_role("global", "observer"));
    assert!(!observer.has_role("global", "member"));
    
    // Test role modification
    let mut multi_role = AuthContext::new("multi_user");
    assert!(!multi_role.has_role("global", "admin"));
    multi_role.add_role("global", "admin");
    assert!(multi_role.has_role("global", "admin"));
    multi_role.add_role("global", "member");
    assert!(multi_role.has_role("global", "member"));
}

#[test]
fn test_inmemory_storage() {
    let mut storage = InMemoryStorage::new();
    let admin = AuthContext::new("admin");
    
    // Test basic operations
    assert!(storage.set(Some(&admin), "default", "test_key", "test_value".as_bytes().to_vec()).is_ok());
    assert_eq!(storage.get(Some(&admin), "default", "test_key").unwrap(), "test_value".as_bytes().to_vec());
    assert!(storage.contains(Some(&admin), "default", "test_key").unwrap());
    assert!(!storage.contains(Some(&admin), "default", "nonexistent").unwrap());
    
    // Test deletion
    assert!(storage.delete(Some(&admin), "default", "test_key").is_ok());
    assert!(!storage.contains(Some(&admin), "default", "test_key").unwrap());
    assert!(storage.get(Some(&admin), "default", "test_key").is_err());
} 