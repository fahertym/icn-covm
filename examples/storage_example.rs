use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;

// Import our storage types
use icn_covm::identity::Identity;
use icn_covm::storage::{
    AuthContext, InMemoryStorage, ResourceAccount, StorageBackend, StorageError, StorageEvent,
    StorageResult, VersionDiff, VersionInfo,
};

// Simple cooperative member structure
/* // Removed Member struct as it's not used after removing create_resource_account
#[derive(Serialize, Deserialize, Debug, Clone)]
struct Member {
    id: String,
    name: String,
    join_date: u64,
    roles: Vec<String>,
} */

// Simple proposal structure
#[derive(Serialize, Deserialize, Debug, Clone)] // Removed PartialEq due to Vec<VersionInfo>
struct Proposal {
    id: String,
    title: String,
    description: String,
    status: String,
    votes: HashMap<String, String>, // voter_id -> vote ("for", "against", "abstain")
    versions: Vec<VersionInfo>,
}

// Helper trait for JSON storage (optional, shows extension pattern)
trait JsonStorageHelper: StorageBackend {
    fn set_json<T: Serialize>(
        &mut self,
        auth: Option<&AuthContext>,
        namespace: &str,
        key: &str,
        value: &T,
    ) -> StorageResult<()>;
    fn get_json<T: for<'de> Deserialize<'de>>(
        &self,
        auth: Option<&AuthContext>,
        namespace: &str,
        key: &str,
    ) -> StorageResult<T>;
}

impl<S: StorageBackend> JsonStorageHelper for S {
    fn set_json<T: Serialize>(
        &mut self,
        auth: Option<&AuthContext>,
        namespace: &str,
        key: &str,
        value: &T,
    ) -> StorageResult<()> {
        let json = serde_json::to_vec(value).map_err(|e| StorageError::SerializationError {
            details: e.to_string(),
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
        serde_json::from_slice(&bytes).map_err(|e| StorageError::SerializationError {
            details: e.to_string(),
        })
    }
}

fn main() -> StorageResult<()> {
    // Use StorageResult for consistency
    println!("--- Storage Example ---");

    // Create storage instance
    let mut storage = InMemoryStorage::new();
    let namespace = "default"; // Define a namespace

    // Set up auth contexts
    let admin = AuthContext::with_roles("admin", vec!["admin".to_string()]);
    let member1 = AuthContext::with_roles("alice", vec!["member".to_string()]);
    let member2 = AuthContext::with_roles("bob", vec!["member".to_string()]);
    let _observer = AuthContext::with_roles("observer", vec!["observer".to_string()]); // Prefixed as unused later

    // Initialize storage - Create accounts and namespaces (required for InMemoryStorage)
    storage.create_account(Some(&admin), "admin", 1024 * 1024)?; // Use admin auth
    storage.create_account(Some(&admin), "alice", 1024 * 1024)?; // Use admin auth
    storage.create_account(Some(&admin), "bob", 1024 * 1024)?; // Use admin auth
    storage.create_account(Some(&admin), "observer", 1024 * 1024)?; // Use admin auth
    storage.create_namespace(Some(&admin), namespace, 1024 * 1024, None)?; // Use admin auth

    // 3. Basic Set/Get Operations
    println!("\n--- Basic Operations ---");
    storage.set(
        Some(&admin),
        namespace,
        "config/site_name",
        "My Coop Platform".as_bytes().to_vec(),
    )?;
    let site_name_bytes = storage.get(Some(&admin), namespace, "config/site_name")?;
    let site_name = String::from_utf8(site_name_bytes).unwrap_or_default();
    println!("Site Name: {}", site_name);

    // 4. Using JSON Helper for Structured Data
    println!("\n--- JSON Operations ---");
    let proposal = Proposal {
        id: "prop-001".to_string(),
        title: "Adopt New Bylaws".to_string(),
        description: "Proposal to update cooperative bylaws...".to_string(),
        status: "open".to_string(),
        votes: HashMap::new(),
        versions: vec![],
    };
    storage.set_json(
        Some(&admin),
        namespace,
        "governance/proposals/prop-001",
        &proposal,
    )?;

    let retrieved_prop: Proposal =
        storage.get_json(Some(&admin), namespace, "governance/proposals/prop-001")?;
    println!("Retrieved Proposal: {:?}", retrieved_prop);
    // assert_eq!(proposal, retrieved_prop); // Cannot compare due to removed PartialEq

    // (set_with_auth seems removed, using regular set)
    // Storing member data might require a different approach now or isn't needed for this example

    // 5. Versioning
    println!("\n--- Versioning ---");

    // Update the proposal (implicitly creates version 2)
    let mut updated_prop = proposal.clone();
    updated_prop.status = "voting".to_string();
    storage.set_json(
        Some(&member1),
        namespace,
        "governance/proposals/prop-001",
        &updated_prop,
    )?;

    // Update again (implicitly creates version 3)
    updated_prop.status = "closed".to_string();
    updated_prop
        .votes
        .insert("alice".to_string(), "for".to_string());
    storage.set_json(
        Some(&member1),
        namespace,
        "governance/proposals/prop-001",
        &updated_prop,
    )?;

    // List versions
    println!("Listing versions for prop-001:");
    match storage.list_versions(Some(&admin), namespace, "governance/proposals/prop-001") {
        Ok(versions) => {
            for version in versions {
                println!(
                    "  - Version: {}, Created By: {}, Timestamp: {}",
                    version.version, version.created_by, version.timestamp
                ); // Use created_by
            }
        }
        Err(e) => println!("Error listing versions: {}", e),
    }

    // Get a specific version
    // Assuming get_version_as exists and takes auth/namespace (needs definition or implementation)
    // let version1: Proposal = storage.get_version_as(Some(&admin), namespace, "governance/proposals/prop-001", 1)?;
    // For now, let's get raw bytes and deserialize manually for version 1
    let (version1_bytes, _) =
        storage.get_version(Some(&admin), namespace, "governance/proposals/prop-001", 1)?;
    let version1: Proposal =
        serde_json::from_slice(&version1_bytes).map_err(|e| StorageError::SerializationError {
            details: e.to_string(),
        })?;
    println!("Version 1 Status: {}", version1.status);
    assert_eq!(version1.status, "open");

    // Get the latest version
    // Assuming get_latest_as exists and takes auth/namespace
    // let latest_version: Proposal = storage.get_latest_as(Some(&admin), namespace, "governance/proposals/prop-001")?;
    // Get raw bytes for latest and deserialize
    let latest_bytes = storage.get(Some(&admin), namespace, "governance/proposals/prop-001")?;
    let latest_version: Proposal =
        serde_json::from_slice(&latest_bytes).map_err(|e| StorageError::SerializationError {
            details: e.to_string(),
        })?;
    println!("Latest Version Status: {}", latest_version.status);
    assert_eq!(latest_version.status, "closed");

    // 6. Audit Log (assuming StorageExtensions is implemented)
    println!("\n--- Audit Log ---");
    // Add some vote data first
    storage.set(
        Some(&member1),
        namespace,
        "governance/votes/prop-001/alice",
        b"for".to_vec(),
    )?;
    storage.set(
        Some(&member2),
        namespace,
        "governance/votes/prop-001/bob",
        b"delegated:alice".to_vec(),
    )?;

    // Retrieve audit log
    match storage.get_audit_log(Some(&admin), Some(namespace), None, 10) {
        // Pass auth, namespace
        Ok(events) => {
            println!("Last {} audit events:", events.len());
            for (i, event) in events.iter().enumerate() {
                print!("  {}: ", i + 1);
                match event {
                    icn_covm::storage::StorageEvent::Set {
                        key,
                        user,
                        timestamp,
                        ..
                    } => {
                        println!(
                            "Set key '{}' by '{}' at {}",
                            key,
                            user.as_deref().unwrap_or("unknown"),
                            timestamp
                        );
                    }
                    icn_covm::storage::StorageEvent::Delete {
                        key,
                        user,
                        timestamp,
                        ..
                    } => {
                        println!(
                            "Delete key '{}' by '{}' at {}",
                            key,
                            user.as_deref().unwrap_or("unknown"),
                            timestamp
                        );
                    }
                    icn_covm::storage::StorageEvent::Transaction {
                        action,
                        user,
                        timestamp,
                        ..
                    } => {
                        println!(
                            "Transaction '{}' by '{}' at {}",
                            action,
                            user.as_deref().unwrap_or("unknown"),
                            timestamp
                        );
                    } // Add other event types if needed
                }
            }
            if events.len() > 5 {
                println!("... and {} more events", events.len().saturating_sub(5));
            }
        }
        Err(e) => println!("Error retrieving audit log: {}", e),
    }

    // 7. Complex Example: Tallying Votes (Simplified)
    println!("\n--- Vote Tally Example ---");
    let mut yes_votes = 0u32;
    let mut no_votes = 0u32;
    let mut total_votes = 0u32;

    let proposal_key = "governance/proposals/prop-001";
    let final_prop: Proposal = storage.get_json(Some(&admin), namespace, proposal_key)?;

    // List keys matching the vote pattern
    let vote_prefix = format!("governance/votes/{}/", final_prop.id);
    let vote_keys_result = storage.list_keys(Some(&admin), namespace, Some(&vote_prefix)); // Pass auth, namespace

    if let Ok(vote_keys) = vote_keys_result {
        println!(
            "Found {} votes/delegations for proposal {}",
            vote_keys.len(),
            final_prop.id
        );
        for key in vote_keys {
            if let Ok(vote_bytes) = storage.get(Some(&admin), namespace, &key) {
                // Pass auth, namespace
                let vote_str = String::from_utf8(vote_bytes).unwrap_or_default();
                if vote_str == "for" {
                    yes_votes += 1;
                    total_votes += 1;
                } else if vote_str == "against" {
                    no_votes += 1;
                    total_votes += 1;
                } else if vote_str.starts_with("delegated:") {
                    // Basic delegation handling (doesn't resolve chains)
                    println!("  (Vote from {} is delegated)", key);
                }
            }
        }
    }

    println!(
        "Vote Results for '{}': Yes: {}, No: {}, Total Direct: {}",
        final_prop.title, yes_votes, no_votes, total_votes
    );

    println!("\n--- Storage Example Complete ---");
    Ok(())
}
