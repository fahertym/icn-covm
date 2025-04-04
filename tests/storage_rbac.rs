use icn_covm::storage::{AuthContext, InMemoryStorage, StorageBackend, StorageError};

#[test]
fn test_auth_context() {
    // Create different auth contexts
    let admin = AuthContext::with_roles("admin_user", vec!["admin".to_string()]);
    let member = AuthContext::with_roles("member_user", vec!["member".to_string()]);
    let observer = AuthContext::with_roles("observer_user", vec!["observer".to_string()]);

    // Test role checking
    assert!(admin.has_role("admin"));
    assert!(!admin.has_role("member"));
    assert!(member.has_role("member"));
    assert!(!member.has_role("admin"));
    assert!(observer.has_role("observer"));
    assert!(!observer.has_role("member"));

    // Test role modification
    let mut multi_role = AuthContext::new("multi_user");
    assert!(!multi_role.has_role("admin"));
    multi_role.add_role("admin");
    assert!(multi_role.has_role("admin"));
    multi_role.add_role("member");
    assert!(multi_role.has_role("member"));
}

#[test]
fn test_inmemory_storage() {
    let mut storage = InMemoryStorage::new();

    // Test basic operations
    assert!(storage.set("test_key", "test_value").is_ok());
    assert_eq!(storage.get("test_key").unwrap(), "test_value");
    assert!(storage.contains("test_key"));
    assert!(!storage.contains("nonexistent"));

    // Test deletion
    assert!(storage.delete("test_key").is_ok());
    assert!(!storage.contains("test_key"));
    assert!(storage.get("test_key").is_err());
}
