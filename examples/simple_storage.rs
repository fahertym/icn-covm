use icn_covm::storage::{AuthContext, InMemoryStorage, StorageBackend, StorageEvent, StorageResult, StorageError, VersionInfo};
use icn_covm::identity::{Identity, MemberProfile};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Member {
    id: String,
    name: String,
    reputation: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct Proposal {
    id: String,
    title: String,
    description: String,
    proposed_by: String,
    required_votes: u32,
    approve_threshold: f64,
}

#[derive(Debug, Serialize, Deserialize)]
struct Vote {
    voter: String,
    proposal_id: String,
    approved: bool,
    comment: Option<String>,
}

// Helper trait for JSON storage (similar to other examples)
trait JsonStorageHelper: StorageBackend {
    fn set_json<T: Serialize>(&mut self, auth: Option<&AuthContext>, namespace: &str, key: &str, value: &T) -> StorageResult<()>;
    fn get_json<T: for<'de> Deserialize<'de>>(&self, auth: Option<&AuthContext>, namespace: &str, key: &str) -> StorageResult<T>;
}

impl<S: StorageBackend> JsonStorageHelper for S {
    fn set_json<T: Serialize>(&mut self, auth: Option<&AuthContext>, namespace: &str, key: &str, value: &T) -> StorageResult<()> {
        let json = serde_json::to_vec(value)
            .map_err(|e| StorageError::SerializationError { details: e.to_string() })?;
        self.set(auth, namespace, key, json)
    }

    fn get_json<T: for<'de> Deserialize<'de>>(&self, auth: Option<&AuthContext>, namespace: &str, key: &str) -> StorageResult<T> {
        let bytes = self.get(auth, namespace, key)?;
        serde_json::from_slice(&bytes)
            .map_err(|e| StorageError::SerializationError { details: e.to_string() })
    }
}

fn main() -> StorageResult<()> {
    println!("=== ICN-COVM Cooperative Storage Example ===");
    
    // Initialize storage
    let mut storage = InMemoryStorage::new();
    
    // Create user roles
    let admin = AuthContext::with_roles("admin1", vec!["admin".to_string(), "member".to_string()]);
    let member1 = AuthContext::with_roles("member1", vec!["member".to_string()]);
    let member2 = AuthContext::with_roles("member2", vec!["member".to_string()]);
    let observer = AuthContext::with_roles("observer1", vec!["observer".to_string()]);
    
    // Initialize storage accounts/namespaces (Resource accounts removed)
    storage.create_account(Some(&admin), "admin1", 1024 * 1024)?;
    storage.create_account(Some(&admin), "member1", 1024 * 1024)?; 
    storage.create_account(Some(&admin), "member2", 1024 * 1024)?;
    storage.create_namespace(Some(&admin), namespace, 1024 * 1024, None)?; 
    
    println!("Initialized storage with roles and resource accounts");
    
    // 1. Store member data with RBAC
    println!("\n=== Storing member data ===");
    
    let admin_member = Member {
        id: "admin1".to_string(),
        name: "Administrator".to_string(),
        reputation: 100,
    };
    
    let member1_data = Member {
        id: "member1".to_string(),
        name: "Alice".to_string(),
        reputation: 50,
    };
    
    let member2_data = Member {
        id: "member2".to_string(),
        name: "Bob".to_string(),
        reputation: 40,
    };
    
    // Store member data - admin can write to members namespace
    storage.set_json(
        &GovernanceNamespace::members(&admin_member.id),
        &admin_member
    )?;
    
    storage.set_json(
        &GovernanceNamespace::members(&member1_data.id),
        &member1_data
    )?;
    
    storage.set_json(
        &GovernanceNamespace::members(&member2_data.id),
        &member2_data
    )?;
    
    println!("Added three members to the system");
    
    // 2. Set up vote delegation (liquid democracy demo)
    println!("\n=== Setting up vote delegation ===");
    
    // Start a transaction for the delegation
    storage.begin_transaction()?;
    
    // Member2 delegates voting power to Member1
    let delegation_key = GovernanceNamespace::delegations("member2", "member1");
    storage.set_with_auth(&member2, &delegation_key, "full")?;
    
    println!("Member Bob has delegated voting power to Alice");
    
    // Commit the transaction
    storage.commit_transaction()?;
    
    // 3. Create a governance proposal
    println!("\n=== Creating governance proposal ===");
    
    let proposal = Proposal {
        id: "prop-001".to_string(),
        title: "Add support for credentials storage".to_string(),
        description: "Add a new namespace for storing member credentials and verification".to_string(),
        proposed_by: "admin1".to_string(),
        required_votes: 3,
        approve_threshold: 0.66, // 66% approval needed
        created_at: icn_covm::storage::now(),
    };
    
    // Start a transaction for the proposal
    storage.begin_transaction()?;
    
    // Store the proposal
    let proposal_key = GovernanceNamespace::proposals(&proposal.id);
    storage.set_with_auth(&admin, &proposal_key, &serde_json::to_string_pretty(&proposal)?)?;
    
    println!("Admin created proposal: {}", proposal.title);
    
    // Commit the transaction
    storage.commit_transaction()?;
    
    // 4. Demonstrate RBAC by testing observer's access attempt
    println!("\n=== Testing RBAC for observer ===");
    
    // Observer attempts to create a proposal (should fail)
    let observer_proposal = Proposal {
        id: "prop-002".to_string(),
        title: "Reduce member fees".to_string(),
        description: "Proposal to reduce membership fees by 20%".to_string(),
        proposed_by: "observer1".to_string(),
        required_votes: 3,
        approve_threshold: 0.5,
        created_at: icn_covm::storage::now(),
    };
    
    let result = storage.set_with_auth(
        &observer,
        &GovernanceNamespace::proposals(&observer_proposal.id),
        &serde_json::to_string_pretty(&observer_proposal)?
    );
    
    match result {
        Ok(_) => println!("Observer created proposal (unexpected)"),
        Err(e) => println!("Observer's attempt was denied: {}", e),
    }
    
    // 5. Cast votes using transactions
    println!("\n=== Casting votes ===");
    
    // Begin transaction for votes
    storage.begin_transaction()?;
    
    // Admin votes yes
    let admin_vote = Vote {
        voter: "admin1".to_string(),
        proposal_id: "prop-001".to_string(),
        approved: true,
        comment: Some("Necessary for secure identity verification".to_string()),
    };
    
    let admin_vote_key = GovernanceNamespace::votes(&proposal.id, "admin1");
    storage.set_with_auth(&admin, &admin_vote_key, &serde_json::to_string_pretty(&admin_vote)?)?;
    
    // Member1 votes yes (also representing member2 through delegation)
    let member1_vote = Vote {
        voter: "member1".to_string(),
        proposal_id: "prop-001".to_string(),
        approved: true,
        comment: None,
    };
    
    let member1_vote_key = GovernanceNamespace::votes(&proposal.id, "member1");
    storage.set_with_auth(&member1, &member1_vote_key, &serde_json::to_string_pretty(&member1_vote)?)?;
    
    // Commit the transaction with votes
    storage.commit_transaction()?;
    
    println!("Votes cast: admin (yes), Alice (yes), Bob (delegated to Alice)");
    
    // 6. Check version history
    println!("\n=== Checking version history ===");
    
    let versions = storage.list_versions(&proposal_key)?;
    println!("Proposal has {} versions", versions.len());
    for (i, version) in versions.iter().enumerate() {
        println!(
            "Version {}: Created by {} at timestamp {}",
            i + 1,
            version.author,
            version.timestamp
        );
    }
    
    // 7. Resource accounting
    println!("\n=== Resource accounting report ===");
    
    let admin_resources = storage.get_resource_account("admin1").unwrap();
    let member1_resources = storage.get_resource_account("member1").unwrap();
    let member2_resources = storage.get_resource_account("member2").unwrap();
    
    println!("Admin resource balance: {:.2}/{:.2}", admin_resources.balance, admin_resources.quota);
    println!("Alice resource balance: {:.2}/{:.2}", member1_resources.balance, member1_resources.quota);
    println!("Bob resource balance: {:.2}/{:.2}", member2_resources.balance, member2_resources.quota);
    
    // 8. Audit trail
    println!("\n=== Audit trail ===");
    
    let events = storage.get_audit_log();
    println!("Total events logged: {}", events.len());
    
    for (i, event) in events.iter().enumerate().take(5) {
        match event {
            StorageEvent::Access { key, action, user, timestamp } => {
                println!("{}: {} {} {} at {}", i, user, action, key, timestamp);
            }
            StorageEvent::Transaction { action, user, timestamp } => {
                println!("{}: {} transaction {} at {}", i, user, action, timestamp);
            }
            StorageEvent::ResourceUsage { account, amount, operation, timestamp } => {
                println!("{}: Account {} used {} resources for {} at {}", 
                    i, account, amount, operation, timestamp);
            }
        }
    }
    println!("... and {} more events", events.len().saturating_sub(5));
    
    // 9. Calculate final status of the proposal  
    println!("\n=== Proposal Status ===");
    
    // Get all votes
    let vote_keys = storage.list_keys(Some(&format!("governance/votes/{}/", proposal.id)));
    let mut yes_votes = 0;
    let mut total_votes = 0;
    
    for key in vote_keys {
        let vote: Vote = storage.get_json(&key)?;
        total_votes += 1;
        if vote.approved {
            yes_votes += 1;
            
            // Check for delegations to this voter
            let delegations = storage.list_keys(Some(&format!("governance/delegations/{}", vote.voter)));
            yes_votes += delegations.len() as u32;
            total_votes += delegations.len() as u32;
        }
    }
    
    let approval_rate = yes_votes as f64 / total_votes as f64;
    let approved = approval_rate >= proposal.approve_threshold && total_votes >= proposal.required_votes;
    
    println!(
        "Proposal status: {} ({} of {} votes, {:.1}% approval)",
        if approved { "APPROVED" } else { "PENDING" },
        yes_votes,
        total_votes,
        approval_rate * 100.0
    );
    
    println!("\nStorage example completed successfully!");
    Ok(())
} 