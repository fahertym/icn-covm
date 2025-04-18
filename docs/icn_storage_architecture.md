# ICN Persistent Storage Architecture

## 1. Overview & Cooperative Principles

The ICN-COVM storage system is designed with cooperative governance principles at its core. The storage layer isn't merely a technical component - it's a fundamental enabler of cooperative operations, transparent governance, and democratic decision-making.

The persistent storage system follows these core cooperative principles:

- **Democratic Member Control**: Storage operations enforce access controls based on member roles and permissions
- **Member Economic Participation**: Resource usage tracking and quota enforcement ensure fair distribution of resources
- **Autonomy and Independence**: Decentralized storage backends with federation support
- **Education, Training, Information**: Human-readable, well-structured data formats with versioning
- **Cooperation Among Cooperatives**: Federation and synchronization features for inter-cooperative collaboration
- **Concern for Community**: Audit logs and transparent governance operations

## 2. Namespace Structure

The storage system organizes cooperative data in clearly defined namespaces:

```
governance/
  ├── proposals/               # Cooperative proposals
  │    ├── [proposal_id]/      # Individual proposal data
  │    │    ├── status         # Status (approved, rejected, pending)
  │    │    ├── approved_at    # Approval timestamp
  │    │    └── ...            # Other proposal metadata
  ├── votes/                   # Votes on proposals
  │    ├── [proposal_id]/      # Votes for a specific proposal
  │    │    ├── [member_id]    # Individual member votes
  ├── delegations/             # Liquid democracy delegations
  │    ├── [from_member]/      # Delegations from this member
  │    │    ├── [to_member]    # Member receiving the delegation
  ├── members/                 # Member registry
  │    ├── [member_id]/        # Individual member data
  │    │    ├── roles          # Member roles
  │    │    ├── voting_power   # Voting power
  │    │    └── ...            # Other member metadata
  └── config/                  # Governance configuration
       ├── quorum_threshold    # Default quorum threshold
       ├── vote_threshold      # Default vote threshold
       └── ...                 # Other governance parameters
```

This namespace structure ensures that governance data is organized systematically and can be accessed with appropriate permissions.

## 3. Multi-stakeholder Authorization Model

The authorization model is built around clearly defined roles:

- **admin**: Can perform all operations, including creating proposals, modifying configuration, etc.
- **member**: Can vote on proposals, delegate voting power, access governance data.
- **observer**: Read-only access to non-sensitive governance data.

The storage system now uses an identity-aware API pattern with `Option<&AuthContext>` for all operations, allowing for flexible authentication handling:

```rust
fn get(&self, auth: Option<&AuthContext>, namespace: &str, key: &str) -> StorageResult<Vec<u8>>;
fn set(&mut self, auth: Option<&AuthContext>, namespace: &str, key: &str, value: &[u8]) -> StorageResult<()>;
fn delete(&mut self, auth: Option<&AuthContext>, namespace: &str, key: &str) -> StorageResult<()>;
```

This design allows operations to be performed with or without authentication context, while ensuring proper permission checks when auth is provided.

## 4. Decentralization & Federation Support

The storage system is designed to support decentralized and federated operations through the `FederatedStorageBackend` trait:

```rust
pub trait FederatedStorageBackend: StorageBackend {
    fn synchronize(&mut self, remote: &dyn StorageBackend) -> StorageResult<()>;
    fn push(&self, remote: &mut dyn StorageBackend) -> StorageResult<()>;
    fn pull(&mut self, remote: &dyn StorageBackend) -> StorageResult<()>;
    fn resolve_conflicts(&mut self, remote: &dyn StorageBackend) -> StorageResult<()>;
}
```

This trait enables:
- Synchronization between cooperative nodes
- Push/pull operations for data sharing
- Conflict resolution for concurrent changes

## 5. Economic & Resource Accounting

The storage system includes resource accounting features through the `ResourceAccount` structure:

```rust
pub struct ResourceAccount {
    pub id: String,       // Account identifier
    pub balance: f64,     // Current resource balance
    pub quota: f64,       // Maximum allowed usage
    pub usage_history: Vec<(Timestamp, f64, String)>, // Usage history
}
```

This enables:
- Enforcement of resource quotas
- Fair distribution of storage resources
- Tracking and auditing of resource usage
- Foundation for economic operations (e.g., resource contributions)

## 6. Transactional Guarantees & Atomicity

All storage backends must implement transaction support:

```rust
fn begin_transaction(&mut self) -> StorageResult<()>;
fn commit_transaction(&mut self) -> StorageResult<()>;
fn rollback_transaction(&mut self) -> StorageResult<()>;
```

This ensures atomic operations for critical governance activities like:
- Proposal creation and voting
- Multi-step governance processes
- Safe concurrent operations

## 7. Audit Logging & Accountability

The storage system includes robust audit logging through the `StorageEvent` enum:

```rust
pub enum StorageEvent {
    Access { key: String, action: String, user: String, timestamp: u64 },
    Transaction { action: String, user: String, timestamp: u64 },
    ResourceUsage { account: String, amount: f64, operation: String, timestamp: u64 },
}
```

Every operation is logged with:
- Who performed the action
- What was accessed/modified
- When the action occurred
- Any resources consumed

This creates a transparent, auditable trail of all governance operations.

## 8. Data Versioning & History

The storage system maintains versioned history of important governance data:

```rust
pub struct VersionInfo {
    pub version: usize,    // Version number
    pub timestamp: u64,    // Creation timestamp
    pub author: String,    // Who created this version
    pub comment: Option<String>, // Optional comment
}
```

This enables:
- Reviewing the history of governance decisions
- Accountability for changes
- Rollback of erroneous changes
- Comprehensive governance audit trails

## 9. Human-Readable Data

The storage system encourages structured, human-readable data formats through JSON serialization helpers:

```rust
fn set_json<T: Serialize>(&mut self, key: &str, value: &T) -> StorageResult<()>;
fn get_json<T: for<'de> Deserialize<'de>>(&self, key: &str) -> StorageResult<T>;
```

This promotes:
- Data portability
- Easier inspection by cooperative members
- Integration with external tools
- Long-term sustainability of data

## 10. Implementation & Storage Backends

The storage architecture currently includes:

1. **InMemoryStorage**: For testing and development
2. **FileStorage**: Robust file-based persistence with:
   - File locking for concurrent access safety
   - Improved contextual error handling
   - Atomic operations through transaction support
   - Directory structure mirroring namespaces

The FileStorage backend has been significantly improved in v0.6.x with:
- File locking using the `fs2` crate to prevent concurrent modification issues
- Enhanced error handling with additional context for debugging
- Robust transaction implementation with journaling for atomic operations
- Improved testing for concurrent access scenarios
- Directory structure validation and automatic creation

Future planned backends include:
- **DatabaseStorage**: SQL database backend for larger datasets
- **DistributedStorage**: CRDT-based distributed storage for robust decentralization
- **FederatedStorage**: Federation support for inter-cooperative collaboration

## 11. Usage Patterns

### Basic Usage

```rust
// Create a storage backend
let mut storage = InMemoryStorage::new();

// Create an auth context with roles
let auth = AuthContext::with_roles("member001", vec!["admin".to_string()]);

// Store a proposal with authorization check
storage.set(Some(&auth), 
    &GovernanceNamespace::proposals("prop-001"), 
    "Proposal content".as_bytes())?;

// Retrieve the proposal
let proposal = storage.get(Some(&auth), 
    &GovernanceNamespace::proposals("prop-001"))?;
```

### Resource Accounting

```rust
// Create a resource account
let mut account = storage.create_resource_account("member001", 1000.0);

// Store data with resource accounting
storage.set_with_resources(Some(&auth), 
    "large_data", 
    &large_value, 
    account)?;
```

### Transactional Operations

```rust
// Begin a transaction
storage.begin_transaction()?;

// Perform multiple operations
storage.set(Some(&auth), "key1", "value1".as_bytes())?;
storage.set(Some(&auth), "key2", "value2".as_bytes())?;

// Commit the transaction
storage.commit_transaction()?;
```

## 12. Integration with ICN-COVM

The storage system integrates with the ICN Cooperative Virtual Machine (COVM) through:

1. **StorageBackend Interface**: The VM accesses storage through the abstract interface
2. **DSL Operations**: `storep` and `loadp` operations in the DSL
3. **Bytecode Operations**: `StoreP` and `LoadP` bytecode operations
4. **Storage Events**: Events emitted during storage operations
5. **CLI Commands**: `storage list-keys` and `storage get-value` commands for inspection

This allows governance scripts (DSL) to interact with the persistent storage layer while maintaining appropriate access controls and audit trails.

## 13. Future Directions

1. **Enhanced Federation**: More sophisticated federation protocols with consensus
2. **Smart Contract Integration**: Storage-aware contracts for automated governance
3. **Encrypted Storage**: End-to-end encryption for sensitive cooperative data
4. **Proof of Cooperation**: Resource contribution tracking across federations
5. **Mobile & Edge Support**: Lightweight storage backends for mobile/edge devices 