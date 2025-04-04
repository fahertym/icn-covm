use serde::{Deserialize, Serialize};
use std::error::Error;

// Import our storage types
use icn_covm::storage::{
    AuthContext, InMemoryStorage, ResourceAccount, StorageBackend, StorageEvent, StorageResult,
};

// Simple proposal structure
#[derive(Serialize, Deserialize, Debug, Clone)]
struct ProposalData {
    title: String,
    description: String,
    creator: String,
    options: Vec<String>,
    created_at: u64,
    status: String,
}

// Simple vote structure
#[derive(Serialize, Deserialize, Debug, Clone)]
struct VoteData {
    proposal_id: String,
    voter: String,
    choice: String,
    weight: f64,
    timestamp: u64,
}

// Helper trait to support JSON ops with resource accounting
trait JsonWithResources: StorageBackend {
    fn set_json_with_resources<T: Serialize>(
        &mut self,
        auth: &AuthContext,
        key: &str,
        value: &T,
        account: &mut ResourceAccount,
    ) -> StorageResult<()> {
        // Serialize to JSON string
        let json = serde_json::to_string_pretty(value)
            .map_err(|e| icn_covm::storage::StorageError::SerializationError(e.to_string()))?;

        // Store with resource accounting
        self.set_with_resources(auth, key, &json, account)
    }

    fn get_json<T: for<'de> Deserialize<'de>>(&self, key: &str) -> StorageResult<T> {
        let json = self.get(key)?;
        serde_json::from_str(&json)
            .map_err(|e| icn_covm::storage::StorageError::SerializationError(e.to_string()))
    }
}

// Implement for InMemoryStorage
impl JsonWithResources for InMemoryStorage {}

fn main() -> Result<(), Box<dyn Error>> {
    // Initialize storage
    let mut storage = InMemoryStorage::new();

    println!("=== Cooperative Governance Demo ===\n");

    // Create auth contexts for different roles
    let admin = AuthContext::with_roles("admin_user", vec!["admin".to_string()]);
    let member1 = AuthContext::with_roles("member_1", vec!["member".to_string()]);
    let member2 = AuthContext::with_roles("member_2", vec!["member".to_string()]);
    let observer = AuthContext::with_roles("observer_1", vec!["observer".to_string()]);

    // Create resource accounts for each user
    let mut admin_account = storage.create_resource_account("admin_user", 100.0);
    let mut member1_account = storage.create_resource_account("member_1", 50.0);
    let mut member2_account = storage.create_resource_account("member_2", 50.0);

    println!("1. Creating member records in governance storage");

    // Store member data
    storage.set_with_auth(
        &admin,
        "governance/members/admin_user",
        "Admin User with full permissions",
    )?;

    storage.set_with_auth(
        &admin,
        "governance/members/member_1",
        "Regular member with voting rights",
    )?;

    storage.set_with_auth(
        &admin,
        "governance/members/member_2",
        "Regular member with voting rights",
    )?;

    // Set governance configuration
    println!("2. Setting up governance configuration");

    storage.set_with_auth(&admin, "governance/config/quorum", "0.5")?;

    storage.set_with_auth(&admin, "governance/config/vote_threshold", "0.66")?;

    // Create a proposal
    println!("3. Creating a new governance proposal");

    let proposal = ProposalData {
        title: "Add Solar Panels to Cooperative Building".to_string(),
        description: "Install 20kW of solar panels on the cooperative's main building.".to_string(),
        creator: "member_1".to_string(),
        options: vec!["approve".to_string(), "reject".to_string()],
        created_at: icn_covm::storage::now(),
        status: "open".to_string(),
    };

    // Use resource accounting when storing the proposal
    storage.set_json_with_resources(
        &member1,
        "governance/proposals/prop-001",
        &proposal,
        &mut member1_account,
    )?;

    println!(
        "  Member 1 resource balance after creating proposal: {}",
        member1_account.balance
    );

    // Demonstrate delegation (liquid democracy)
    println!("4. Setting up vote delegation (liquid democracy)");

    storage.set_with_auth(&member2, "governance/delegations/member_2/member_1", "1.0")?;

    println!("  Member 2 delegated their voting power to Member 1");

    // Cast votes
    println!("5. Casting votes on the proposal");

    // Member 1 votes (with double weight due to delegation)
    let vote1 = VoteData {
        proposal_id: "prop-001".to_string(),
        voter: "member_1".to_string(),
        choice: "approve".to_string(),
        weight: 2.0, // Their vote plus delegation from member_2
        timestamp: icn_covm::storage::now(),
    };

    storage.set_json_with_resources(
        &member1,
        "governance/votes/prop-001/member_1",
        &vote1,
        &mut member1_account,
    )?;

    // Admin votes
    let vote2 = VoteData {
        proposal_id: "prop-001".to_string(),
        voter: "admin_user".to_string(),
        choice: "approve".to_string(),
        weight: 1.0,
        timestamp: icn_covm::storage::now(),
    };

    storage.set_json_with_resources(
        &admin,
        "governance/votes/prop-001/admin_user",
        &vote2,
        &mut admin_account,
    )?;

    // Observer trying to vote (should fail)
    println!("6. Testing access control - Observer trying to vote");

    let vote_result = storage.set_with_auth(
        &observer,
        "governance/votes/prop-001/observer_1",
        "Invalid vote",
    );

    match vote_result {
        Ok(_) => println!("  UNEXPECTED: Observer was allowed to vote!"),
        Err(e) => println!("  Expected error: {:?}", e),
    }

    // Calculating result
    println!("7. Calculating vote results with transaction");

    // Start transaction for atomic updates
    storage.begin_transaction()?;

    // Update proposal status
    let mut prop_data: ProposalData = storage.get_json("governance/proposals/prop-001")?;
    prop_data.status = "approved".to_string();

    let json = serde_json::to_string_pretty(&prop_data)
        .map_err(|e| icn_covm::storage::StorageError::SerializationError(e.to_string()))?;
    storage.set("governance/proposals/prop-001", &json)?;

    // Record the final tally
    storage.set("governance/config/last_proposal_result", "approved")?;

    // Commit all changes
    storage.commit_transaction()?;

    println!("  Proposal status updated to: {}", prop_data.status);

    // Check audit log
    println!("\n8. Reviewing audit trail");

    for (i, event) in storage.get_audit_log().iter().enumerate() {
        match event {
            StorageEvent::Access {
                key, action, user, ..
            } => {
                println!("  {}: {} {} key '{}'", i + 1, user, action, key);
            }
            StorageEvent::Transaction { action, user, .. } => {
                println!("  {}: {} performed transaction '{}'", i + 1, user, action);
            }
            StorageEvent::ResourceUsage {
                account,
                amount,
                operation,
                ..
            } => {
                println!(
                    "  {}: Account '{}' used {} resources for '{}'",
                    i + 1,
                    account,
                    amount,
                    operation
                );
            }
        }
    }

    // Display final resource accounting
    println!("\n9. Final resource accounting");
    println!(
        "  Admin account: {}/{} units remaining",
        admin_account.balance, admin_account.quota
    );
    println!(
        "  Member 1 account: {}/{} units remaining",
        member1_account.balance, member1_account.quota
    );
    println!(
        "  Member 2 account: {}/{} units remaining",
        member2_account.balance, member2_account.quota
    );

    // Display versioning information
    println!("\n10. Checking proposal version history");

    match storage.list_versions("governance/proposals/prop-001") {
        Ok(versions) => {
            for version in versions {
                println!(
                    "  Version {}: created by {} at timestamp {}",
                    version.version, version.author, version.timestamp
                );
            }
        }
        Err(e) => println!("  Error retrieving versions: {:?}", e),
    }

    Ok(())
}
