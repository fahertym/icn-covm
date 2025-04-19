use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error as StdError;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

// --- SIMPLIFIED VERSION OF STORAGE MODULE FOR DEMO PURPOSES ---

/// Type alias for a timestamp (milliseconds since Unix epoch)
type Timestamp = u64;

/// Get the current timestamp
fn now() -> Timestamp {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Authentication context for storage operations
#[derive(Debug, Clone, PartialEq)]
struct AuthContext {
    /// The ID of the caller
    pub caller: String,

    /// Roles associated with the caller
    pub roles: Vec<String>,

    /// Timestamp of the request
    pub timestamp: Timestamp,
}

impl AuthContext {
    /// Create a new authentication context with roles
    pub fn with_roles(caller: &str, roles: Vec<String>) -> Self {
        Self {
            caller: caller.to_string(),
            roles,
            timestamp: now(),
        }
    }

    /// Check if the caller has a specific role
    pub fn has_role(&self, role: &str) -> bool {
        self.roles.contains(&role.to_string())
    }
}

/// Resource account for tracking storage usage
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct ResourceAccount {
    /// Account identifier
    pub id: String,

    /// Current resource balance
    pub balance: f64,

    /// Maximum allowed usage (quota)
    pub quota: f64,

    /// Usage history for auditing
    pub usage_history: Vec<(Timestamp, f64, String)>,
}

impl ResourceAccount {
    /// Create a new resource account with the given quota
    pub fn new(id: &str, quota: f64) -> Self {
        Self {
            id: id.to_string(),
            balance: quota,
            quota,
            usage_history: Vec::new(),
        }
    }

    /// Deduct resources and record the operation
    pub fn deduct(&mut self, amount: f64, operation: &str) -> bool {
        if self.balance >= amount {
            self.balance -= amount;
            self.usage_history
                .push((now(), amount, operation.to_string()));
            true
        } else {
            false
        }
    }
}

/// Version information for stored data
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct VersionInfo {
    /// Version number (increments with each update)
    pub version: usize,

    /// Timestamp of when this version was created
    pub timestamp: Timestamp,

    /// ID of the user who created this version
    pub author: String,
}

/// Error types for storage operations
#[derive(Debug, Clone, PartialEq)]
enum StorageError {
    /// Key not found in storage
    KeyNotFound(String),

    /// Error accessing the storage backend
    AccessError(String),

    /// Permission denied for the requested operation
    PermissionDenied(String),

    /// Transaction-related error
    TransactionError(String),

    /// Resource quota exceeded
    QuotaExceeded(String),
}

// Implement Display for StorageError
impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageError::KeyNotFound(key) => write!(f, "Key not found: {}", key),
            StorageError::AccessError(msg) => write!(f, "Storage access error: {}", msg),
            StorageError::PermissionDenied(msg) => write!(f, "Permission denied: {}", msg),
            StorageError::TransactionError(msg) => write!(f, "Transaction error: {}", msg),
            StorageError::QuotaExceeded(msg) => write!(f, "Resource quota exceeded: {}", msg),
        }
    }
}

// Implement Error for StorageError
impl StdError for StorageError {}

/// Result type for storage operations
type StorageResult<T> = Result<T, StorageError>;

/// Storage event for audit logging
#[derive(Debug, Clone, PartialEq)]
enum StorageEvent {
    /// Access to a storage key
    Access {
        /// The key being accessed
        key: String,

        /// The type of access (get, set, delete)
        action: String,

        /// The ID of the user performing the action
        user: String,

        /// The timestamp of the action
        timestamp: Timestamp,
    },

    /// Transaction operation
    Transaction {
        /// The type of transaction operation (begin, commit, rollback)
        action: String,

        /// The ID of the user performing the action
        user: String,

        /// The timestamp of the action
        timestamp: Timestamp,
    },
}

/// Namespace helper
struct GovernanceNamespace;

impl GovernanceNamespace {
    /// Create a key in the proposals namespace
    pub fn proposals(proposal_id: &str) -> String {
        format!("governance/proposals/{}", proposal_id)
    }

    /// Create a key in the votes namespace
    pub fn votes(proposal_id: &str, voter_id: &str) -> String {
        format!("governance/votes/{}/{}", proposal_id, voter_id)
    }

    /// Create a key in the members namespace
    pub fn members(member_id: &str) -> String {
        format!("governance/members/{}", member_id)
    }

    /// Create a key in the delegations namespace
    pub fn delegations(from: &str, to: &str) -> String {
        format!("governance/delegations/{}/{}", from, to)
    }
}

/// In-memory implementation of storage
#[derive(Debug, Clone, Default)]
struct InMemoryStorage {
    data: HashMap<String, String>,
    transaction_data: Option<HashMap<String, String>>,
    versioned_data: HashMap<String, Vec<(VersionInfo, String)>>,
    audit_log: Vec<StorageEvent>,
    resource_accounts: HashMap<String, ResourceAccount>,
}

impl InMemoryStorage {
    /// Create a new empty in-memory storage
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
            transaction_data: None,
            versioned_data: HashMap::new(),
            audit_log: Vec::new(),
            resource_accounts: HashMap::new(),
        }
    }

    /// Get a value from storage
    pub fn get(&self, key: &str) -> StorageResult<String> {
        // If in a transaction, check transaction data first
        if let Some(transaction) = &self.transaction_data {
            if let Some(value) = transaction.get(key) {
                return Ok(value.clone());
            }
        }

        // Otherwise check main data
        self.data
            .get(key)
            .cloned()
            .ok_or_else(|| StorageError::KeyNotFound(key.to_string()))
    }

    /// Set a value in storage
    pub fn set(&mut self, key: &str, value: &str) -> StorageResult<()> {
        // If in a transaction, store in transaction data
        if let Some(transaction) = &mut self.transaction_data {
            transaction.insert(key.to_string(), value.to_string());
        } else {
            // Otherwise store in main data
            self.data.insert(key.to_string(), value.to_string());
        }
        Ok(())
    }

    /// Delete a value from storage
    pub fn delete(&mut self, key: &str) -> StorageResult<()> {
        // If in a transaction, mark deletion in transaction
        if let Some(transaction) = &mut self.transaction_data {
            transaction.remove(key);
        } else if self.data.remove(key).is_none() {
            return Err(StorageError::KeyNotFound(key.to_string()));
        }
        Ok(())
    }

    /// Check if a key exists in storage
    pub fn contains(&self, key: &str) -> bool {
        // If in a transaction, check transaction data first
        if let Some(transaction) = &self.transaction_data {
            if transaction.contains_key(key) {
                return true;
            }
        }

        // Otherwise check main data
        self.data.contains_key(key)
    }

    /// List all keys in storage with given prefix
    pub fn list_keys(&self, prefix: Option<&str>) -> Vec<String> {
        let mut keys = Vec::new();

        // Get keys from base storage first
        for key in self.data.keys() {
            if let Some(prefix_str) = prefix {
                if key.starts_with(prefix_str) {
                    keys.push(key.clone());
                }
            } else {
                keys.push(key.clone());
            }
        }

        keys
    }

    /// Begin a transaction
    pub fn begin_transaction(&mut self) -> StorageResult<()> {
        if self.transaction_data.is_some() {
            return Err(StorageError::TransactionError(
                "Transaction already in progress".to_string(),
            ));
        }
        self.transaction_data = Some(HashMap::new());

        // Log transaction begin
        self.audit_log.push(StorageEvent::Transaction {
            action: "begin".to_string(),
            user: "system".to_string(),
            timestamp: now(),
        });

        Ok(())
    }

    /// Commit the current transaction
    pub fn commit_transaction(&mut self) -> StorageResult<()> {
        if let Some(transaction) = self.transaction_data.take() {
            // Apply changes to main data
            for (key, value) in transaction {
                self.data.insert(key, value);
            }

            // Log transaction commit
            self.audit_log.push(StorageEvent::Transaction {
                action: "commit".to_string(),
                user: "system".to_string(),
                timestamp: now(),
            });

            Ok(())
        } else {
            Err(StorageError::TransactionError(
                "No transaction in progress".to_string(),
            ))
        }
    }

    /// Rollback the current transaction
    pub fn rollback_transaction(&mut self) -> StorageResult<()> {
        if self.transaction_data.is_some() {
            self.transaction_data = None;

            // Log transaction rollback
            self.audit_log.push(StorageEvent::Transaction {
                action: "rollback".to_string(),
                user: "system".to_string(),
                timestamp: now(),
            });

            Ok(())
        } else {
            Err(StorageError::TransactionError(
                "No transaction in progress".to_string(),
            ))
        }
    }

    /// Get a value with authorization check
    pub fn get_with_auth(&mut self, auth: &AuthContext, key: &str) -> StorageResult<String> {
        // Implement RBAC checks
        if key.starts_with("governance/") {
            // Governance data requires admin or member role
            if !auth.has_role("admin") && !auth.has_role("member") {
                return Err(StorageError::PermissionDenied(format!(
                    "Access to governance data requires admin or member role"
                )));
            }
        }

        // Log the access
        self.audit_log.push(StorageEvent::Access {
            key: key.to_string(),
            action: "get".to_string(),
            user: auth.caller.clone(),
            timestamp: auth.timestamp,
        });

        // Call the normal get
        self.get(key)
    }

    /// Set a value with authorization check
    pub fn set_with_auth(
        &mut self,
        auth: &AuthContext,
        key: &str,
        value: &str,
    ) -> StorageResult<()> {
        // Implement RBAC checks
        if key.starts_with("governance/") {
            // Governance data requires admin role
            if !auth.has_role("admin") {
                return Err(StorageError::PermissionDenied(format!(
                    "Writing to governance data requires admin role"
                )));
            }
        }

        // Log the access
        self.audit_log.push(StorageEvent::Access {
            key: key.to_string(),
            action: "set".to_string(),
            user: auth.caller.clone(),
            timestamp: auth.timestamp,
        });

        // Add a version
        let versions = self.versioned_data.entry(key.to_string()).or_default();
        let version = versions.len() + 1;

        let version_info = VersionInfo {
            version,
            timestamp: now(),
            author: auth.caller.clone(),
        };

        versions.push((version_info, value.to_string()));

        // Call the normal set
        self.set(key, value)
    }

    /// Get a versioned value
    pub fn list_versions(&self, key: &str) -> StorageResult<Vec<VersionInfo>> {
        let versions = self
            .versioned_data
            .get(key)
            .ok_or_else(|| StorageError::KeyNotFound(key.to_string()))?;

        let version_infos = versions.iter().map(|(info, _)| info.clone()).collect();

        Ok(version_infos)
    }

    /// Get the audit log
    pub fn get_audit_log(&self) -> &[StorageEvent] {
        &self.audit_log
    }

    /// Create a resource account
    pub fn create_resource_account(&mut self, id: &str, quota: f64) -> &mut ResourceAccount {
        self.resource_accounts
            .entry(id.to_string())
            .or_insert_with(|| ResourceAccount::new(id, quota))
    }

    /// Get a resource account
    pub fn get_resource_account(&self, id: &str) -> Option<&ResourceAccount> {
        self.resource_accounts.get(id)
    }
}

// --- EXAMPLE STRUCTURES ---

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

// --- MAIN FUNCTION ---

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== ICN-COVM Simplified Cooperative Storage Example ===");

    // Initialize storage
    let mut storage = InMemoryStorage::new();

    // Create user roles
    let admin = AuthContext::with_roles("admin1", vec!["admin".to_string(), "member".to_string()]);
    let member1 = AuthContext::with_roles("member1", vec!["member".to_string()]);
    let member2 = AuthContext::with_roles("member2", vec!["member".to_string()]);
    let observer = AuthContext::with_roles("observer1", vec!["observer".to_string()]);

    // Create resource accounts
    let _admin_account = storage.create_resource_account("admin1", 1000.0);
    let _member1_account = storage.create_resource_account("member1", 500.0);
    let _member2_account = storage.create_resource_account("member2", 500.0);

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
    let member_key = GovernanceNamespace::members(&admin_member.id);
    let admin_json = serde_json::to_string_pretty(&admin_member)?;
    storage.set_with_auth(&admin, &member_key, &admin_json)?;

    let member_key = GovernanceNamespace::members(&member1_data.id);
    let member1_json = serde_json::to_string_pretty(&member1_data)?;
    storage.set_with_auth(&admin, &member_key, &member1_json)?;

    let member_key = GovernanceNamespace::members(&member2_data.id);
    let member2_json = serde_json::to_string_pretty(&member2_data)?;
    storage.set_with_auth(&admin, &member_key, &member2_json)?;

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
        description: "Add a new namespace for storing member credentials and verification"
            .to_string(),
        proposed_by: "admin1".to_string(),
        required_votes: 3,
        approve_threshold: 0.66, // 66% approval needed
    };

    // Start a transaction for the proposal
    storage.begin_transaction()?;

    // Store the proposal
    let proposal_key = GovernanceNamespace::proposals(&proposal.id);
    let proposal_json = serde_json::to_string_pretty(&proposal)?;
    storage.set_with_auth(&admin, &proposal_key, &proposal_json)?;

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
    };

    let obs_proposal_key = GovernanceNamespace::proposals(&observer_proposal.id);
    let obs_proposal_json = serde_json::to_string_pretty(&observer_proposal)?;

    let result = storage.set_with_auth(&observer, &obs_proposal_key, &obs_proposal_json);

    match result {
        Ok(_) => println!("Observer created proposal (unexpected)"),
        Err(e) => println!("Observer's attempt was denied: {:?}", e),
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
    let admin_vote_json = serde_json::to_string_pretty(&admin_vote)?;
    storage.set_with_auth(&admin, &admin_vote_key, &admin_vote_json)?;

    // Member1 votes yes (also representing member2 through delegation)
    let member1_vote = Vote {
        voter: "member1".to_string(),
        proposal_id: "prop-001".to_string(),
        approved: true,
        comment: None,
    };

    let member1_vote_key = GovernanceNamespace::votes(&proposal.id, "member1");
    let member1_vote_json = serde_json::to_string_pretty(&member1_vote)?;
    storage.set_with_auth(&member1, &member1_vote_key, &member1_vote_json)?;

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

    println!(
        "Admin resource balance: {:.2}/{:.2}",
        admin_resources.balance, admin_resources.quota
    );
    println!(
        "Alice resource balance: {:.2}/{:.2}",
        member1_resources.balance, member1_resources.quota
    );
    println!(
        "Bob resource balance: {:.2}/{:.2}",
        member2_resources.balance, member2_resources.quota
    );

    // 8. Audit trail
    println!("\n=== Audit trail ===");

    let events = storage.get_audit_log();
    println!("Total events logged: {}", events.len());

    for (i, event) in events.iter().enumerate() {
        match event {
            StorageEvent::Access {
                key,
                action,
                user,
                timestamp,
            } => {
                println!("{}: {} {} {} at {}", i, user, action, key, timestamp);
            }
            StorageEvent::Transaction {
                action,
                user,
                timestamp,
            } => {
                println!("{}: {} transaction {} at {}", i, user, action, timestamp);
            }
        }
    }

    // 9. Calculate final status of the proposal
    println!("\n=== Proposal Status ===");

    // Get all votes
    let vote_keys = storage.list_keys(Some(&format!("governance/votes/{}/", proposal.id)));
    let mut yes_votes = 0;
    let mut total_votes = 0;

    for key in vote_keys {
        let vote_json = storage.get(&key)?;
        let vote: Vote = serde_json::from_str(&vote_json)?;
        total_votes += 1;
        if vote.approved {
            yes_votes += 1;

            // Check for delegations to this voter
            let delegations =
                storage.list_keys(Some(&format!("governance/delegations/{}", vote.voter)));
            yes_votes += delegations.len() as u32;
            total_votes += delegations.len() as u32;
        }
    }

    let approval_rate = yes_votes as f64 / total_votes as f64;
    let approved =
        approval_rate >= proposal.approve_threshold && total_votes >= proposal.required_votes;

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
