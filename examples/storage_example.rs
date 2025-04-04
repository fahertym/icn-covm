use serde::{Deserialize, Serialize};
use std::error::Error;

// Import our storage types
use icn_covm::storage::{
    AuthContext, InMemoryStorage, ResourceAccount, StorageBackend, StorageError, StorageEvent,
    StorageResult,
};

// Simple cooperative member structure
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Member {
    id: String,
    name: String,
    join_date: u64,
    roles: Vec<String>,
}

// Simple proposal structure
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Proposal {
    id: String,
    title: String,
    description: String,
    creator: String,
    votes_for: f64,
    votes_against: f64,
    status: String,
}

// Helper trait for JSON operations
trait JsonHelper: StorageBackend {
    fn set_json<T: Serialize>(&mut self, key: &str, value: &T) -> StorageResult<()> {
        let json = serde_json::to_string_pretty(value)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;
        self.set(key, &json)
    }

    fn get_json<T: for<'de> Deserialize<'de>>(&self, key: &str) -> StorageResult<T> {
        let json = self.get(key)?;
        serde_json::from_str(&json).map_err(|e| StorageError::SerializationError(e.to_string()))
    }
}

// Implement for InMemoryStorage
impl JsonHelper for InMemoryStorage {}

fn main() -> Result<(), Box<dyn Error>> {
    println!("=== ICN-COVM Storage Example ===\n");

    // Create storage instance
    let mut storage = InMemoryStorage::new();

    // Set up auth contexts
    let admin = AuthContext::with_roles("admin", vec!["admin".to_string()]);
    let member1 = AuthContext::with_roles("alice", vec!["member".to_string()]);
    let member2 = AuthContext::with_roles("bob", vec!["member".to_string()]);
    let observer = AuthContext::with_roles("observer", vec!["observer".to_string()]);

    // Create resource accounts for users
    let mut admin_account = storage.create_resource_account("admin", 100.0);
    let mut member1_account = storage.create_resource_account("alice", 50.0);
    let mut member2_account = storage.create_resource_account("bob", 50.0);

    println!("1. Storing member data with role-based access control");

    // Store admin member
    let admin_data = Member {
        id: "admin".to_string(),
        name: "Admin User".to_string(),
        join_date: icn_covm::storage::now(),
        roles: vec!["admin".to_string()],
    };

    storage.set_with_auth(
        &admin,
        "governance/members/admin",
        &serde_json::to_string(&admin_data)?,
    )?;

    // Store regular members
    let member1_data = Member {
        id: "alice".to_string(),
        name: "Alice".to_string(),
        join_date: icn_covm::storage::now(),
        roles: vec!["member".to_string()],
    };

    storage.set_with_auth(
        &admin,
        "governance/members/alice",
        &serde_json::to_string(&member1_data)?,
    )?;

    let member2_data = Member {
        id: "bob".to_string(),
        name: "Bob".to_string(),
        join_date: icn_covm::storage::now(),
        roles: vec!["member".to_string()],
    };

    storage.set_with_auth(
        &admin,
        "governance/members/bob",
        &serde_json::to_string(&member2_data)?,
    )?;

    println!("  All members stored successfully");

    // Set up a vote delegation (liquid democracy)
    println!("\n2. Setting up vote delegation (liquid democracy)");

    // Bob delegates his voting power to Alice
    storage.set_with_auth(&member2, "governance/delegations/bob/alice", "1.0")?;

    println!("  Bob delegated voting power to Alice");

    // Create a proposal
    println!("\n3. Creating a governance proposal with resource accounting");

    let proposal = Proposal {
        id: "prop-001".to_string(),
        title: "Expand community garden".to_string(),
        description: "Add 10 new plots to our cooperative community garden".to_string(),
        creator: "alice".to_string(),
        votes_for: 0.0,
        votes_against: 0.0,
        status: "open".to_string(),
    };

    // Store proposal with resource accounting
    storage.set_json("governance/proposals/prop-001", &proposal)?;
    println!("  Created proposal: {}", proposal.title);

    // Demonstrate RBAC by trying with observer (should fail)
    println!("\n4. Testing RBAC by having observer try to create a proposal");

    let observer_proposal = Proposal {
        id: "prop-002".to_string(),
        title: "Observer's proposal".to_string(),
        description: "This should be rejected".to_string(),
        creator: "observer".to_string(),
        votes_for: 0.0,
        votes_against: 0.0,
        status: "open".to_string(),
    };

    let result = storage.set_with_auth(
        &observer,
        "governance/proposals/prop-002",
        &serde_json::to_string(&observer_proposal)?,
    );

    match result {
        Ok(_) => println!("  Unexpected! Observer was allowed to create a proposal."),
        Err(e) => println!("  Expected error: {:?}", e),
    }

    // Cast votes using transactions
    println!("\n5. Casting votes with transaction guarantee");

    // Begin transaction
    storage.begin_transaction()?;

    // Alice votes for herself and Bob (delegated)
    let mut prop: Proposal = storage.get_json("governance/proposals/prop-001")?;
    prop.votes_for += 2.0; // Alice's vote + Bob's delegated vote

    // Store vote records
    storage.set("governance/votes/prop-001/alice", "for")?;
    storage.set("governance/votes/prop-001/bob", "delegated:alice")?;

    // Update proposal with new vote tallies
    storage.set_json("governance/proposals/prop-001", &prop)?;

    // Admin votes against
    prop.votes_against += 1.0;
    storage.set("governance/votes/prop-001/admin", "against")?;

    // Update proposal with final tally
    storage.set_json("governance/proposals/prop-001", &prop)?;

    // Commit all changes atomically
    storage.commit_transaction()?;

    println!(
        "  Votes cast and recorded. Current tally: {} for, {} against",
        prop.votes_for, prop.votes_against
    );

    // Check versioning
    println!("\n6. Checking version history of proposal");

    match storage.list_versions("governance/proposals/prop-001") {
        Ok(versions) => {
            println!("  Found {} versions of the proposal", versions.len());
            for version in versions {
                println!(
                    "  - Version {} by {} at {}",
                    version.version, version.author, version.timestamp
                );
            }
        }
        Err(e) => println!("  Error retrieving versions: {:?}", e),
    }

    // Print resource usage
    println!("\n7. Resource accounting report");
    println!(
        "  Admin account: {}/{} units remaining",
        admin_account.balance, admin_account.quota
    );
    println!(
        "  Alice account: {}/{} units remaining",
        member1_account.balance, member1_account.quota
    );
    println!(
        "  Bob account: {}/{} units remaining",
        member2_account.balance, member2_account.quota
    );

    // Show audit trail
    println!("\n8. Audit trail of all storage operations");

    for (i, event) in storage.get_audit_log().iter().enumerate().take(10) {
        match event {
            StorageEvent::Access {
                key,
                action,
                user,
                timestamp,
            } => {
                println!(
                    "  {}. [{}] User '{}' {}ed key '{}'",
                    i + 1,
                    timestamp,
                    user,
                    action,
                    key
                );
            }
            StorageEvent::Transaction {
                action,
                user,
                timestamp,
            } => {
                println!(
                    "  {}. [{}] User '{}' performed transaction '{}'",
                    i + 1,
                    timestamp,
                    user,
                    action
                );
            }
            StorageEvent::ResourceUsage {
                account,
                amount,
                operation,
                timestamp,
            } => {
                println!(
                    "  {}. [{}] Account '{}' used {} units for '{}'",
                    i + 1,
                    timestamp,
                    account,
                    amount,
                    operation
                );
            }
        }
    }

    // Show final status
    println!("\n9. Final proposal status");
    let final_prop: Proposal = storage.get_json("governance/proposals/prop-001")?;

    // Calculate result
    let total_votes = final_prop.votes_for + final_prop.votes_against;
    let approval_percentage = if total_votes > 0.0 {
        (final_prop.votes_for / total_votes) * 100.0
    } else {
        0.0
    };

    println!("  Proposal: {}", final_prop.title);
    println!("  Status: {}", final_prop.status);
    println!(
        "  Votes: {} for ({:.1}%), {} against",
        final_prop.votes_for, approval_percentage, final_prop.votes_against
    );

    if approval_percentage >= 66.0 {
        println!("  Result: APPROVED (≥66% threshold met)");

        // Update status
        let mut updated_prop = final_prop;
        updated_prop.status = "approved".to_string();
        storage.set_json("governance/proposals/prop-001", &updated_prop)?;
    } else {
        println!("  Result: REJECTED (<66% threshold)");

        // Update status
        let mut updated_prop = final_prop;
        updated_prop.status = "rejected".to_string();
        storage.set_json("governance/proposals/prop-001", &updated_prop)?;
    }

    println!("\nStorage example completed successfully.");

    Ok(())
}
