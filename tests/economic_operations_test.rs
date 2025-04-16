use icn_covm::storage::auth::AuthContext;
use icn_covm::storage::implementations::in_memory::InMemoryStorage;
use icn_covm::storage::traits::StorageBackend;
use icn_covm::vm::{Op, VMError, VM};
use std::fmt::Debug;

// This trait adds extension methods to VM for testing purposes
trait VMTestExtensions<S> 
where 
    S: StorageBackend + Send + Sync + Clone + Debug + 'static 
{
    // Helper to check if a key exists in storage
    fn storage_contains(&self, key: &str) -> bool;
    
    // Helper to access storage for verification in tests
    fn with_storage<F, T>(&self, f: F) -> T
    where
        F: FnOnce(&S, &AuthContext, &str) -> T;
}

// Implementation of extension methods for VM
impl<S> VMTestExtensions<S> for VM<S> 
where 
    S: StorageBackend + Send + Sync + Clone + Debug + 'static 
{
    fn storage_contains(&self, key: &str) -> bool {
        self.with_storage(|storage, auth, namespace| {
            storage.contains(Some(auth), namespace, key).unwrap_or(false)
        })
    }
    
    fn with_storage<F, T>(&self, f: F) -> T
    where
        F: FnOnce(&S, &AuthContext, &str) -> T,
    {
        // Fork to safely access the storage
        let forked = self.fork().expect("Failed to fork VM for storage operation");
        
        // Get the storage backend using the accessor method
        let storage = forked.get_storage_backend()
            .expect("No storage backend available in VM");
        
        // Get the auth context from the forked VM
        let auth_context = forked.get_auth_context()
            .expect("No auth context available in VM");
        
        // Get the namespace from the forked VM
        let namespace = forked.get_namespace()
            .expect("No namespace available in VM");
        
        // Call the provided function with the retrieved values
        f(storage, auth_context, namespace)
    }
}

fn setup_vm() -> VM<InMemoryStorage> {
    let storage = InMemoryStorage::new();

    // Create auth context with appropriate permissions
    let mut auth_context = AuthContext::new("test_user");

    // Add roles with permissions for the test namespace
    auth_context.add_role("test_namespace", "admin");
    auth_context.add_role("global", "admin"); // Global admin bypasses all permission checks

    // Create VM with in-memory storage and set auth context
    let mut vm = VM::with_storage_backend(storage);
    vm.set_auth_context(auth_context);
    vm.set_namespace("test_namespace");

    // Create a storage account for the test user and namespace
    let create_account_op = Op::CreateAccount {
        user_id: "test_user".to_string(),
        quota: 1_000_000,
    };
    vm.execute(&[create_account_op]).expect("Failed to create account");

    // Create the test namespace
    let create_namespace_op = Op::CreateNamespace {
        namespace: "test_namespace".to_string(),
        quota: 1_000_000,
        metadata: None,
    };
    vm.execute(&[create_namespace_op]).expect("Failed to create namespace");

    // Create a resources directory that will hold all resources
    let create_resources_op = Op::StoreBytes {
        key: "resources".to_string(),
        value: "{}".as_bytes().to_vec(),
    };
    vm.execute(&[create_resources_op]).expect("Failed to create resources directory");

    vm
}

// Helper functions to run economic operations
fn create_resource(vm: &mut VM<InMemoryStorage>, resource_id: &str) -> Result<(), VMError> {
    let op = Op::CreateResource(resource_id.to_string());
    println!("Creating resource: {}", resource_id);

    // Check resources directory before operation
    let resources_exists = vm.storage_contains("resources");
    println!("Resources directory exists: {}", resources_exists);

    // Execute the create resource operation
    let result = vm.execute(&[op]);
    if let Err(err) = &result {
        println!("Create resource error: {:?}", err);
        return result;
    } 
    
    println!("Resource created successfully!");

    // Verify the resource was created using our extension method
    let resource_path = format!("resources/{}", resource_id);
    let resource_exists = vm.storage_contains(&resource_path);
    println!("Resource now exists: {}", resource_exists);

    let balances_path = format!("resources/{}/balances", resource_id);
    let balances_exists = vm.storage_contains(&balances_path);
    println!("Balances now exist: {}", balances_exists);

    result
}

fn mint_tokens(
    vm: &mut VM<InMemoryStorage>,
    resource: &str,
    account: &str,
    amount: f64,
    reason: Option<String>,
) -> Result<(), VMError> {
    let op = Op::Mint {
        resource: resource.to_string(),
        account: account.to_string(),
        amount,
        reason,
    };
    let result = vm.execute(&[op]);
    if let Err(err) = &result {
        println!("Mint tokens error: {:?}", err);
    }
    result
}

fn transfer_tokens(
    vm: &mut VM<InMemoryStorage>,
    resource: &str,
    from: &str,
    to: &str,
    amount: f64,
    reason: Option<String>,
) -> Result<(), VMError> {
    let op = Op::Transfer {
        resource: resource.to_string(),
        from: from.to_string(),
        to: to.to_string(),
        amount,
        reason,
    };
    let result = vm.execute(&[op]);
    if let Err(err) = &result {
        println!("Transfer tokens error: {:?}", err);
    }
    result
}

fn burn_tokens(
    vm: &mut VM<InMemoryStorage>,
    resource: &str,
    account: &str,
    amount: f64,
    reason: Option<String>,
) -> Result<(), VMError> {
    let op = Op::Burn {
        resource: resource.to_string(),
        account: account.to_string(),
        amount,
        reason,
    };
    let result = vm.execute(&[op]);
    if let Err(err) = &result {
        println!("Burn tokens error: {:?}", err);
    }
    result
}

fn get_balance(vm: &mut VM<InMemoryStorage>, resource: &str, account: &str) -> Result<f64, VMError> {
    let op = Op::Balance {
        resource: resource.to_string(),
        account: account.to_string(),
    };
    let result = vm.execute(&[op]);
    if let Err(err) = &result {
        println!("Get balance error: {:?}", err);
        return Err(err.clone());
    }
    
    // The result of a balance operation is pushed onto the stack
    Ok(vm.top().unwrap_or(0.0))
}

// Economic Operations Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_resource() {
        let mut vm = setup_vm();
        
        // Create a test resource
        let resource_id = "test_coin";
        let result = create_resource(&mut vm, resource_id);
        assert!(result.is_ok(), "Failed to create resource: {:?}", result);
        
        // Verify the resource was created
        let resource_path = format!("resources/{}", resource_id);
        let resource_exists = vm.storage_contains(&resource_path);
        assert!(resource_exists, "Resource was not created in storage");
        
        // Verify the resource has a balances node
        let balances_path = format!("resources/{}/balances", resource_id);
        let balances_exists = vm.storage_contains(&balances_path);
        assert!(balances_exists, "Resource balances were not created in storage");
    }
    
    #[test]
    fn test_mint_and_burn() {
        let mut vm = setup_vm();
        
        // Create a test resource
        let resource_id = "test_coin";
        create_resource(&mut vm, resource_id).unwrap();
        
        // Mint some tokens for 'alice'
        let account = "alice";
        let amount = 100.0;
        let mint_result = mint_tokens(&mut vm, resource_id, account, amount, None);
        assert!(mint_result.is_ok(), "Failed to mint tokens: {:?}", mint_result);
        
        // Check the balance after minting
        let alice_balance = get_balance(&mut vm, resource_id, account).unwrap();
        assert_eq!(alice_balance, amount, "Balance after mint is incorrect");
        
        // Burn some tokens
        let burn_amount = 30.0;
        let burn_result = burn_tokens(&mut vm, resource_id, account, burn_amount, None);
        assert!(burn_result.is_ok(), "Failed to burn tokens: {:?}", burn_result);
        
        // Check the balance after burning
        let alice_balance_after_burn = get_balance(&mut vm, resource_id, account).unwrap();
        assert_eq!(alice_balance_after_burn, amount - burn_amount, 
                  "Balance after burn is incorrect");
        
        // Try to burn more than available, should fail
        let burn_too_much = burn_tokens(&mut vm, resource_id, account, 100.0, None);
        assert!(burn_too_much.is_err(), "Burning more than available should fail");
    }

    #[test]
    fn test_transfer() {
        let mut vm = setup_vm();
        
        // Create a test resource
        let resource_id = "test_coin";
        create_resource(&mut vm, resource_id).unwrap();
        
        // Mint some tokens for 'alice'
        let alice = "alice";
        let bob = "bob";
        let initial_amount = 100.0;
        mint_tokens(&mut vm, resource_id, alice, initial_amount, None).unwrap();
        
        // Transfer some tokens from alice to bob
        let transfer_amount = 30.0;
        let transfer_result = transfer_tokens(
            &mut vm, resource_id, alice, bob, transfer_amount, None
        );
        assert!(transfer_result.is_ok(), "Failed to transfer tokens: {:?}", transfer_result);
        
        // Check the balances after transfer
        let alice_balance = get_balance(&mut vm, resource_id, alice).unwrap();
        let bob_balance = get_balance(&mut vm, resource_id, bob).unwrap();
        
        assert_eq!(alice_balance, initial_amount - transfer_amount,
                  "Alice's balance after transfer is incorrect");
        assert_eq!(bob_balance, transfer_amount,
                  "Bob's balance after transfer is incorrect");
        
        // Try to transfer more than available, should fail
        let transfer_too_much = transfer_tokens(
            &mut vm, resource_id, alice, bob, 100.0, None
        );
        assert!(transfer_too_much.is_err(), 
                "Transferring more than available should fail");
    }

    #[test]
    fn test_transfer_with_reason() {
        let mut vm = setup_vm();
        
        // Create a test resource
        let resource_id = "test_coin";
        create_resource(&mut vm, resource_id).unwrap();
        
        // Mint some tokens for 'alice'
        let alice = "alice";
        let bob = "bob";
        let initial_amount = 100.0;
        mint_tokens(&mut vm, resource_id, alice, initial_amount, None).unwrap();
        
        // Transfer with reason
        let transfer_amount = 30.0;
        let reason = Some("Payment for services".to_string());
        let transfer_result = transfer_tokens(
            &mut vm, resource_id, alice, bob, transfer_amount, reason.clone()
        );
        assert!(transfer_result.is_ok(), 
                "Failed to transfer tokens with reason: {:?}", transfer_result);
        
        // Check the balances after transfer
        let alice_balance = get_balance(&mut vm, resource_id, alice).unwrap();
        let bob_balance = get_balance(&mut vm, resource_id, bob).unwrap();
        
        assert_eq!(alice_balance, initial_amount - transfer_amount,
                  "Alice's balance after transfer is incorrect");
        assert_eq!(bob_balance, transfer_amount,
                  "Bob's balance after transfer is incorrect");
    }
    
    #[test]
    fn test_cross_resource_transfer() {
        let mut vm = setup_vm();
        
        // Create two different resources
        let gold_coin = "gold_coin";
        let silver_coin = "silver_coin";
        create_resource(&mut vm, gold_coin).unwrap();
        create_resource(&mut vm, silver_coin).unwrap();
        
        // Mint some tokens for 'alice' in both resources
        let alice = "alice";
        let bob = "bob";
        let gold_amount = 100.0;
        let silver_amount = 200.0;
        
        mint_tokens(&mut vm, gold_coin, alice, gold_amount, None).unwrap();
        mint_tokens(&mut vm, silver_coin, alice, silver_amount, None).unwrap();
        
        // Transfer gold from alice to bob
        let gold_transfer = 30.0;
        transfer_tokens(&mut vm, gold_coin, alice, bob, gold_transfer, None).unwrap();
        
        // Check the balances after gold transfer
        let alice_gold = get_balance(&mut vm, gold_coin, alice).unwrap();
        let bob_gold = get_balance(&mut vm, gold_coin, bob).unwrap();
        
        assert_eq!(alice_gold, gold_amount - gold_transfer,
                  "Alice's gold balance after transfer is incorrect");
        assert_eq!(bob_gold, gold_transfer,
                  "Bob's gold balance after transfer is incorrect");
        
        // Transfer silver from alice to bob
        let silver_transfer = 50.0;
        transfer_tokens(&mut vm, silver_coin, alice, bob, silver_transfer, None).unwrap();
        
        // Check the balances after silver transfer
        let alice_silver = get_balance(&mut vm, silver_coin, alice).unwrap();
        let bob_silver = get_balance(&mut vm, silver_coin, bob).unwrap();
        
        assert_eq!(alice_silver, silver_amount - silver_transfer,
                  "Alice's silver balance after transfer is incorrect");
        assert_eq!(bob_silver, silver_transfer,
                  "Bob's silver balance after transfer is incorrect");
        
        // Ensure gold balances weren't affected by silver transfer
        let alice_gold_after = get_balance(&mut vm, gold_coin, alice).unwrap();
        let bob_gold_after = get_balance(&mut vm, gold_coin, bob).unwrap();
        
        assert_eq!(alice_gold_after, alice_gold,
                  "Alice's gold balance should not change during silver transfer");
        assert_eq!(bob_gold_after, bob_gold,
                  "Bob's gold balance should not change during silver transfer");
    }
}
