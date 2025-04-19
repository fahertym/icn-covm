// Standalone tests for VM identity operations
// These tests are independent of the storage implementation

use icn_covm::identity::{Identity, Profile};
use icn_covm::storage::auth::AuthContext;
use icn_covm::storage::implementations::in_memory::InMemoryStorage;
use icn_covm::storage::traits::StorageExtensions;
use icn_covm::storage::utils;
use icn_covm::vm::Op;
use icn_covm::vm::VM;
use std::collections::HashMap;

fn create_test_identity(id: &str, identity_type: &str) -> Identity {
    // Create an identity with the new constructor
    let public_username = format!("{}_user", id);
    let identity = Identity::new(
        public_username,
        None, // no full name
        identity_type.to_string(),
        None, // no other profile fields
    )
    .expect("Failed to create test identity");

    identity
}

fn setup_identity_context() -> AuthContext {
    // Create an auth context with identities and roles
    let member_id = "member1";
    let member_identity = create_test_identity(member_id, "member");
    let mut auth = AuthContext::new(&member_identity.did);

    // Add some roles
    auth.add_role("test_coop", "member");
    auth.add_role("coops/test_coop", "member");
    auth.add_role("coops/test_coop/proposals", "proposer");

    // Add identities to registry
    auth.register_identity(member_identity.clone());
    auth.register_identity(create_test_identity("member2", "member"));
    auth.register_identity(create_test_identity("test_coop", "cooperative"));

    // Add memberships
    auth.add_membership(&member_identity.did, "coops/test_coop");

    // Add delegations (member2 delegates to member1)
    let member2_identity = create_test_identity("member2", "member");
    auth.add_delegation(&member2_identity.did, &member_identity.did, "voting");

    auth
}

#[test]
fn test_identity_verification() {
    let auth = setup_identity_context();
    let storage = InMemoryStorage::new();
    let mut vm = VM::with_storage_backend(storage);
    vm.set_auth_context(auth);
    vm.mock_storage_operations(); // Use mock storage for tests

    // Test verifying a signature (using the mock that always returns true if identity exists)
    let ops = vec![Op::VerifyIdentity {
        identity_id: "member1_user".to_string(),
        message: "test message".to_string(),
        signature: "mock signature".to_string(),
    }];

    vm.execute(&ops).unwrap();
    assert_eq!(vm.top(), Some(1.0)); // Should be true (1.0) with mock

    // Test with non-existent identity
    let ops = vec![Op::VerifyIdentity {
        identity_id: "nonexistent".to_string(),
        message: "test message".to_string(),
        signature: "mock signature".to_string(),
    }];

    vm.execute(&ops).unwrap();
    assert_eq!(vm.top(), Some(0.0)); // Should be false (0.0) for unknown identity
}

#[test]
fn test_membership_check() {
    let auth = setup_identity_context();
    let storage = InMemoryStorage::new();
    let mut vm = VM::with_storage_backend(storage);
    vm.set_auth_context(auth);
    vm.mock_storage_operations(); // Use mock storage for tests

    // Test checking membership in a namespace where the member belongs
    let ops = vec![Op::CheckMembership {
        identity_id: "member1_user".to_string(),
        namespace: "coops/test_coop".to_string(),
    }];

    vm.execute(&ops).unwrap();
    assert_eq!(vm.top(), Some(1.0)); // Should be true (1.0)

    // Test with a namespace where the member doesn't belong
    let ops = vec![Op::CheckMembership {
        identity_id: "member1_user".to_string(),
        namespace: "coops/other_coop".to_string(),
    }];

    vm.execute(&ops).unwrap();
    assert_eq!(vm.top(), Some(0.0)); // Should be false (0.0)
}

#[test]
fn test_delegation_check() {
    let auth = setup_identity_context();
    let storage = InMemoryStorage::new();
    let mut vm = VM::with_storage_backend(storage);
    vm.set_auth_context(auth);
    vm.mock_storage_operations(); // Use mock storage for tests

    // Test checking a valid delegation
    let ops = vec![Op::CheckDelegation {
        delegator_id: "member2_user".to_string(),
        delegate_id: "member1_user".to_string(),
    }];

    vm.execute(&ops).unwrap();
    assert_eq!(vm.top(), Some(1.0)); // Should be true (1.0)

    // Test with invalid delegation
    let ops = vec![Op::CheckDelegation {
        delegator_id: "member1_user".to_string(),
        delegate_id: "member2_user".to_string(),
    }];

    vm.execute(&ops).unwrap();
    assert_eq!(vm.top(), Some(0.0)); // Should be false (0.0)
}

#[test]
fn test_storage_operations_mock() {
    // Create a VM with InMemoryStorage
    let storage = InMemoryStorage::new();
    let mut vm = VM::with_storage_backend(storage);
    vm.mock_storage_operations();

    // Set up auth context
    let auth = setup_identity_context();
    vm.set_auth_context(auth);
    vm.set_namespace("default");

    // Test with membership check which doesn't need actual storage
    let ops = vec![Op::CheckMembership {
        identity_id: "member1_user".to_string(),
        namespace: "coops/test_coop".to_string(),
    }];

    vm.execute(&ops).unwrap();
    assert_eq!(vm.top(), Some(1.0)); // Should be true (1.0) for membership

    // Test with false membership
    let ops = vec![Op::CheckMembership {
        identity_id: "member1_user".to_string(),
        namespace: "coops/unknown_coop".to_string(),
    }];

    vm.execute(&ops).unwrap();
    assert_eq!(vm.top(), Some(0.0)); // Should be false (0.0) for non-membership
}

#[test]
fn test_identity_operations_with_storage() {
    // Create a VM with InMemoryStorage
    let storage = InMemoryStorage::new();
    let mut vm = VM::with_storage_backend(storage);
    vm.mock_storage_operations();

    // Set up auth context
    let auth = setup_identity_context();
    vm.set_auth_context(auth);

    // Save an identity to storage
    let test_identity = create_test_identity("test_store", "member");
    let auth_context = setup_identity_context();

    vm.with_storage_mut(|storage| {
        storage.set_json(
            Some(&auth_context),
            "identity",
            &format!("identities/{}", test_identity.did),
            &test_identity,
        )
    })
    .unwrap();

    // Retrieve the identity using StorageExtensions trait
    let retrieved_identity = vm
        .with_storage(|storage| storage.get_identity(&test_identity.did))
        .unwrap();

    assert_eq!(retrieved_identity.did, test_identity.did);
    assert_eq!(
        retrieved_identity.identity_type,
        test_identity.identity_type
    );

    // Test with identity verification which doesn't need actual storage
    let ops = vec![Op::VerifyIdentity {
        identity_id: "member1_user".to_string(),
        message: "test message".to_string(),
        signature: "mock signature".to_string(),
    }];

    vm.execute(&ops).unwrap();
    assert_eq!(vm.top(), Some(1.0)); // Should be true (1.0) for a known identity

    // Test with non-existent identity
    let ops = vec![Op::VerifyIdentity {
        identity_id: "unknown_member".to_string(),
        message: "test message".to_string(),
        signature: "mock signature".to_string(),
    }];

    vm.execute(&ops).unwrap();
    assert_eq!(vm.top(), Some(0.0)); // Should be false (0.0) for unknown identity
}
