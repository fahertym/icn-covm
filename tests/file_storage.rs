use icn_covm::storage::auth::AuthContext;
use icn_covm::storage::errors::StorageResult;
use icn_covm::storage::implementations::file_storage::FileStorage;
use icn_covm::storage::traits::StorageBackend;
use std::fs;
use std::path::PathBuf;

mod test_helpers;
use test_helpers::{create_admin_auth, from_bytes, to_bytes};

fn get_test_dir(test_name: &str) -> PathBuf {
    let test_dir = PathBuf::from(format!("target/test/file_storage/{}", test_name));

    // Clean up test directory if it exists
    if test_dir.exists() {
        match fs::remove_dir_all(&test_dir) {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Warning: Failed to clean up test directory: {}", e);
                // Continue even if we couldn't clean up
            }
        }
    }

    // Create the test directory
    fs::create_dir_all(&test_dir).expect("Failed to create test directory");

    test_dir
}

#[test]
fn test_file_storage_basic() -> StorageResult<()> {
    let test_dir = get_test_dir("basic");
    let mut storage = FileStorage::new(test_dir)?;
    let admin = create_admin_auth();

    // Create a namespace
    storage.create_namespace(Some(&admin), "test", 1024 * 1024, None)?;

    // Create a user account
    storage.create_account(Some(&admin), "admin_user", 1024 * 1024)?;

    // Set a value
    storage.set(Some(&admin), "test", "key1", to_bytes("Hello, world!"))?;

    // Get the value back
    let value = storage.get(Some(&admin), "test", "key1")?;
    assert_eq!(from_bytes(&value), "Hello, world!");

    // Update the value
    storage.set(Some(&admin), "test", "key1", to_bytes("Updated value"))?;
    let value = storage.get(Some(&admin), "test", "key1")?;
    assert_eq!(from_bytes(&value), "Updated value");

    // List versions
    let versions = storage.list_versions(Some(&admin), "test", "key1")?;
    assert_eq!(versions.len(), 2);
    assert_eq!(versions[0].version, 1);
    assert_eq!(versions[1].version, 2);

    // Get specific version
    let (v1_data, v1_info) = storage.get_version(Some(&admin), "test", "key1", 1)?;
    assert_eq!(from_bytes(&v1_data), "Hello, world!");
    assert_eq!(v1_info.version, 1);

    // List keys
    let keys = storage.list_keys(Some(&admin), "test", None)?;
    assert_eq!(keys.len(), 1);
    assert!(keys.contains(&"key1".to_string()));

    // Delete the key
    storage.delete(Some(&admin), "test", "key1")?;
    assert!(storage.get(Some(&admin), "test", "key1").is_err());

    Ok(())
}

#[test]
fn test_file_storage_namespaces() -> StorageResult<()> {
    let test_dir = get_test_dir("namespaces");
    let mut storage = FileStorage::new(test_dir)?;
    let admin = create_admin_auth();

    // Create account and root namespace
    storage.create_account(Some(&admin), "admin_user", 1024 * 1024)?;
    storage.create_namespace(Some(&admin), "governance", 1024 * 1024, None)?;

    // Create child namespaces
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

    // List namespaces
    let namespaces = storage.list_namespaces(Some(&admin), "governance")?;
    assert_eq!(namespaces.len(), 2);

    // Store data in different namespaces
    storage.set(
        Some(&admin),
        "governance",
        "config",
        to_bytes("Global config"),
    )?;
    storage.set(
        Some(&admin),
        "governance/proposals",
        "prop-001",
        to_bytes("Proposal 1"),
    )?;
    storage.set(
        Some(&admin),
        "governance/votes",
        "prop-001/admin",
        to_bytes("1"),
    )?;

    // Verify data in each namespace
    assert_eq!(
        from_bytes(&storage.get(Some(&admin), "governance", "config")?),
        "Global config"
    );
    assert_eq!(
        from_bytes(&storage.get(Some(&admin), "governance/proposals", "prop-001")?),
        "Proposal 1"
    );
    assert_eq!(
        from_bytes(&storage.get(Some(&admin), "governance/votes", "prop-001/admin")?),
        "1"
    );

    Ok(())
}

#[test]
fn test_file_storage_transactions() -> StorageResult<()> {
    let test_dir = get_test_dir("transactions");
    let mut storage = FileStorage::new(test_dir)?;
    let admin = create_admin_auth();

    // Setup namespace and account
    storage.create_account(Some(&admin), "admin_user", 1024 * 1024)?;
    storage.create_namespace(Some(&admin), "test", 1024 * 1024, None)?;

    // Initial value
    storage.set(Some(&admin), "test", "key1", to_bytes("Initial value"))?;

    // Begin transaction
    storage.begin_transaction()?;

    // Make some changes in the transaction
    storage.set(
        Some(&admin),
        "test",
        "key1",
        to_bytes("Updated in transaction"),
    )?;
    storage.set(
        Some(&admin),
        "test",
        "key2",
        to_bytes("New key in transaction"),
    )?;

    // Values should be visible during the transaction
    assert_eq!(
        from_bytes(&storage.get(Some(&admin), "test", "key1")?),
        "Updated in transaction"
    );
    assert_eq!(
        from_bytes(&storage.get(Some(&admin), "test", "key2")?),
        "New key in transaction"
    );

    // Rollback the transaction
    storage.rollback_transaction()?;

    // Values should be restored to pre-transaction state
    assert_eq!(
        from_bytes(&storage.get(Some(&admin), "test", "key1")?),
        "Initial value"
    );
    assert!(storage.get(Some(&admin), "test", "key2").is_err());

    // New transaction
    storage.begin_transaction()?;

    // Make changes again
    storage.set(Some(&admin), "test", "key1", to_bytes("Final value"))?;
    storage.set(Some(&admin), "test", "key2", to_bytes("Another new key"))?;

    // Commit the transaction
    storage.commit_transaction()?;

    // Values should persist after commit
    assert_eq!(
        from_bytes(&storage.get(Some(&admin), "test", "key1")?),
        "Final value"
    );
    assert_eq!(
        from_bytes(&storage.get(Some(&admin), "test", "key2")?),
        "Another new key"
    );

    Ok(())
}

#[test]
fn test_file_storage_permissions() -> StorageResult<()> {
    let test_dir = get_test_dir("permissions");
    let mut storage = FileStorage::new(test_dir)?;
    let admin = create_admin_auth();

    // Create accounts and namespace
    storage.create_account(Some(&admin), "admin_user", 1024 * 1024)?;
    storage.create_account(Some(&admin), "reader_user", 1024 * 1024)?;
    storage.create_account(Some(&admin), "writer_user", 1024 * 1024)?;
    storage.create_namespace(Some(&admin), "test", 1024 * 1024, None)?;

    // Set initial value as admin
    storage.set(Some(&admin), "test", "key1", to_bytes("Test value"))?;

    // Create reader user without write permissions
    let mut reader = AuthContext::new("reader_user");
    reader.add_role("test", "reader");

    // Create writer user with write permissions
    let mut writer = AuthContext::new("writer_user");
    writer.add_role("test", "writer");

    // Reader should be able to read
    assert_eq!(
        from_bytes(&storage.get(Some(&reader), "test", "key1")?),
        "Test value"
    );

    // Reader should not be able to write
    let write_result = storage.set(
        Some(&reader),
        "test",
        "key1",
        to_bytes("Modified by reader"),
    );
    assert!(write_result.is_err());

    // Writer should be able to read and write
    assert_eq!(
        from_bytes(&storage.get(Some(&writer), "test", "key1")?),
        "Test value"
    );
    storage.set(
        Some(&writer),
        "test",
        "key1",
        to_bytes("Modified by writer"),
    )?;
    assert_eq!(
        from_bytes(&storage.get(Some(&writer), "test", "key1")?),
        "Modified by writer"
    );

    Ok(())
}
