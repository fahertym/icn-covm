use serde::{Deserialize, Serialize};
use std::error::Error;

// Import our storage types
use icn_covm::storage::{
    AuthContext, InMemoryStorage, ResourceAccount, StorageBackend, StorageError, StorageEvent,
    StorageResult, VersionInfo,
};

// Import necessary types
use icn_covm::identity::{Identity, MemberProfile};

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
        let json = serde_json::to_vec(value).map_err(|e| {
            icn_covm::storage::StorageError::SerializationError {
                details: e.to_string(),
            }
        })?;
        self.set(auth, namespace, key, json)
    }

    fn get_json<T: for<'de> Deserialize<'de>>(
        &self,
        auth: Option<&AuthContext>,
        namespace: &str,
        key: &str,
    ) -> StorageResult<T> {
        let bytes = self.get(auth, namespace, key)?;
        serde_json::from_slice(&bytes).map_err(|e| {
            icn_covm::storage::StorageError::SerializationError {
                details: e.to_string(),
            }
        })
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

    let admin_auth = AuthContext::new(admin_id);
    let member1_auth = AuthContext::new(member1_id);
    let member2_auth = AuthContext::new(member2_id);

    // Initialize storage accounts/namespaces
    storage.create_account(Some(&admin_auth), admin_id, 1024 * 1024)?;
    storage.create_account(Some(&admin_auth), member1_id, 1024 * 1024)?;
    storage.create_account(Some(&admin_auth), member2_id, 1024 * 1024)?;
    storage.create_namespace(Some(&admin_auth), namespace, 1024 * 1024, None)?;

    // 3. Store Member Profiles (using admin privileges)
    let admin_profile =
        MemberProfile::new(Identity::new(admin_id, "admin"), icn_covm::storage::now());
    storage.set_json_with_resources(
        Some(&admin_auth),
        namespace,
        &format!("members/{}", admin_id),
        &admin_profile,
    )?;
    let member1_profile = MemberProfile::new(
        Identity::new(member1_id, "member"),
        icn_covm::storage::now(),
    );
    storage.set_json_with_resources(
        Some(&admin_auth),
        namespace,
        &format!("members/{}", member1_id),
        &member1_profile,
    )?;
    let member2_profile = MemberProfile::new(
        Identity::new(member2_id, "member"),
        icn_covm::storage::now(),
    );
    storage.set_json_with_resources(
        Some(&admin_auth),
        namespace,
        &format!("members/{}", member2_id),
        &member2_profile,
    )?;

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
    storage.set_json_with_resources(
        Some(&member1_auth),
        namespace,
        "proposals/prop-001",
        &proposal,
    )?;
    println!(
        "Proposal '{}' created by {}",
        proposal.title, proposal.creator
    );

    // 5. Submit Votes
    // Member 1 votes FOR
    let vote1 = Vote {
        voter: member1_id.to_string(),
        choice: "for".to_string(),
        reason: None,
        timestamp: icn_covm::storage::now(),
    };
    storage.set_json_with_resources(
        Some(&member1_auth),
        namespace,
        "votes/prop-001/member_1",
        &vote1,
    )?;
    println!("Vote cast by {}", member1_id);

    // Member 2 votes AGAINST
    let vote2 = Vote {
        voter: member2_id.to_string(),
        choice: "against".to_string(),
        reason: Some("Budget is fine".to_string()),
        timestamp: icn_covm::storage::now(),
    };
    let vote_result = storage.set_json_with_resources(
        Some(&member2_auth),
        namespace,
        "votes/prop-001/member_2",
        &vote2,
    );

    if let Err(e) = vote_result {
        println!("Error submitting vote: {}", e);
    }
    println!("Vote cast by {}", member2_id);

    // 6. Tally Votes & Update Proposal Status (by admin)
    let mut prop_data: ProposalData =
        storage.get_json(Some(&admin_auth), namespace, "proposals/prop-001")?;

    // (Simplified tally - assumes direct votes exist as keys)
    let votes_for = storage
        .contains(Some(&admin_auth), namespace, "votes/prop-001/member_1")?
        .then_some(1)
        .unwrap_or(0);
    let votes_against = storage
        .contains(Some(&admin_auth), namespace, "votes/prop-001/member_2")?
        .then_some(1)
        .unwrap_or(0);

    prop_data.votes_for = votes_for;
    prop_data.votes_against = votes_against;

    // Simple majority rule
    if prop_data.votes_for > prop_data.votes_against {
        prop_data.status = "approved".to_string();
        println!("Proposal approved!");
    } else {
        prop_data.status = "rejected".to_string();
        println!("Proposal rejected.");
    }

    storage.set_json_with_resources(
        Some(&admin_auth),
        namespace,
        "proposals/prop-001",
        &prop_data,
    )?;

    storage.set(
        Some(&admin_auth),
        namespace,
        "config/last_proposal_result",
        prop_data.status.as_bytes().to_vec(),
    )?;

    // 7. Check Audit Log (optional)
    println!("\n--- Audit Log Snippet ---");
    match storage.get_audit_log(Some(&admin_auth), Some(namespace), None, 5) {
        Ok(events) => {
            for event in events {
                match event {
                    icn_covm::storage::StorageEvent::Set {
                        key,
                        user,
                        timestamp,
                        ..
                    } => {
                        println!("  Set '{}' by {}", key, user.as_deref().unwrap_or("anon"));
                    }
                    icn_covm::storage::StorageEvent::Delete { key, user, .. } => {
                        println!(
                            "  Delete '{}' by {}",
                            key,
                            user.as_deref().unwrap_or("anon")
                        );
                    }
                    icn_covm::storage::StorageEvent::Transaction {
                        action,
                        user,
                        timestamp,
                        ..
                    } => {
                        println!(
                            "  Tx '{}' by {} @ {}",
                            action,
                            user.as_deref().unwrap_or("anon"),
                            timestamp
                        );
                    }
                    _ => {} // Ignore other event types
                }
            }
        }
        Err(e) => println!("Error getting audit log: {}", e),
    }

    // 8. List Versions
    println!("\n--- Proposal Versions ---");
    match storage.list_versions(Some(&admin_auth), namespace, "proposals/prop-001") {
        Ok(versions) => {
            for version in versions {
                println!(
                    "  Version: {}, By: {}, At: {}",
                    version.version, version.created_by, version.timestamp
                );
            }
        }
        Err(e) => println!("Error listing versions: {}", e),
    }

    println!("\n--- Cooperative Governance Example Complete ---");
    Ok(())
}
