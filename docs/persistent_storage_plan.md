# Persistent Storage Implementation Plan for ICN-COVM

## Overview

This document outlines the implementation plan for adding persistent storage capabilities to the ICN Cooperative Virtual Machine (ICN-COVM). Persistent storage is a foundational feature that will enable governance systems to maintain state across program executions, track historical decisions, and build more complex cooperative applications.

## Design Goals

1. **Consistency**: Storage operations should be atomic and transactional
2. **Namespacing**: Clear separation of concerns through hierarchical namespaces
3. **Simplicity**: Straightforward API that mirrors existing memory operations
4. **Performance**: Efficient read/write operations with reasonable caching
5. **Security**: Access control through identity/permission systems
6. **Extensibility**: Ability to support future storage backends and formats

## Core Components

### 1. Storage Interface

Create a trait that defines the core storage operations:

```rust
pub trait StorageBackend {
    fn get(&self, key: &str) -> Option<f64>;
    fn set(&mut self, key: &str, value: f64) -> Result<(), StorageError>;
    fn delete(&mut self, key: &str) -> Result<(), StorageError>;
    fn contains(&self, key: &str) -> bool;
    fn list_keys(&self, prefix: &str) -> Vec<String>;
    fn commit(&mut self) -> Result<(), StorageError>;
    fn rollback(&mut self);
}
```

### 2. Storage Backends

Implement multiple storage backends:

1. **InMemoryStorage**: For testing and ephemeral use cases
2. **FileStorage**: Simple file-based persistence using JSON or similar format
3. **DatabaseStorage**: (Future) SQL or NoSQL database integration

### 3. VM Integration

Add VM capabilities to interact with persistent storage:

```rust
pub struct VM {
    // Existing fields...
    memory: HashMap<String, f64>,
    
    // New fields
    storage: Box<dyn StorageBackend>,
    transaction_active: bool,
}
```

### 4. New Operations

Add the following operations to the VM:

| Operation | Description |
|-----------|-------------|
| `StoreP(key)` | Store a value in persistent storage |
| `LoadP(key)` | Load a value from persistent storage |
| `DeleteP(key)` | Remove a key from persistent storage |
| `KeyExists(key)` | Check if a key exists in persistent storage |
| `ListKeys(prefix)` | List all keys with a given prefix |
| `BeginTx` | Begin a storage transaction |
| `CommitTx` | Commit a storage transaction |
| `RollbackTx` | Rollback a storage transaction |

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

## Security Considerations

1. **Access Control**: Integration with identity system to restrict storage access
2. **Storage Quotas**: Limit storage usage per namespace/organization
3. **Sanitization**: Validate keys to prevent injection attacks
4. **Auditing**: Log all storage operations for later review

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