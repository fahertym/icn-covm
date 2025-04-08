use icn_covm::storage::auth::AuthContext;
use icn_covm::storage::implementations::in_memory::InMemoryStorage;
use icn_covm::storage::traits::StorageBackend;
use icn_covm::vm::{VM, Op, VMError};

fn setup_vm() -> VM {
    let mut storage = InMemoryStorage::new();
    
    // Create auth context with appropriate permissions
    let mut auth_context = AuthContext::new("test_user");
    
    // Add roles with permissions for the test namespace
    auth_context.add_role("test_namespace", "admin");
    auth_context.add_role("global", "admin"); // Global admin bypasses all permission checks
    
    // Create a storage account for the test user
    storage.create_account(Some(&auth_context), "test_user", 1_000_000).unwrap();
    
    // Create the test namespace
    storage.create_namespace(Some(&auth_context), "test_namespace", 1_000_000, None).unwrap();
    
    // Create a resources directory that will hold all resources
    storage.set(Some(&auth_context), "test_namespace", "resources", "{}".as_bytes().to_vec()).unwrap();
    
    // Create VM with in-memory storage and set auth context
    let mut vm = VM::new();
    vm.set_auth_context(auth_context);
    vm.set_namespace("test_namespace");
    
    // Set storage backend
    vm.storage_backend = Some(Box::new(storage));
    
    vm
}

// Helper functions to run economic operations
fn create_resource(vm: &mut VM, resource_id: &str) -> Result<(), VMError> {
    let op = Op::CreateResource(resource_id.to_string());
    println!("Creating resource: {}", resource_id);
    
    // Check resources directory before operation
    {
        let storage = vm.storage_backend.as_ref().unwrap();
        let auth = vm.auth_context.as_ref();
        let ns = &vm.namespace;
        
        let resources_exists = storage.contains(auth, ns, "resources").unwrap();
        println!("Resources directory exists: {}", resources_exists);
    }
    
    // Execute the create resource operation
    let result = vm.execute(&[op]);
    if result.is_err() {
        println!("Create resource error: {:?}", result);
    } else {
        println!("Resource created successfully!");
        
        // Verify the resource was created in a new scope to avoid borrow issues
        let storage = vm.storage_backend.as_ref().unwrap();
        let auth = vm.auth_context.as_ref();
        let ns = &vm.namespace;
        
        let resource_path = format!("resources/{}", resource_id);
        let resource_exists = storage.contains(auth, ns, &resource_path).unwrap();
        println!("Resource now exists: {}", resource_exists);
        
        let balances_path = format!("resources/{}/balances", resource_id);
        let balances_exists = storage.contains(auth, ns, &balances_path).unwrap();
        println!("Balances now exist: {}", balances_exists);
    }
    
    result
}

fn mint_tokens(vm: &mut VM, resource: &str, account: &str, amount: f64, reason: Option<String>) -> Result<(), VMError> {
    let op = Op::Mint {
        resource: resource.to_string(),
        account: account.to_string(),
        amount,
        reason,
    };
    let result = vm.execute(&[op]);
    if result.is_err() {
        println!("Mint tokens error: {:?}", result);
    }
    result
}

fn transfer_tokens(vm: &mut VM, resource: &str, from: &str, to: &str, amount: f64, reason: Option<String>) -> Result<(), VMError> {
    let op = Op::Transfer {
        resource: resource.to_string(),
        from: from.to_string(),
        to: to.to_string(),
        amount,
        reason,
    };
    let result = vm.execute(&[op]);
    if result.is_err() {
        println!("Transfer tokens error: {:?}", result);
    }
    result
}

fn burn_tokens(vm: &mut VM, resource: &str, account: &str, amount: f64, reason: Option<String>) -> Result<(), VMError> {
    let op = Op::Burn {
        resource: resource.to_string(),
        account: account.to_string(),
        amount,
        reason,
    };
    let result = vm.execute(&[op]);
    if result.is_err() {
        println!("Burn tokens error: {:?}", result);
    }
    result
}

fn get_balance(vm: &mut VM, resource: &str, account: &str) -> Result<f64, VMError> {
    let op = Op::Balance {
        resource: resource.to_string(),
        account: account.to_string(),
    };
    let result = vm.execute(&[op]);
    if result.is_err() {
        println!("Get balance error: {:?}", result);
        return Err(result.err().unwrap());
    }
    
    // Balance should be pushed to stack
    Ok(vm.stack.pop().unwrap_or(0.0))
}

#[test]
fn test_create_resource() {
    let mut vm = setup_vm();
    
    // Execute the create resource operation via execute with proper Op
    let result = create_resource(&mut vm, "test_coin");
    assert!(result.is_ok(), "Failed to create resource: {:?}", result);
    
    // Verify the resource was created
    let storage = vm.storage_backend.as_mut().unwrap();
    let auth = vm.auth_context.clone();
    let ns = vm.namespace.clone();
    
    // The resource should be stored under "resources/test_coin" in the namespace
    let resource_exists = storage.contains(auth.as_ref(), &ns, "resources/test_coin").unwrap();
    assert!(resource_exists, "Resource not found in storage");
    
    // Check that balances were initialized
    let balances_exists = storage.contains(auth.as_ref(), &ns, "resources/test_coin/balances").unwrap();
    assert!(balances_exists, "Resource balances not found in storage");
}

#[test]
fn test_mint_and_burn() {
    let mut vm = setup_vm();
    
    // First create a resource
    let result = create_resource(&mut vm, "test_coin");
    assert!(result.is_ok(), "Failed to create resource: {:?}", result);
    
    // Test minting tokens
    let mint_result = mint_tokens(&mut vm, "test_coin", "alice", 100.0, None);
    assert!(mint_result.is_ok(), "Failed to mint tokens: {:?}", mint_result);
    
    // Verify balance was updated
    let balance = get_balance(&mut vm, "test_coin", "alice").unwrap();
    assert_eq!(balance, 100.0, "Balance incorrect after minting");
    
    // Test minting more tokens
    let mint_result = mint_tokens(&mut vm, "test_coin", "alice", 50.0, Some("Bonus reward".to_string()));
    assert!(mint_result.is_ok(), "Failed to mint additional tokens: {:?}", mint_result);
    
    // Verify balance was updated
    let balance = get_balance(&mut vm, "test_coin", "alice").unwrap();
    assert_eq!(balance, 150.0, "Balance incorrect after second minting");
    
    // Test burning tokens
    let burn_result = burn_tokens(&mut vm, "test_coin", "alice", 30.0, None);
    assert!(burn_result.is_ok(), "Failed to burn tokens: {:?}", burn_result);
    
    // Verify balance was updated
    let balance = get_balance(&mut vm, "test_coin", "alice").unwrap();
    assert_eq!(balance, 120.0, "Balance incorrect after burning");
    
    // Test burning more tokens than available
    let burn_result = burn_tokens(&mut vm, "test_coin", "alice", 200.0, None);
    assert!(burn_result.is_err(), "Should fail when burning more than available");
}

#[test]
fn test_transfer() {
    let mut vm = setup_vm();
    
    // First create a resource
    let result = create_resource(&mut vm, "test_coin");
    assert!(result.is_ok(), "Failed to create resource: {:?}", result);
    
    // Mint tokens to alice
    let mint_result = mint_tokens(&mut vm, "test_coin", "alice", 100.0, None);
    assert!(mint_result.is_ok(), "Failed to mint tokens to alice: {:?}", mint_result);
    
    // Verify alice's balance
    let balance = get_balance(&mut vm, "test_coin", "alice").unwrap();
    assert_eq!(balance, 100.0, "Alice's initial balance is incorrect");
    
    // Transfer tokens from alice to bob
    let transfer_result = transfer_tokens(&mut vm, "test_coin", "alice", "bob", 40.0, None);
    assert!(transfer_result.is_ok(), "Failed to transfer tokens: {:?}", transfer_result);
    
    // Verify balances were updated
    let alice_balance = get_balance(&mut vm, "test_coin", "alice").unwrap();
    assert_eq!(alice_balance, 60.0, "Alice's balance incorrect after transfer");
    
    let bob_balance = get_balance(&mut vm, "test_coin", "bob").unwrap();
    assert_eq!(bob_balance, 40.0, "Bob's balance incorrect after transfer");
    
    // Test transferring more than available
    let transfer_result = transfer_tokens(&mut vm, "test_coin", "alice", "bob", 100.0, None);
    assert!(transfer_result.is_err(), "Should fail when transferring more than available");
}

#[test]
fn test_integration_economic_ops() {
    let mut vm = setup_vm();
    
    // Create the resource
    let result = create_resource(&mut vm, "community_token");
    assert!(result.is_ok(), "Failed to create community_token: {:?}", result);
    
    // Mint initial tokens to founders
    let mint_result = mint_tokens(&mut vm, "community_token", "founder1", 1000.0, None);
    assert!(mint_result.is_ok(), "Failed to mint to founder1: {:?}", mint_result);
    
    let mint_result = mint_tokens(&mut vm, "community_token", "founder2", 1000.0, None);
    assert!(mint_result.is_ok(), "Failed to mint to founder2: {:?}", mint_result);
    
    // Create treasury account and transfer initial funding
    let transfer_result = transfer_tokens(&mut vm, "community_token", "founder1", "treasury", 500.0, None);
    assert!(transfer_result.is_ok(), "Failed to fund treasury: {:?}", transfer_result);
    
    let transfer_result = transfer_tokens(&mut vm, "community_token", "founder2", "treasury", 500.0, None);
    assert!(transfer_result.is_ok(), "Failed to fund treasury from founder2: {:?}", transfer_result);
    
    // Create community team account and fund from treasury
    let transfer_result = transfer_tokens(&mut vm, "community_token", "treasury", "team", 450.0, Some("Team allocation".to_string()));
    assert!(transfer_result.is_ok(), "Failed to fund team: {:?}", transfer_result);
    
    // Check final balances
    let founder1_balance = get_balance(&mut vm, "community_token", "founder1").unwrap();
    assert_eq!(founder1_balance, 500.0);
    
    let founder2_balance = get_balance(&mut vm, "community_token", "founder2").unwrap();
    assert_eq!(founder2_balance, 500.0);
    
    let treasury_balance = get_balance(&mut vm, "community_token", "treasury").unwrap();
    assert_eq!(treasury_balance, 550.0);
    
    let team_balance = get_balance(&mut vm, "community_token", "team").unwrap();
    assert_eq!(team_balance, 450.0);
}

#[test]
fn test_multiple_resources() {
    let mut vm = setup_vm();
    
    // Create first resource
    let result = create_resource(&mut vm, "test_coin");
    assert!(result.is_ok(), "Failed to create test_coin: {:?}", result);
    
    // Create second resource
    let result = create_resource(&mut vm, "community_token");
    assert!(result.is_ok(), "Failed to create community_token: {:?}", result);
    
    // Mint tokens for first resource
    let mint_result = mint_tokens(&mut vm, "test_coin", "alice", 100.0, None);
    assert!(mint_result.is_ok(), "Failed to mint test_coin: {:?}", mint_result);
    
    // Mint tokens for second resource
    let mint_result = mint_tokens(&mut vm, "community_token", "alice", 200.0, None);
    assert!(mint_result.is_ok(), "Failed to mint community_token: {:?}", mint_result);
    
    // Verify balances
    let test_coin_balance = get_balance(&mut vm, "test_coin", "alice").unwrap();
    assert_eq!(test_coin_balance, 100.0, "test_coin balance incorrect");
    
    let community_token_balance = get_balance(&mut vm, "community_token", "alice").unwrap();
    assert_eq!(community_token_balance, 200.0, "community_token balance incorrect");
    
    // Transfer first resource
    let transfer_result = transfer_tokens(&mut vm, "test_coin", "alice", "bob", 40.0, None);
    assert!(transfer_result.is_ok(), "Failed to transfer test_coin: {:?}", transfer_result);
    
    // Transfer second resource
    let transfer_result = transfer_tokens(&mut vm, "community_token", "alice", "bob", 50.0, None);
    assert!(transfer_result.is_ok(), "Failed to transfer community_token: {:?}", transfer_result);
    
    // Verify balances after transfers
    let alice_test_coin = get_balance(&mut vm, "test_coin", "alice").unwrap();
    assert_eq!(alice_test_coin, 60.0, "Alice's test_coin balance incorrect after transfer");
    
    let bob_test_coin = get_balance(&mut vm, "test_coin", "bob").unwrap();
    assert_eq!(bob_test_coin, 40.0, "Bob's test_coin balance incorrect after transfer");
    
    let alice_community_token = get_balance(&mut vm, "community_token", "alice").unwrap();
    assert_eq!(alice_community_token, 150.0, "Alice's community_token balance incorrect after transfer");
    
    let bob_community_token = get_balance(&mut vm, "community_token", "bob").unwrap();
    assert_eq!(bob_community_token, 50.0, "Bob's community_token balance incorrect after transfer");
} 