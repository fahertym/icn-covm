use icn_covm::vm::VM;
use icn_covm::vm::types::Op;
use icn_covm::vm::stack::StackOps;
use icn_covm::storage::implementations::in_memory::InMemoryStorage;
use icn_covm::governance::try_handle_governance_op;
use std::fmt::Debug;

// Helper function to create a VM for testing
fn create_test_vm() -> VM<InMemoryStorage> {
    let storage = InMemoryStorage::new();
    VM::with_storage_backend(storage)
}

// ========== RankedVote Tests ==========

#[test]
fn test_ranked_vote_success() {
    let mut vm = create_test_vm();
    let op = Op::RankedVote { candidates: 3, ballots: 2 };

    // Push ballots (2 ballots x 3 candidates)
    vm.stack.push(2.0); // ballot 2, 3rd choice
    vm.stack.push(1.0); // ballot 2, 2nd choice
    vm.stack.push(0.0); // ballot 2, 1st choice

    vm.stack.push(0.0); // ballot 1, 3rd choice
    vm.stack.push(1.0); // ballot 1, 2nd choice
    vm.stack.push(2.0); // ballot 1, 1st choice

    let result = try_handle_governance_op(&mut vm, &op);

    assert!(result.is_ok());
    assert_eq!(vm.top(), Some(1.0)); // Winner should be candidate 1
}

#[test]
fn test_ranked_vote_invalid_input() {
    let mut vm = create_test_vm();
    
    // Test with invalid candidates number
    let op = Op::RankedVote { candidates: 1, ballots: 2 };
    let result = try_handle_governance_op(&mut vm, &op);
    assert!(result.is_err());
    
    // Test with invalid ballots number
    let op = Op::RankedVote { candidates: 3, ballots: 0 };
    let result = try_handle_governance_op(&mut vm, &op);
    assert!(result.is_err());
    
    // Test with stack underflow (not enough values on stack)
    let mut vm = create_test_vm();
    let op = Op::RankedVote { candidates: 3, ballots: 2 };
    vm.stack.push(1.0); // Only one value, need 6 for 2 ballots with 3 candidates each
    let result = try_handle_governance_op(&mut vm, &op);
    assert!(result.is_err());
}

// ========== LiquidDelegate Tests ==========

#[test]
fn test_liquid_delegate_create() {
    let mut vm = create_test_vm();
    let op = Op::LiquidDelegate { 
        from: "alice".to_string(), 
        to: "bob".to_string()
    };
    
    let result = try_handle_governance_op(&mut vm, &op);
    assert!(result.is_ok());
    
    // Check that the delegation count is stored in memory
    let delegations_key = "governance_delegations";
    let count = vm.memory.load(delegations_key);
    assert!(count.is_ok());
    assert_eq!(count.unwrap(), 1.0);
}

#[test]
fn test_liquid_delegate_revoke() {
    let mut vm = create_test_vm();
    
    // First create a delegation
    let create_op = Op::LiquidDelegate { 
        from: "alice".to_string(), 
        to: "bob".to_string()
    };
    
    let result = try_handle_governance_op(&mut vm, &create_op);
    assert!(result.is_ok());
    
    // Then revoke it
    let revoke_op = Op::LiquidDelegate { 
        from: "alice".to_string(), 
        to: "".to_string()
    };
    
    let result = try_handle_governance_op(&mut vm, &revoke_op);
    assert!(result.is_ok());
    
    // Check that the delegation count is reduced
    let delegations_key = "governance_delegations";
    let count = vm.memory.load(delegations_key);
    assert!(count.is_ok());
    assert_eq!(count.unwrap(), 0.0);
}

#[test]
fn test_liquid_delegate_cycle_prevention() {
    let mut vm = create_test_vm();
    
    // Create chain: A -> B
    let op1 = Op::LiquidDelegate { 
        from: "alice".to_string(), 
        to: "bob".to_string()
    };
    
    let result = try_handle_governance_op(&mut vm, &op1);
    assert!(result.is_ok());
    
    // Create chain: B -> C
    let op2 = Op::LiquidDelegate { 
        from: "bob".to_string(), 
        to: "charlie".to_string()
    };
    
    let result = try_handle_governance_op(&mut vm, &op2);
    assert!(result.is_ok());
    
    // Try to create cycle: C -> A (should fail)
    let op3 = Op::LiquidDelegate { 
        from: "charlie".to_string(), 
        to: "alice".to_string()
    };
    
    let result = try_handle_governance_op(&mut vm, &op3);
    assert!(result.is_err());
}

#[test]
fn test_liquid_delegate_empty_from() {
    let mut vm = create_test_vm();
    let op = Op::LiquidDelegate { 
        from: "".to_string(), 
        to: "bob".to_string()
    };
    
    let result = try_handle_governance_op(&mut vm, &op);
    assert!(result.is_err());
}

// ========== QuorumThreshold Tests ==========

#[test]
fn test_quorum_threshold_met() {
    let mut vm = create_test_vm();
    let op = Op::QuorumThreshold(0.5); // 50% threshold
    
    // Push values: total_possible = 100, votes_cast = 60
    vm.stack.push(100.0); // total_possible
    vm.stack.push(60.0);  // votes_cast
    
    let result = try_handle_governance_op(&mut vm, &op);
    
    assert!(result.is_ok());
    assert_eq!(vm.top(), Some(0.0)); // 0.0 means threshold met (truthy in VM)
}

#[test]
fn test_quorum_threshold_not_met() {
    let mut vm = create_test_vm();
    let op = Op::QuorumThreshold(0.5); // 50% threshold
    
    // Push values: total_possible = 100, votes_cast = 40
    vm.stack.push(100.0); // total_possible
    vm.stack.push(40.0);  // votes_cast
    
    let result = try_handle_governance_op(&mut vm, &op);
    
    assert!(result.is_ok());
    assert_eq!(vm.top(), Some(1.0)); // 1.0 means threshold not met (falsey in VM)
}

#[test]
fn test_quorum_threshold_invalid() {
    let mut vm = create_test_vm();
    
    // Test with negative threshold
    let op = Op::QuorumThreshold(-0.1);
    let result = try_handle_governance_op(&mut vm, &op);
    assert!(result.is_err());
    
    // Test with threshold > 1.0
    let op = Op::QuorumThreshold(1.1);
    let result = try_handle_governance_op(&mut vm, &op);
    assert!(result.is_err());
    
    // Test with invalid total_possible
    let mut vm = create_test_vm();
    let op = Op::QuorumThreshold(0.5);
    vm.stack.push(0.0);  // total_possible = 0 (invalid)
    vm.stack.push(1.0);  // votes_cast
    let result = try_handle_governance_op(&mut vm, &op);
    assert!(result.is_err());
}

// ========== VoteThreshold Tests ==========

#[test]
fn test_vote_threshold_met() {
    let mut vm = create_test_vm();
    let op = Op::VoteThreshold(50.0); // Need 50 votes to pass
    
    vm.stack.push(60.0); // total_votes
    
    let result = try_handle_governance_op(&mut vm, &op);
    
    assert!(result.is_ok());
    assert_eq!(vm.top(), Some(0.0)); // 0.0 means threshold met (truthy in VM)
}

#[test]
fn test_vote_threshold_not_met() {
    let mut vm = create_test_vm();
    let op = Op::VoteThreshold(50.0); // Need 50 votes to pass
    
    vm.stack.push(40.0); // total_votes
    
    let result = try_handle_governance_op(&mut vm, &op);
    
    assert!(result.is_ok());
    assert_eq!(vm.top(), Some(1.0)); // 1.0 means threshold not met (falsey in VM)
}

#[test]
fn test_vote_threshold_invalid() {
    let mut vm = create_test_vm();
    
    // Test with negative threshold
    let op = Op::VoteThreshold(-1.0);
    let result = try_handle_governance_op(&mut vm, &op);
    assert!(result.is_err());
    
    // Test with stack underflow
    let mut vm = create_test_vm();
    let op = Op::VoteThreshold(50.0);
    // Not pushing any value to the stack
    let result = try_handle_governance_op(&mut vm, &op);
    assert!(result.is_err());
} 