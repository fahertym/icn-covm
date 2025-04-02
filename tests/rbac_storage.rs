use icn_covm::storage::{StorageBackend, InMemoryStorage, AuthContext, StorageError};

#[test]
fn test_rbac_basic() {
    let mut storage = InMemoryStorage::new();
    
    // Create admin user
    let admin = AuthContext::with_roles("admin_user", vec!["admin".to_string()]);
    
    // Create regular member
    let member = AuthContext::with_roles("member_user", vec!["member".to_string()]);
    
    // Create observer user
    let observer = AuthContext::with_roles("observer_user", vec!["observer".to_string()]);
    
    // Admin can write to governance namespace
    assert!(storage.set_with_auth(&admin, "governance/config/vote_threshold", "66.0").is_ok());
    
    // Member can read from governance namespace
    assert!(storage.set_with_auth(&member, "governance/votes/prop-001/member_user", "1.0").is_ok());
    assert!(storage.get_with_auth(&member, "governance/config/vote_threshold").is_ok());
    
    // Member cannot write to governance config
    assert!(storage.set_with_auth(&member, "governance/config/vote_threshold", "50.0").is_err());
    
    // Observer cannot write to governance namespace
    let result = storage.set_with_auth(&observer, "governance/votes/prop-001/observer_user", "1.0");
    assert!(result.is_err());
    if let Err(StorageError::PermissionDenied(_)) = result {
        // Expected error
    } else {
        panic!("Expected PermissionDenied error, got: {:?}", result);
    }
}

#[test]
fn test_governance_namespaces() {
    let mut storage = InMemoryStorage::new();
    let admin = AuthContext::with_roles("admin_user", vec!["admin".to_string()]);
    
    // Test different governance namespaces
    assert!(storage.set_with_auth(&admin, "governance/proposals/prop-001", "Proposal data").is_ok());
    assert!(storage.set_with_auth(&admin, "governance/votes/prop-001/admin_user", "1.0").is_ok());
    assert!(storage.set_with_auth(&admin, "governance/delegations/member1/member2", "1.0").is_ok());
    assert!(storage.set_with_auth(&admin, "governance/members/member1", "Member data").is_ok());
    assert!(storage.set_with_auth(&admin, "governance/config/quorum", "0.5").is_ok());
    
    // Verify data was stored correctly
    assert_eq!(storage.get("governance/proposals/prop-001").unwrap(), "Proposal data");
    assert_eq!(storage.get("governance/votes/prop-001/admin_user").unwrap(), "1.0");
    assert_eq!(storage.get("governance/delegations/member1/member2").unwrap(), "1.0");
    assert_eq!(storage.get("governance/members/member1").unwrap(), "Member data");
    assert_eq!(storage.get("governance/config/quorum").unwrap(), "0.5");
}

#[test]
fn test_versioning() {
    let mut storage = InMemoryStorage::new();
    let admin = AuthContext::with_roles("admin_user", vec!["admin".to_string()]);
    
    // Create a proposal with multiple versions
    assert!(storage.set_with_auth(&admin, "governance/proposals/prop-001", "Initial draft").is_ok());
    assert!(storage.set_with_auth(&admin, "governance/proposals/prop-001", "Revised draft").is_ok());
    assert!(storage.set_with_auth(&admin, "governance/proposals/prop-001", "Final version").is_ok());
    
    // Get latest version
    assert_eq!(storage.get("governance/proposals/prop-001").unwrap(), "Final version");
    
    // Get specific versions
    assert_eq!(storage.get_versioned("governance/proposals/prop-001", 1).unwrap(), "Initial draft");
    assert_eq!(storage.get_versioned("governance/proposals/prop-001", 2).unwrap(), "Revised draft");
    assert_eq!(storage.get_versioned("governance/proposals/prop-001", 3).unwrap(), "Final version");
    
    // List versions
    let versions = storage.list_versions("governance/proposals/prop-001").unwrap();
    assert_eq!(versions.len(), 3);
    assert_eq!(versions[0].version, 1);
    assert_eq!(versions[1].version, 2);
    assert_eq!(versions[2].version, 3);
    assert_eq!(versions[0].author, "admin_user");
}

#[test]
fn test_resource_accounting() {
    let mut storage = InMemoryStorage::new();
    let admin = AuthContext::with_roles("admin_user", vec!["admin".to_string()]);
    
    // Create a resource account with limited quota
    let account_id = "admin_user";
    let mut account = storage.create_resource_account(account_id, 10.0);
    
    // Small values should work fine (assuming less than 10KB)
    let small_value = "Small value".repeat(10); // ~100 bytes
    assert!(storage.set_with_resources(&admin, "test_key1", &small_value, &mut account).is_ok());
    
    // Balance should be reduced
    assert!(account.balance < 10.0);
    
    // Large values exceeding quota should fail
    let large_value = "X".repeat(11 * 1024); // 11KB, exceeds the 10KB quota
    let result = storage.set_with_resources(&admin, "test_key2", &large_value, &mut account);
    assert!(result.is_err());
    
    if let Err(StorageError::QuotaExceeded(_)) = result {
        // Expected error
    } else {
        panic!("Expected QuotaExceeded error, got: {:?}", result);
    }
}

#[test]
fn test_transaction_support() {
    let mut storage = InMemoryStorage::new();
    
    // Begin a transaction
    assert!(storage.begin_transaction().is_ok());
    
    // Make some changes in the transaction
    assert!(storage.set("key1", "value1").is_ok());
    assert!(storage.set("key2", "value2").is_ok());
    
    // Values should be visible within the transaction
    assert_eq!(storage.get("key1").unwrap(), "value1");
    assert_eq!(storage.get("key2").unwrap(), "value2");
    
    // Roll back the transaction
    assert!(storage.rollback_transaction().is_ok());
    
    // Values should not exist after rollback
    assert!(storage.get("key1").is_err());
    assert!(storage.get("key2").is_err());
    
    // Begin a new transaction
    assert!(storage.begin_transaction().is_ok());
    
    // Make some changes
    assert!(storage.set("key3", "value3").is_ok());
    assert!(storage.set("key4", "value4").is_ok());
    
    // Commit the transaction
    assert!(storage.commit_transaction().is_ok());
    
    // Values should persist after commit
    assert_eq!(storage.get("key3").unwrap(), "value3");
    assert_eq!(storage.get("key4").unwrap(), "value4");
}

#[test]
fn test_json_serialization() {
    use serde::{Serialize, Deserialize};
    
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct TestStruct {
        id: String,
        value: i32,
        metadata: Vec<String>,
    }
    
    let mut storage = InMemoryStorage::new();
    
    // Create a test struct
    let test_data = TestStruct {
        id: "test123".to_string(),
        value: 42,
        metadata: vec!["tag1".to_string(), "tag2".to_string()],
    };
    
    // Store as JSON
    assert!(storage.set_json("json_test", &test_data).is_ok());
    
    // Retrieve the raw JSON
    let raw_json = storage.get("json_test").unwrap();
    assert!(raw_json.contains("test123"));
    assert!(raw_json.contains("42"));
    assert!(raw_json.contains("tag1"));
    
    // Retrieve and deserialize
    let retrieved: TestStruct = storage.get_json("json_test").unwrap();
    assert_eq!(retrieved, test_data);
} 