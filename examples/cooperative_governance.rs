use std::error::Error;
use serde::{Serialize, Deserialize};

// Import our storage types
use icn_covm::storage::{
    AuthContext, InMemoryStorage, ResourceAccount, StorageBackend, StorageEvent, StorageResult
};

// Define a simple data structure for testing
#[derive(Serialize, Deserialize, Debug, Clone)]
struct ProposalData {
    id: String,
    title: String,
    description: String,
    creator: String,
    status: String,
    votes_for: u32,
    votes_against: u32,
}

// Define Vote structure
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Vote {
    voter: String,
    choice: String, // "for", "against", "abstain"
    reason: Option<String>,
    timestamp: u64,
}

// Helper trait for JSON storage with resource accounting (adapt as needed)
trait JsonStorageHelper: StorageBackend {
    fn set_json_with_resources<T: Serialize>(
        &mut self,
        auth: Option<&AuthContext>,
        namespace: &str,
        key: &str,
        value: &T,
    ) -> StorageResult<()> {
        let json = serde_json::to_vec(value)
            .map_err(|e| icn_covm::storage::StorageError::SerializationError { details: e.to_string() })?;
        self.set(auth, namespace, key, json)
    }
    
    fn get_json<T: for<'de> Deserialize<'de>>(
        &self,
        auth: Option<&AuthContext>,
        namespace: &str,
        key: &str,
    ) -> StorageResult<T> {
        let bytes = self.get(auth, namespace, key)?;
        serde_json::from_slice(&bytes)
            .map_err(|e| icn_covm::storage::StorageError::SerializationError { details: e.to_string() })
    }
}

impl<S: StorageBackend> JsonStorageHelper for S {}

fn main() -> Result<(), Box<dyn Error>> {
    println!("--- Cooperative Governance Example ---");
    
    // 1. Initialize Storage Backend
    let mut storage = InMemoryStorage::new();
    let namespace = "governance"; // Define a namespace
    
    // 2. Create Auth Contexts & Identities
    let admin_id = "admin_user";
    let member1_id = "member_1";
    let member2_id = "member_2";
    
    let admin_auth = AuthContext::new(admin_id).with_roles(vec!["admin".to_string()]); // Roles applied to "default" namespace by default
    let member1_auth = AuthContext::new(member1_id).with_roles(vec!["member".to_string()]);
    let member2_auth = AuthContext::new(member2_id).with_roles(vec!["member".to_string()]);
    
    // Initialize storage accounts/namespaces
    storage.create_account(Some(&admin_auth), admin_id, 1024 * 1024)?; 
    storage.create_account(Some(&admin_auth), member1_id, 1024 * 1024)?; 
    storage.create_account(Some(&admin_auth), member2_id, 1024 * 1024)?;
    storage.create_namespace(Some(&admin_auth), namespace, 1024 * 1024, None)?; 
    
    // 3. Store Member Profiles (using admin privileges)
    let admin_profile = MemberProfile::new(Identity::new(admin_id, "admin"), icn_covm::storage::now());
    storage.set_json_with_resources(Some(&admin_auth), namespace, &format!("members/{}", admin_id), &admin_profile)?;
    let member1_profile = MemberProfile::new(Identity::new(member1_id, "member"), icn_covm::storage::now());
    storage.set_json_with_resources(Some(&admin_auth), namespace, &format!("members/{}", member1_id), &member1_profile)?;
    let member2_profile = MemberProfile::new(Identity::new(member2_id, "member"), icn_covm::storage::now());
    storage.set_json_with_resources(Some(&admin_auth), namespace, &format!("members/{}", member2_id), &member2_profile)?;
    
    // 4. Create a Proposal (by a member)
    let proposal = ProposalData {
        id: "prop-001".to_string(),
        title: "Increase Budget for Snacks".to_string(),
        description: "Allocate an additional $50 monthly for snacks.".to_string(),
        creator: member1_id.to_string(),
        status: "open".to_string(),
        votes_for: 0,
        votes_against: 0,
    };
    storage.set_json_with_resources(Some(&member1_auth), namespace, "proposals/prop-001", &proposal)?;
    println!("Proposal '{}' created by {}", proposal.title, proposal.creator);
    
    // 5. Submit Votes
    // Member 1 votes FOR
    let vote1 = Vote { voter: member1_id.to_string(), choice: "for".to_string(), reason: None, timestamp: icn_covm::storage::now() };
    storage.set_json_with_resources(Some(&member1_auth), namespace, "votes/prop-001/member_1", &vote1)?;
    println!("Vote cast by {}", member1_id);
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
    storage.set_with_auth(&admin, 
        "governance/members/admin_user", 
        "Admin User with full permissions"
    )?;
    
    storage.set_with_auth(&admin, 
        "governance/members/member_1", 
        "Regular member with voting rights"
    )?;
    
    storage.set_with_auth(&admin, 
        "governance/members/member_2", 
        "Regular member with voting rights"
    )?;
    
    // Set governance configuration
    println!("2. Setting up governance configuration");
    
    storage.set_with_auth(&admin, 
        "governance/config/quorum", 
        "0.5"
    )?;
    
    storage.set_with_auth(&admin, 
        "governance/config/vote_threshold", 
        "0.66"
    )?;
    
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
        &mut member1_account
    )?;
    
    println!("  Member 1 resource balance after creating proposal: {}", member1_account.balance);
    
    // Demonstrate delegation (liquid democracy)
    println!("4. Setting up vote delegation (liquid democracy)");
    
    storage.set_with_auth(
        &member2, 
        "governance/delegations/member_2/member_1", 
        "1.0"
    )?;
    
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
        &mut member1_account
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
        &mut admin_account
    )?;
    
    // Observer trying to vote (should fail)
    println!("6. Testing access control - Observer trying to vote");
    
    let vote_result = storage.set_with_auth(
        &observer,
        "governance/votes/prop-001/observer_1",
        "Invalid vote"
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
            StorageEvent::Access { key, action, user, .. } => {
                println!("  {}: {} {} key '{}'", i+1, user, action, key);
            },
            StorageEvent::Transaction { action, user, .. } => {
                println!("  {}: {} performed transaction '{}'", i+1, user, action);
            },
            StorageEvent::ResourceUsage { account, amount, operation, .. } => {
                println!("  {}: Account '{}' used {} resources for '{}'", i+1, account, amount, operation);
            }
        }
    }
    
    // Display final resource accounting
    println!("\n9. Final resource accounting");
    println!("  Admin account: {}/{} units remaining", admin_account.balance, admin_account.quota);
    println!("  Member 1 account: {}/{} units remaining", member1_account.balance, member1_account.quota);
    println!("  Member 2 account: {}/{} units remaining", member2_account.balance, member2_account.quota);
    
    // Display versioning information
    println!("\n10. Checking proposal version history");
    
    match storage.list_versions("governance/proposals/prop-001") {
        Ok(versions) => {
            for version in versions {
                println!("  Version {}: created by {} at timestamp {}", 
                    version.version, version.author, version.timestamp);
            }
        },
        Err(e) => println!("  Error retrieving versions: {:?}", e),
    }
    
    Ok(())
} 