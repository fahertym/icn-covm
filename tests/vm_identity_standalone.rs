// Standalone tests for VM identity operations
// These tests are independent of the storage implementation

use icn_covm::identity::{Credential, DelegationLink, Identity, MemberProfile};
use icn_covm::storage::auth::AuthContext;
use icn_covm::storage::utils;
use icn_covm::vm::Op;
use icn_covm::vm::VM;

fn create_test_identity(id: &str, identity_type: &str) -> Identity {
    let mut identity = Identity::new(id, identity_type);

    // Add a public key (mock)
    let public_key = vec![1, 2, 3, 4, 5];
    identity.public_key = Some(public_key);
    identity.crypto_scheme = Some("ed25519".to_string());

    // Add metadata
    identity.add_metadata("coop_id", "test_coop");

    identity
}

fn setup_identity_context() -> AuthContext {
    // Create an auth context with identities and roles
    let member_id = "member1";
    let mut auth = AuthContext::new(member_id);

    // Add some roles
    auth.add_role("test_coop", "member");
    auth.add_role("coops/test_coop", "member");
    auth.add_role("coops/test_coop/proposals", "proposer");

    // Add identities to registry
    let member_identity = create_test_identity(member_id, "member");
    auth.register_identity(member_identity);
    auth.register_identity(create_test_identity("member2", "member"));
    auth.register_identity(create_test_identity("test_coop", "cooperative"));

    // Add member profiles
    let mut member = MemberProfile::new(create_test_identity("member1", "member"), utils::now());
    member.add_role("member");
    auth.register_member(member);

    // Add credentials
    let mut credential =
        Credential::new("cred1", "membership", "test_coop", "member1", utils::now());
    credential.add_claim("namespace", "test_coop");
    auth.register_credential(credential);

    // Add delegations
    let mut delegation =
        DelegationLink::new("deleg1", "member2", "member1", "voting", utils::now());
    delegation.add_permission("vote");
    auth.register_delegation(delegation);

    auth
}

#[test]
fn test_identity_verification() {
    let auth = setup_identity_context();
    let mut vm = VM::new();
    vm.set_auth_context(auth);
    vm.mock_storage_operations(); // Use mock storage for tests

    // Test verifying a signature (using the mock that always returns true if identity exists)
    let ops = vec![Op::VerifyIdentity {
        identity_id: "member1".to_string(),
        message: "test message".to_string(),
        signature: "mock signature".to_string(),
    }];

    vm.execute(&ops).unwrap();
    assert_eq!(vm.top(), Some(1.0)); // Should be true (1.0)

    // Test with non-existent identity
    let ops = vec![Op::VerifyIdentity {
        identity_id: "nonexistent".to_string(),
        message: "test message".to_string(),
        signature: "mock signature".to_string(),
    }];

    vm.execute(&ops).unwrap();
    assert_eq!(vm.top(), Some(0.0)); // Should be false (0.0)
}

#[test]
fn test_membership_check() {
    let auth = setup_identity_context();
    let mut vm = VM::new();
    vm.set_auth_context(auth);
    vm.mock_storage_operations(); // Use mock storage for tests

    // Test checking membership in a namespace where the member belongs
    let ops = vec![Op::CheckMembership {
        identity_id: "member1".to_string(),
        namespace: "coops/test_coop".to_string(),
    }];

    vm.execute(&ops).unwrap();
    assert_eq!(vm.top(), Some(1.0)); // Should be true (1.0)

    // Test with a namespace where the member doesn't belong
    let ops = vec![Op::CheckMembership {
        identity_id: "member1".to_string(),
        namespace: "coops/other_coop".to_string(),
    }];

    vm.execute(&ops).unwrap();
    assert_eq!(vm.top(), Some(0.0)); // Should be false (0.0)
}

#[test]
fn test_delegation_check() {
    let auth = setup_identity_context();
    let mut vm = VM::new();
    vm.set_auth_context(auth);
    vm.mock_storage_operations(); // Use mock storage for tests

    // Test checking a valid delegation
    let ops = vec![Op::CheckDelegation {
        delegator_id: "member2".to_string(),
        delegate_id: "member1".to_string(),
    }];

    vm.execute(&ops).unwrap();
    assert_eq!(vm.top(), Some(1.0)); // Should be true (1.0)

    // Test with invalid delegation
    let ops = vec![Op::CheckDelegation {
        delegator_id: "member1".to_string(),
        delegate_id: "member2".to_string(),
    }];

    vm.execute(&ops).unwrap();
    assert_eq!(vm.top(), Some(0.0)); // Should be false (0.0)
}

#[test]
fn test_storage_operations_mock() {
    let mut vm = VM::new();
    vm.mock_storage_operations(); // Use mock storage for tests

    // Test storing and loading values
    let ops = vec![
        Op::Push(42.0),
        Op::StoreP("test_key".to_string()),
        Op::LoadP("test_key".to_string()),
    ];

    vm.execute(&ops).unwrap();
    assert_eq!(vm.top(), Some(42.0));
}
