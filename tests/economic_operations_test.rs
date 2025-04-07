use icn_covm::storage::auth::AuthContext;
use icn_covm::storage::implementations::in_memory::InMemoryStorage;
use icn_covm::storage::traits::StorageBackend;
use icn_covm::vm::VM;
use std::fs;

fn setup_vm() -> VM {
    let mut storage = InMemoryStorage::new();
    let auth_context = AuthContext::new("test_user", Some("test_namespace"));
    
    // Create the namespace for testing
    storage.create_namespace(
        &auth_context,
        "test_namespace",
        Some("Test Namespace for Economic Operations"),
    ).unwrap();
    
    // Grant the test user admin role in the namespace
    storage.grant_role(
        &AuthContext::system(),
        "test_namespace",
        "test_user",
        "admin",
    ).unwrap();
    
    let mut vm = VM::new();
    vm.with_auth_context(auth_context)
      .with_storage(Box::new(storage))
      .with_namespace("test_namespace".to_string());
    
    vm
}

#[test]
fn test_create_resource() {
    let mut vm = setup_vm();
    
    // Resource metadata in memory
    vm.memory.insert("resource_metadata".to_string(), 1.0);
    vm.memory.insert("_str_1".to_string(), 1.0); // Using the string storage convention
    
    // The actual metadata string
    let resource_metadata = r#"{
        "id": "test_coin",
        "name": "Test Coin",
        "description": "A test cryptocurrency for unit testing",
        "resource_type": "currency",
        "issuer_namespace": "test_namespace",
        "created_at": 1618531200000,
        "metadata": {
            "symbol": "TSTC",
            "decimals": "2"
        },
        "transferable": true,
        "divisible": true
    }"#;
    
    // Insert the metadata string
    vm.output = resource_metadata.to_string();
    
    // Execute the create resource operation
    let result = vm.execute_create_resource("test_coin");
    assert!(result.is_ok(), "Failed to create resource: {:?}", result);
    
    // Verify the resource was created
    let storage = vm.get_storage_mut();
    let auth = vm.auth_context.clone();
    let ns = vm.namespace.clone();
    
    let resource_exists = storage.contains(&auth, &ns, "resources/test_coin").unwrap();
    assert!(resource_exists, "Resource not found in storage");
    
    // Check that balances were initialized
    let balances_exists = storage.contains(&auth, &ns, "resources/test_coin/balances").unwrap();
    assert!(balances_exists, "Resource balances not found in storage");
}

#[test]
fn test_mint_and_burn() {
    let mut vm = setup_vm();
    
    // Set up a resource for testing
    let resource_key = "resources/test_coin";
    let balances_key = "resources/test_coin/balances";
    
    // Create resource and empty balances
    let storage = vm.get_storage_mut();
    let auth = vm.auth_context.clone();
    let ns = vm.namespace.clone();
    
    storage.set(&auth, &ns, resource_key, r#"{"id":"test_coin"}"#.as_bytes().to_vec()).unwrap();
    storage.set(&auth, &ns, balances_key, "{}".as_bytes().to_vec()).unwrap();
    
    // Test minting
    let mint_result = vm.execute_mint("test_coin", "alice", 100.0, &None);
    assert!(mint_result.is_ok(), "Failed to mint tokens: {:?}", mint_result);
    
    // Verify balance was updated
    let result = vm.execute_balance("test_coin", "alice");
    assert!(result.is_ok(), "Failed to get balance: {:?}", result);
    assert_eq!(vm.stack.pop().unwrap(), 100.0, "Balance incorrect after minting");
    
    // Test minting more tokens
    let mint_result = vm.execute_mint("test_coin", "alice", 50.0, &Some("Bonus reward".to_string()));
    assert!(mint_result.is_ok(), "Failed to mint additional tokens: {:?}", mint_result);
    
    // Verify balance was updated
    let result = vm.execute_balance("test_coin", "alice");
    assert!(result.is_ok(), "Failed to get balance: {:?}", result);
    assert_eq!(vm.stack.pop().unwrap(), 150.0, "Balance incorrect after second minting");
    
    // Test burning tokens
    let burn_result = vm.execute_burn("test_coin", "alice", 30.0, &None);
    assert!(burn_result.is_ok(), "Failed to burn tokens: {:?}", burn_result);
    
    // Verify balance was updated
    let result = vm.execute_balance("test_coin", "alice");
    assert!(result.is_ok(), "Failed to get balance: {:?}", result);
    assert_eq!(vm.stack.pop().unwrap(), 120.0, "Balance incorrect after burning");
    
    // Test burning more tokens than available
    let burn_result = vm.execute_burn("test_coin", "alice", 200.0, &None);
    assert!(burn_result.is_err(), "Should fail when burning more than available");
}

#[test]
fn test_transfer() {
    let mut vm = setup_vm();
    
    // Set up a resource for testing
    let resource_key = "resources/test_coin";
    let balances_key = "resources/test_coin/balances";
    
    // Create resource and empty balances
    let storage = vm.get_storage_mut();
    let auth = vm.auth_context.clone();
    let ns = vm.namespace.clone();
    
    storage.set(&auth, &ns, resource_key, r#"{"id":"test_coin"}"#.as_bytes().to_vec()).unwrap();
    storage.set(&auth, &ns, balances_key, "{}".as_bytes().to_vec()).unwrap();
    
    // Mint initial tokens to alice
    let mint_result = vm.execute_mint("test_coin", "alice", 100.0, &None);
    assert!(mint_result.is_ok(), "Failed to mint tokens: {:?}", mint_result);
    
    // Test transfer to bob
    let transfer_result = vm.execute_transfer("test_coin", "alice", "bob", 40.0, &None);
    assert!(transfer_result.is_ok(), "Failed to transfer tokens: {:?}", transfer_result);
    
    // Verify balances were updated
    let result = vm.execute_balance("test_coin", "alice");
    assert!(result.is_ok(), "Failed to get alice balance: {:?}", result);
    assert_eq!(vm.stack.pop().unwrap(), 60.0, "Alice's balance incorrect after transfer");
    
    let result = vm.execute_balance("test_coin", "bob");
    assert!(result.is_ok(), "Failed to get bob balance: {:?}", result);
    assert_eq!(vm.stack.pop().unwrap(), 40.0, "Bob's balance incorrect after transfer");
    
    // Test transfer with message
    let transfer_result = vm.execute_transfer(
        "test_coin", 
        "alice", 
        "bob", 
        10.0, 
        &Some("Payment for services".to_string())
    );
    assert!(transfer_result.is_ok(), "Failed to transfer tokens with message: {:?}", transfer_result);
    
    // Verify balances were updated
    let result = vm.execute_balance("test_coin", "alice");
    assert!(result.is_ok(), "Failed to get alice balance: {:?}", result);
    assert_eq!(vm.stack.pop().unwrap(), 50.0, "Alice's balance incorrect after second transfer");
    
    let result = vm.execute_balance("test_coin", "bob");
    assert!(result.is_ok(), "Failed to get bob balance: {:?}", result);
    assert_eq!(vm.stack.pop().unwrap(), 50.0, "Bob's balance incorrect after second transfer");
    
    // Test transfer of more tokens than available
    let transfer_result = vm.execute_transfer("test_coin", "alice", "bob", 60.0, &None);
    assert!(transfer_result.is_err(), "Should fail when transferring more than available");
}

#[test]
fn test_integration_economic_ops() {
    let mut vm = setup_vm();
    
    // Set up a resource for testing
    let resource_key = "resources/community_token";
    let balances_key = "resources/community_token/balances";
    
    // Create resource and empty balances
    let storage = vm.get_storage_mut();
    let auth = vm.auth_context.clone();
    let ns = vm.namespace.clone();
    
    let token_metadata = r#"{
        "id": "community_token",
        "name": "Community Token",
        "description": "A token for community governance and resource sharing",
        "resource_type": "currency",
        "issuer_namespace": "test_namespace",
        "created_at": 1618531200000,
        "metadata": {
            "symbol": "COMM",
            "decimals": "2"
        },
        "transferable": true,
        "divisible": true
    }"#;
    
    storage.set(&auth, &ns, resource_key, token_metadata.as_bytes().to_vec()).unwrap();
    storage.set(&auth, &ns, balances_key, "{}".as_bytes().to_vec()).unwrap();
    
    // Initial minting to founders and community fund
    let mint_result = vm.execute_mint("community_token", "founder1", 1000.0, 
                                       &Some("Founder allocation".to_string()));
    assert!(mint_result.is_ok(), "Failed to mint to founder1: {:?}", mint_result);
    
    let mint_result = vm.execute_mint("community_token", "founder2", 1000.0, 
                                       &Some("Founder allocation".to_string()));
    assert!(mint_result.is_ok(), "Failed to mint to founder2: {:?}", mint_result);
    
    let mint_result = vm.execute_mint("community_token", "community_fund", 8000.0, 
                                       &Some("Community fund initial allocation".to_string()));
    assert!(mint_result.is_ok(), "Failed to mint to community fund: {:?}", mint_result);
    
    // Transfer from community fund to project team
    let transfer_result = vm.execute_transfer(
        "community_token", 
        "community_fund", 
        "project_team", 
        500.0, 
        &Some("Funding for Project Alpha".to_string())
    );
    assert!(transfer_result.is_ok(), "Failed to transfer to project team: {:?}", transfer_result);
    
    // Check balances
    let result = vm.execute_balance("community_token", "founder1");
    assert!(result.is_ok());
    assert_eq!(vm.stack.pop().unwrap(), 1000.0);
    
    let result = vm.execute_balance("community_token", "founder2");
    assert!(result.is_ok());
    assert_eq!(vm.stack.pop().unwrap(), 1000.0);
    
    let result = vm.execute_balance("community_token", "community_fund");
    assert!(result.is_ok());
    assert_eq!(vm.stack.pop().unwrap(), 7500.0);
    
    let result = vm.execute_balance("community_token", "project_team");
    assert!(result.is_ok());
    assert_eq!(vm.stack.pop().unwrap(), 500.0);
    
    // Test burning tokens
    let burn_result = vm.execute_burn("community_token", "project_team", 50.0, 
                                       &Some("Redeemed for community workshop".to_string()));
    assert!(burn_result.is_ok(), "Failed to burn tokens: {:?}", burn_result);
    
    let result = vm.execute_balance("community_token", "project_team");
    assert!(result.is_ok());
    assert_eq!(vm.stack.pop().unwrap(), 450.0);
    
    // Check events were recorded
    assert!(vm.events.len() > 0, "No events were recorded");
    assert!(vm.events.iter().any(|e| e.category == "economic"), "No economic events were recorded");
} 