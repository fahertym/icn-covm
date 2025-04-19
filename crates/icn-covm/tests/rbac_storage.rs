use icn_covm::storage::errors::{StorageError, StorageResult};
use icn_covm::storage::traits::StorageExtensions;
use icn_covm::storage::StorageBackend;
use serde::{Deserialize, Serialize};

mod test_helpers;
use test_helpers::{
    create_admin_auth, create_member_auth, create_observer_auth, from_bytes, setup_test_storage,
    to_bytes,
};

#[test]
fn test_rbac_basic() -> StorageResult<()> {
    let mut storage = setup_test_storage()?;

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
    let mut storage = setup_test_storage()?;
    let admin = create_admin_auth();

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
    let mut storage = setup_test_storage()?;
    let admin = create_admin_auth();

    // Create a proposal with multiple versions
    storage.set(
        Some(&admin),
        "governance/proposals",
        "prop-001",
        to_bytes("Initial draft"),
    )?;
    storage.set(
        Some(&admin),
        "governance/proposals",
        "prop-001",
        to_bytes("Revised draft"),
    )?;
    storage.set(
        Some(&admin),
        "governance/proposals",
        "prop-001",
        to_bytes("Final version"),
    )?;

    // Get latest version
    let latest_version =
        from_bytes(&storage.get(Some(&admin), "governance/proposals", "prop-001")?);
    assert_eq!(latest_version, "Final version");

    // List versions
    let versions = storage.list_versions(Some(&admin), "governance/proposals", "prop-001")?;
    assert_eq!(versions.len(), 3);
    assert_eq!(versions[0].version, 1);
    assert_eq!(versions[1].version, 2);
    assert_eq!(versions[2].version, 3);
    assert_eq!(versions[0].created_by, "admin_user");

    // Get version 1
    let (v1_data, v1_info) =
        storage.get_version(Some(&admin), "governance/proposals", "prop-001", 1)?;
    assert_eq!(from_bytes(&v1_data), "Initial draft");
    assert_eq!(v1_info.version, 1);

    // Get version 2
    let (v2_data, v2_info) =
        storage.get_version(Some(&admin), "governance/proposals", "prop-001", 2)?;
    assert_eq!(from_bytes(&v2_data), "Revised draft");
    assert_eq!(v2_info.version, 2);

    // Get version 3
    let (v3_data, v3_info) =
        storage.get_version(Some(&admin), "governance/proposals", "prop-001", 3)?;
    assert_eq!(from_bytes(&v3_data), "Final version");
    assert_eq!(v3_info.version, 3);

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

    let mut storage = setup_test_storage()?;
    let admin = create_admin_auth();

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
