# Storage System Documentation

## Overview

The ICN-COVM Storage System provides persistent state management across VM executions. It enables cooperative applications to maintain state, implement transactions, and manage namespaced data with proper access controls.

This document describes the storage system architecture, interfaces, implementations, and integration with the identity system.

## Core Components

### StorageBackend Trait

The foundation of the storage system is the `StorageBackend` trait:

```rust
pub trait StorageBackend {
    // Basic operations
    fn get(&self, key: &str, auth: Option<&AuthContext>) -> Result<Option<String>, StorageError>;
    fn set(&mut self, key: &str, value: &str, auth: Option<&AuthContext>) -> Result<(), StorageError>;
    fn delete(&mut self, key: &str, auth: Option<&AuthContext>) -> Result<(), StorageError>;
    fn exists(&self, key: &str, auth: Option<&AuthContext>) -> Result<bool, StorageError>;
    
    // JSON operations
    fn get_json<T: DeserializeOwned>(&self, key: &str, auth: Option<&AuthContext>) -> Result<Option<T>, StorageError>;
    fn set_json<T: Serialize>(&mut self, key: &str, value: &T, auth: Option<&AuthContext>) -> Result<(), StorageError>;
    
    // Key listing
    fn list_keys(&self, prefix: &str, auth: Option<&AuthContext>) -> Result<Vec<String>, StorageError>;
    
    // Transaction support
    fn begin_transaction(&mut self) -> Result<(), StorageError>;
    fn commit_transaction(&mut self) -> Result<(), StorageError>;
    fn rollback_transaction(&mut self) -> Result<(), StorageError>;
    fn in_transaction(&self) -> bool;
    
    // Version information
    fn get_version(&self, key: &str, auth: Option<&AuthContext>) -> Result<Option<u64>, StorageError>;
    fn get_latest_version(&self) -> Result<u64, StorageError>;
}
```

### StorageError

Error types for storage operations:

```rust
pub enum StorageError {
    // Basic errors
    KeyNotFound(String),
    InvalidKey(String),
    SerializationError(String),
    DeserializationError(String),
    
    // Transaction errors
    TransactionError(String),
    NoActiveTransaction,
    NestedTransactionNotSupported,
    
    // Permission errors
    PermissionDenied(String),
    AuthenticationRequired(String),
    
    // Backend-specific errors
    BackendError(String),
    IOError(String),
}
```

### Storage Implementations

The system provides multiple storage backend implementations:

1. **InMemoryStorage**: Volatile storage for testing and development
2. **FileStorage**: File-based persistent storage using JSON serialization
3. **Custom backends**: The trait can be implemented for other storage systems

## VM Operations

The storage system adds several operations to the VM:

### Basic Storage Operations

```
# Store a value
push 100.0
storep "org/treasury/balance"

# Load a value
loadp "org/treasury/balance"

# Check if a key exists
keyexists "org/treasury/balance"

# Delete a key
deletep "org/treasury/balance"

# List keys with a prefix
push "org/treasury/"
listkeys
```

### Transaction Operations

```
# Begin a transaction
begintx

# Commit a transaction
committx

# Rollback a transaction
rollbacktx
```

## DSL Examples

### Basic Key-Value Storage

```
# Store and retrieve a value
push 42.0
storep "app/counter"

loadp "app/counter"
emit

# Store a calculated value
push 10.0
push 5.0
add
storep "app/sum"
```

### Transaction Example

```
# Atomic transfer between accounts
begintx
    # Load first account
    loadp "accounts/alice"
    push 10.0
    sub
    storep "accounts/alice"
    
    # Load second account
    loadp "accounts/bob"
    push 10.0
    add
    storep "accounts/bob"
committx

# Handle errors gracefully
onerror:
    rollbacktx
    emit "Transfer failed"
enderr
```

### Namespace Management

```
# List all accounts
push "accounts/"
listkeys

# Create a counter for each account
begintx
    push "accounts/"
    listkeys
    store "accounts"
    
    push 0
    store "i"
    push 0
    store "count"
    
    load "accounts"
    length
    store "num_accounts"
    
    while:
        load "i"
        load "num_accounts"
        lt
    do:
        load "accounts"
        load "i"
        array_get
        
        push "/counter"
        concat
        
        push 0
        storep
        
        load "i"
        push 1
        add
        store "i"
    endwhile
committx
```

## Integration with Identity System

The storage system integrates with the identity system to enable:

1. **Permission Checking**: StorageBackend operations validate permissions via the AuthContext
2. **Namespace Permissions**: Different roles can have access to different namespaces
3. **Audit Trails**: Storage operations can be logged with identity information

Example with permissions:

```
# Only admins can access this namespace
begintx
    requirerole "admin"
    push 100.0
    storep "admin/config/max_users"
committx

# Regular members can only access their own namespace
begintx
    requirerole "member"
    getcaller
    store "user_id"
    
    push 42.0
    push "users/"
    load "user_id"
    concat
    push "/profile/score"
    concat
    storep
committx
```

## Namespacing Conventions

The storage system uses hierarchical namespaces:

1. **Organization-level**: `org/*` or `coop/*`
2. **Module-level**: `org/module/*`
3. **User-level**: `users/user_id/*`
4. **Governance-level**: `governance/*`

These conventions help organize data and implement proper access controls.

## File-based Storage Implementation

The `FileStorage` backend stores data in a structured file hierarchy:

```
storage/
├── v1/
│   ├── namespace1/
│   │   ├── key1.json
│   │   └── key2.json
│   └── namespace2/
│       └── key3.json
└── latest_version.json
```

This allows:
- Easy backup and inspection
- Version history
- Clear namespace separation

## Performance Considerations

1. **Caching**: Both implementations include caching for frequently accessed keys
2. **Transaction Performance**: Transactions are optimized to minimize disk I/O
3. **Batch Operations**: Consider using transactions for batch operations

## Security Considerations

1. **Key Validation**: All keys are validated for security
2. **Access Control**: Namespaces should enforce proper access control
3. **Transactions**: Use transactions for consistent state updates
4. **Error Handling**: Always handle storage errors gracefully

## Best Practices

1. **Namespace Design**: Create a clear namespace hierarchy
2. **Versioning**: Use version information when needed
3. **Transactions**: Use transactions for related operations
4. **Key Prefixes**: Use consistent key prefixes for organization
5. **Error Handling**: Implement proper error recovery

## Future Extensions

1. **Query Support**: Advanced querying beyond simple key lookups
2. **Compression**: Data compression for efficient storage
3. **Encryption**: Encrypt sensitive data at rest
4. **Remote Storage**: Network-based storage backends
5. **Schema Validation**: Validate data against schemas
