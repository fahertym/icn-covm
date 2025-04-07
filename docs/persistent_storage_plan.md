# Persistent Storage Implementation for ICN-COVM

## Overview

This document describes the implementation of persistent storage capabilities in the ICN Cooperative Virtual Machine (ICN-COVM). Persistent storage is a foundational feature that enables governance systems to maintain state across program executions, track historical decisions, and build more complex cooperative applications.

## Design Goals

1. **Consistency**: Storage operations are atomic and transactional
2. **Namespacing**: Clear separation of concerns through hierarchical namespaces
3. **Simplicity**: Straightforward API that mirrors existing memory operations
4. **Performance**: Efficient read/write operations with reasonable caching
5. **Security**: Access control through identity/permission systems
6. **Extensibility**: Ability to support future storage backends and formats

## Core Components

### 1. Storage Interface

The core storage operations are defined in the `StorageBackend` trait:

```rust
pub trait StorageBackend {
    fn get(&self, auth: &AuthContext, namespace: &str, key: &str) -> StorageResult<Vec<u8>>;
    fn set(&mut self, auth: &AuthContext, namespace: &str, key: &str, value: Vec<u8>) -> StorageResult<()>;
    fn delete(&mut self, auth: &AuthContext, namespace: &str, key: &str) -> StorageResult<()>;
    fn key_exists(&self, auth: &AuthContext, namespace: &str, key: &str) -> StorageResult<bool>;
    fn list_keys(&self, auth: &AuthContext, namespace: &str, prefix: Option<&str>) -> StorageResult<Vec<String>>;
    fn begin_transaction(&mut self) -> StorageResult<()>;
    fn commit_transaction(&mut self) -> StorageResult<()>;
    fn rollback_transaction(&mut self) -> StorageResult<()>;
    fn create_account(&mut self, auth: &AuthContext, user_id: &str, quota_bytes: u64) -> StorageResult<()>;
}
```

### 2. Storage Backends

Multiple storage backends are implemented:

1. **InMemoryStorage**: For testing and ephemeral use cases
2. **FileStorage**: Simple file-based persistence 

### 3. VM Integration

The VM includes capabilities to interact with persistent storage:

```rust
pub struct VM {
    // Existing fields...
    memory: HashMap<String, f64>,
    
    // Storage fields
    storage_backend: Option<Box<dyn StorageBackend>>,
    auth_context: AuthContext,
    namespace: String,
}
```

### 4. Storage Operations

The VM supports the following storage operations:

| Operation | Description |
|-----------|-------------|
| `StoreP(key)` | Store a value in persistent storage |
| `LoadP(key)` | Load a value from persistent storage |
| `DeleteP(key)` | Remove a key from persistent storage |
| `KeyExistsP(key)` | Check if a key exists in persistent storage |
| `ListKeysP(prefix)` | List all keys with a given prefix |
| `StorePTyped(key, type)` | Store a value with type validation |
| `LoadPTyped(key, type)` | Load a value with type validation |
| `BeginTx` | Begin a storage transaction |
| `CommitTx` | Commit a storage transaction |
| `RollbackTx` | Rollback a storage transaction |

## Implementation Details

### Authentication and Authorization

All storage operations require an `AuthContext` that provides:

```rust
pub struct AuthContext {
    pub user_id: String,
    pub roles: HashMap<String, HashSet<String>>,
}
```

This enables:
- Role-based access control for storage operations
- Audit trails with attribution
- Resource accounting for storage operations

### Transactions

The storage system supports atomic transactions with:

```
BeginTx    # Begin a transaction
CommitTx   # Commit the current transaction
RollbackTx # Rollback the current transaction
```

This ensures consistency for multi-step operations like voting or configuration changes.

### Namespaces

Storage uses a hierarchical namespace structure for organization and access control:

- `governance/{org_id}/...` - Organization-specific governance data
- `member/{member_id}/...` - Member-specific data
- `vote/{vote_id}/...` - Vote-related data
- `system/...` - System configuration and metadata

### Typed Storage

The storage system supports typed values through JSON serialization:

- **number**: Floating-point values
- **integer**: Integer values
- **boolean**: True/false values
- **string**: Text values
- **null**: Empty values

Operations:
```
StorePTyped(key, type)  # Store with type validation
LoadPTyped(key, type)   # Load with type validation
```

### Resource Accounting

Storage operations track resource usage through resource accounts:

```rust
pub struct ResourceAccount {
    pub user_id: String,
    pub quota_bytes: u64,
    pub used_bytes: u64,
}
```

## Usage Examples

### Basic Storage Operations

```
# Store a value in persistent storage
push 100.0
storep "org/treasury/balance"

# Load a value from storage
loadp "org/treasury/balance"

# Check if a key exists
keyexistsp "org/treasury/balance"

# Delete a key
deletep "org/treasury/balance"
```

### Typed Storage Operations

```
# Store an integer
push 42.0
storepTyped "config/max_votes" "integer"

# Store a boolean
push 1.0  # true
storepTyped "config/voting_enabled" "boolean"

# Load a string
loadpTyped "member/alice/name" "string"
```

### Transaction Example

```
# Begin a transaction for atomic operations
begintx

# Update multiple values atomically
push 90.0
storep "org/treasury/balance"
push 10.0
storep "org/projects/funding"

# Commit the transaction
committx
```

### Integration with Identity

```
# Get the current caller's ID
getcaller
store "current_user"

# Check if the caller has the treasurer role
hasrole "treasurer"
if:
    # Perform treasury operations
    push 100.0
    storep "treasury/balance"
else:
    emit "Access denied"
```

## Security Considerations

The storage system includes these security features:

1. **Access Control**: Integration with identity system restricts storage access
2. **Storage Quotas**: Limit storage usage per user through resource accounts
3. **Namespace Validation**: Prevents invalid namespace/key access
4. **Audit Logging**: Tracks all storage operations with user attribution

## Implementation Phases

### Phase 1: Core Storage Interface

1. Define the `StorageBackend` trait
2. Implement `InMemoryStorage` for testing
3. Add persistent storage field to VM struct
4. Add basic error handling for storage operations

### Phase 2: Basic Operations

1. Implement `StoreP` and `LoadP` operations
2. Update bytecode compiler and parser to handle new operations
3. Add tests for basic storage operations
4. Create file-based storage backend

### Phase 3: Transactions and Namespaces

1. Implement transaction support (`BeginTx`, `CommitTx`, `RollbackTx`)
2. Add namespace validation and hierarchical key structure
3. Implement `ListKeys` and `KeyExists` operations
4. Add namespace-based access control hooks for future integration with identity system

### Phase 4: Integration and Tooling

1. Create storage inspection tools for debugging
2. Implement storage migration capabilities
3. Add CLI commands for storage management
4. Create storage benchmark and performance tests

## Persistent Storage DSL Example

```
# Store a value in persistent storage
push 100.0
storep "org/treasury/balance"

# Begin a transaction for atomic operations
begintx

# Update multiple values atomically
push 90.0
storep "org/treasury/balance"
push 10.0
storep "org/projects/funding"

# Commit the transaction
committx

# List all keys in the treasury namespace
push "org/treasury/"
listkeys
```

## Namespace Structure

We'll adopt a hierarchical namespace structure with segments separated by forward slashes:

- `org/{org_id}/...` - Organization-specific data
- `member/{member_id}/...` - Member-specific data
- `vote/{vote_id}/...` - Vote-related data
- `system/...` - System configuration and metadata

For example:
- `org/acme/treasury/balance`
- `member/alice/voting_power`
- `vote/proposal_42/tallies/option_1`
- `system/version`

## Future Extensions

1. **TypedStorage**: Support for storing different data types beyond f64
2. **Object Storage**: Key-value storage for more complex data structures
3. **Versioned Storage**: Track historical values for keys
4. **Encrypted Storage**: Add encryption for sensitive data
5. **Remote Storage**: Distributed storage across federated VMs

## Technical Challenges

1. **Consistency**: Ensuring atomic operations across multiple storage actions
2. **Performance**: Balancing cache usage and persistence guarantees
3. **Migration**: Supporting schema evolution over time
4. **Concurrency**: Handling multiple VMs accessing the same storage
5. **Error Handling**: Graceful recovery from storage failures

## Next Steps

1. Create the `StorageBackend` trait and `InMemoryStorage` implementation
2. Add the `StoreP` and `LoadP` operations to the VM
3. Update the DSL parser to support the new operations
4. Develop comprehensive tests for the storage subsystem
5. Implement a file-based storage backend for basic persistence 