use icn_covm::storage::auth::*;
use icn_covm::storage::error::*;
use icn_covm::storage::errors::{StorageError, StorageResult};
use icn_covm::storage::implementations::in_memory::InMemoryStorage;
use icn_covm::storage::implementations::*;
use icn_covm::storage::traits::StorageBackend;
use icn_covm::storage::utils::*;
use icn_covm::storage::*;
use icn_covm::Identity;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use temp_testdir::TempDir;

mod test_helpers;
use test_helpers::{
    create_admin_auth, create_member_auth, create_observer_auth, from_bytes, to_bytes,
};

#[test]
fn test_rbac_basic() -> StorageResult<()> {
    let mut storage = InMemoryStorage::new();
    let admin = create_admin_auth();

    // Create necessary namespaces and accounts
    storage.create_namespace(Some(&admin), "governance", 1024 * 1024, None)?;
    storage.create_namespace(
        Some(&admin),
        "governance/config",
        1024 * 1024,
        Some("governance"),
    )?;
    storage.create_namespace(
        Some(&admin),
        "governance/votes",
        1024 * 1024,
        Some("governance"),
    )?;
    storage.create_namespace(
        Some(&admin),
        "governance/votes/member1",
        1024 * 1024,
        Some("governance/votes"),
    )?;
    storage.create_account(Some(&admin), "admin_user", 1024 * 1024)?;
    storage.create_account(Some(&admin), "member1", 1024 * 1024)?;
    storage.create_account(Some(&admin), "observer_user", 1024 * 100)?;

    // Get predefined users
    let admin = create_admin_auth();
    let member = create_member_auth("member1");
    let observer = create_observer_auth("observer_user");

    // Admin can write to governance namespace
    storage.set(
        Some(&admin),
        "governance/config",
        "vote_threshold",
        to_bytes("66.0"),
    )?;

    // Member can read from governance namespace
    assert_eq!(
        from_bytes(&storage.get(Some(&member), "governance/config", "vote_threshold")?),
        "66.0"
    );

    // Member should be able to write to their own votes
    storage.set(
        Some(&member),
        "governance/votes/member1",
        "prop-001",
        to_bytes("1.0"),
    )?;

    // Member cannot write to governance config
    let result = storage.set(
        Some(&member),
        "governance/config",
        "vote_threshold",
        to_bytes("50.0"),
    );
    assert!(result.is_err());

    // Observer cannot write to governance namespace
    let result = storage.set(
        Some(&observer),
        "governance/votes",
        "prop-001/observer_user",
        to_bytes("1.0"),
    );
    assert!(result.is_err());
    if let Err(StorageError::PermissionDenied { .. }) = result {
        // Expected error
    } else {
        panic!("Expected PermissionDenied error, got: {:?}", result);
    }

    Ok(())
}

#[test]
fn test_governance_namespaces() -> StorageResult<()> {
    let mut storage = InMemoryStorage::new();
    let admin = create_admin_auth();

    // Create necessary namespaces and accounts
    storage.create_namespace(Some(&admin), "governance", 1024 * 1024, None)?;
    storage.create_namespace(
        Some(&admin),
        "governance/proposals",
        1024 * 1024,
        Some("governance"),
    )?;
    storage.create_namespace(
        Some(&admin),
        "governance/votes",
        1024 * 1024,
        Some("governance"),
    )?;
    storage.create_namespace(
        Some(&admin),
        "governance/delegations",
        1024 * 1024,
        Some("governance"),
    )?;
    storage.create_namespace(
        Some(&admin),
        "governance/members",
        1024 * 1024,
        Some("governance"),
    )?;
    storage.create_namespace(
        Some(&admin),
        "governance/config",
        1024 * 1024,
        Some("governance"),
    )?;
    storage.create_account(Some(&admin), "admin_user", 1024 * 1024)?;

    // Test different governance namespaces
    storage.set(
        Some(&admin),
        "governance/proposals",
        "prop-001",
        to_bytes("Proposal data"),
    )?;
    storage.set(
        Some(&admin),
        "governance/votes",
        "prop-001/admin_user",
        to_bytes("1.0"),
    )?;
    storage.set(
        Some(&admin),
        "governance/delegations",
        "member1/member2",
        to_bytes("1.0"),
    )?;
    storage.set(
        Some(&admin),
        "governance/members",
        "member1",
        to_bytes("Member data"),
    )?;
    storage.set(Some(&admin), "governance/config", "quorum", to_bytes("0.5"))?;

    // Verify data was stored correctly
    assert_eq!(
        from_bytes(&storage.get(Some(&admin), "governance/proposals", "prop-001")?),
        "Proposal data"
    );
    assert_eq!(
        from_bytes(&storage.get(Some(&admin), "governance/votes", "prop-001/admin_user")?),
        "1.0"
    );
    assert_eq!(
        from_bytes(&storage.get(Some(&admin), "governance/delegations", "member1/member2")?),
        "1.0"
    );
    assert_eq!(
        from_bytes(&storage.get(Some(&admin), "governance/members", "member1")?),
        "Member data"
    );
    assert_eq!(
        from_bytes(&storage.get(Some(&admin), "governance/config", "quorum")?),
        "0.5"
    );

    Ok(())
}

#[test]
fn test_versioning() -> StorageResult<()> {
    let mut storage = InMemoryStorage::new();
    let admin = create_admin_auth();

    // Create necessary namespaces
    storage.create_namespace(Some(&admin), "governance", 1024 * 1024, None)?;
    storage.create_namespace(
        Some(&admin),
        "governance/proposals",
        1024 * 1024,
        Some("governance"),
    )?;
    storage.create_account(Some(&admin), "admin_user", 1024 * 1024)?;

    // Set up version data
    println!("Set Initial draft");
    storage.set(
        Some(&admin),
        "governance/proposals",
        "prop-001",
        "Initial draft".as_bytes().to_vec(),
    )?;

    println!("Set Revised draft");
    storage.set(
        Some(&admin),
        "governance/proposals",
        "prop-001",
        "Revised draft".as_bytes().to_vec(),
    )?;

    println!("Set Final version");
    storage.set(
        Some(&admin),
        "governance/proposals",
        "prop-001",
        "Final version".as_bytes().to_vec(),
    )?;

    // Test getting the latest version
    let latest_data = storage.get(Some(&admin), "governance/proposals", "prop-001")?;
    let latest_version = String::from_utf8(latest_data).unwrap();
    println!("Latest version: {}", latest_version);
    assert_eq!(latest_version, "Final version");

    // Test listing versions
    let versions = storage.list_versions(Some(&admin), "governance/proposals", "prop-001")?;
    println!("Found {} versions", versions.len());
    assert_eq!(versions.len(), 3);

    // Print version info for debugging
    for (i, v) in versions.iter().enumerate() {
        println!("Version {}: created by {}", i + 1, v.created_by);
    }

    // Get each specific version by its version number
    let (data, _) = storage.get_version(Some(&admin), "governance/proposals", "prop-001", 1)?;
    let version1 = String::from_utf8(data).unwrap();
    println!("Version 1: data: {}", version1);

    let (data, _) = storage.get_version(Some(&admin), "governance/proposals", "prop-001", 2)?;
    let version2 = String::from_utf8(data).unwrap();
    println!("Version 2: data: {}", version2);

    let (data, _) = storage.get_version(Some(&admin), "governance/proposals", "prop-001", 3)?;
    let version3 = String::from_utf8(data).unwrap();
    println!("Version 3: data: {}", version3);

    // Check that versions are stored in order from oldest to newest
    assert_eq!(version1, "Initial draft");
    assert_eq!(version2, "Revised draft");
    assert_eq!(version3, "Final version");

    Ok(())
}

#[test]
fn test_json_serialization() -> StorageResult<()> {
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct TestStruct {
        id: String,
        value: i32,
        metadata: Vec<String>,
    }

    let mut storage = InMemoryStorage::new();
    let admin = create_admin_auth();

    // Create necessary namespaces and account
    storage.create_namespace(Some(&admin), "governance", 1024 * 1024, None)?;
    storage.create_namespace(
        Some(&admin),
        "governance/config",
        1024 * 1024,
        Some("governance"),
    )?;
    storage.create_account(Some(&admin), "admin_user", 1024 * 1024)?;

    // Create a test struct
    let test_data = TestStruct {
        id: "test123".to_string(),
        value: 42,
        metadata: vec!["tag1".to_string(), "tag2".to_string()],
    };

    // Store as JSON
    storage.set_json(Some(&admin), "governance/config", "json_test", &test_data)?;

    // Retrieve the raw JSON
    let raw_json = storage.get(Some(&admin), "governance/config", "json_test")?;
    let raw_json_str = from_bytes(&raw_json);

    // Check for content in the JSON string
    assert!(raw_json_str.contains("test123"));
    assert!(raw_json_str.contains("42"));
    assert!(raw_json_str.contains("tag1"));

    // Retrieve and deserialize
    let retrieved: TestStruct = storage.get_json(Some(&admin), "governance/config", "json_test")?;
    assert_eq!(retrieved, test_data);

    Ok(())
}
