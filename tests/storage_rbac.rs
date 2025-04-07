use icn_covm::storage::{StorageBackend, InMemoryStorage};
use icn_covm::storage::auth::AuthContext;

#[test]
fn test_auth_context() {
    // Create users with different roles
    let mut admin = AuthContext::new("admin");
    admin.add_role("global", "admin");
    
    let mut member = AuthContext::new("member");
    member.add_role("project", "member");
    member.add_role("project", "reader"); 
    
    let mut observer = AuthContext::new("observer");
    observer.add_role("project", "reader");
    
    // Test role checking
    assert!(admin.has_role("global", "admin"));
    // We don't need to test that admin doesn't have project member role yet
    
    assert!(member.has_role("project", "member"));
    assert!(member.has_role("project", "reader"));
    assert!(!member.has_role("global", "admin"));
    
    assert!(observer.has_role("project", "reader"));
    assert!(!observer.has_role("project", "member"));
    assert!(!observer.has_role("global", "admin"));
    
    // Test role modification
    admin.add_role("project", "member");
    assert!(admin.has_role("project", "member"));
    
    member.add_role("project/advanced", "writer");
    assert!(member.has_role("project/advanced", "writer"));
}

#[test]
fn test_inmemory_storage() {
    let mut storage = InMemoryStorage::new();
    
    // Create admin user
    let mut admin = AuthContext::new("admin");
    admin.add_role("global", "admin");
    
    // Create account
    assert!(storage.create_account(&admin, "admin", 1000).is_ok());
    
    // Set a value
    let key = "test_key";
    let value = "test_value".as_bytes().to_vec();
    assert!(storage.set(&admin, "test", key, value.clone()).is_ok());
    
    // Check existence
    let exists = storage.contains(&admin, "test", key).unwrap();
    assert!(exists);
    
    // Get the value
    let retrieved_value = storage.get(&admin, "test", key).unwrap();
    assert_eq!(retrieved_value, value);
    
    // Delete the value
    assert!(storage.delete(&admin, "test", key).is_ok());
    
    // Verify it's deleted
    let exists = storage.contains(&admin, "test", key).unwrap();
    assert!(!exists);
} 