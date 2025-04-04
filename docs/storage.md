# Storage System Documentation

## Overview

The ICN-COVM Storage System provides persistent state management across VM executions. It enables cooperative applications to maintain state, implement transactions, and manage namespaced data with proper access controls.

This document describes the storage system architecture, interfaces, implementations, and integration with the identity system.

## Core Components

### StorageBackend Trait

The foundation of the storage system is the `StorageBackend` trait:

```rust
pub trait StorageBackend {
    /// Retrieves raw byte data associated with a key within a namespace.
    /// Performs permission checks based on the provided `AuthContext`.
    fn get(&self, auth: Option<&AuthContext>, namespace: &str, key: &str) -> StorageResult<Vec<u8>>;

    /// Retrieves data along with its versioning information.
    fn get_versioned(&self, auth: Option<&AuthContext>, namespace: &str, key: &str) -> StorageResult<(Vec<u8>, VersionInfo)>;
    
    /// Retrieves a specific version of data
    fn get_version(&self, auth: Option<&AuthContext>, namespace: &str, key: &str, version: u64) -> StorageResult<(Vec<u8>, VersionInfo)>;
    
    /// Lists all available versions for a key
    fn list_versions(&self, auth: Option<&AuthContext>, namespace: &str, key: &str) -> StorageResult<Vec<VersionInfo>>;
    
    /// Compare two versions and return differences
    fn diff_versions(&self, auth: Option<&AuthContext>, namespace: &str, key: &str, v1: u64, v2: u64) -> StorageResult<VersionDiff<Vec<u8>>>;

    /// Sets raw byte data for a key within a namespace.
    /// Performs permission checks and resource accounting.
    /// Updates version information.
    fn set(&mut self, auth: Option<&AuthContext>, namespace: &str, key: &str, value: Vec<u8>) -> StorageResult<()>;
    
    /// List keys in a namespace
    fn list_keys(&self, auth: Option<&AuthContext>, namespace: &str, prefix: Option<&str>) -> StorageResult<Vec<String>>;
    
    /// List sub-namespaces
    fn list_namespaces(&self, auth: Option<&AuthContext>, parent_namespace: &str) -> StorageResult<Vec<NamespaceMetadata>>;

    /// Creates a resource account for a user.
    /// Typically requires administrative privileges.
    fn create_account(&mut self, auth: Option<&AuthContext>, user_id: &str, quota_bytes: u64) -> StorageResult<()>;
    
    /// Creates a new namespace
    fn create_namespace(&mut self, auth: Option<&AuthContext>, namespace: &str, quota_bytes: u64, parent: Option<&str>) -> StorageResult<()>;

    /// Checks if the user has the required permission for an action in a namespace.
    /// This might be used internally by other methods or exposed for direct checks.
    fn check_permission(&self, auth: Option<&AuthContext>, action: &str, namespace: &str) -> StorageResult<()>;

    /// Begins a transaction.
    /// Subsequent `set` operations should be part of this transaction until commit/rollback.
    fn begin_transaction(&mut self) -> StorageResult<()>;

    /// Commits the current transaction, making changes permanent.
    fn commit_transaction(&mut self) -> StorageResult<()>;

    /// Rolls back the current transaction, discarding changes.
    fn rollback_transaction(&mut self) -> StorageResult<()>;

    /// Retrieves audit log entries, potentially filtered.
    /// Requires appropriate permissions.
    fn get_audit_log(&self, auth: Option<&AuthContext>, namespace: Option<&str>, event_type: Option<&str>, limit: usize) -> StorageResult<Vec<StorageEvent>>;

    /// Delete a key and its versions
    fn delete(&mut self, auth: Option<&AuthContext>, namespace: &str, key: &str) -> StorageResult<()>;
    
    /// Get storage usage for a namespace
    fn get_usage(&self, auth: Option<&AuthContext>, namespace: &str) -> StorageResult<u64>;
}
```

### StorageError

Error types for storage operations:

```rust
pub enum StorageError {
    NotFound { key: String },
    PermissionDenied { user_id: String, action: String, key: String },
    QuotaExceeded { account_id: String, requested: u64, available: u64 },
    VersionConflict { key: String, expected: u64, actual: u64 },
    SerializationError { details: String },
    TransactionError { details: String },
    IoError { details: String },
    // Add other specific errors as needed
}
```

### Storage Backend Implementations

The system provides multiple `StorageBackend` implementations:

1. **InMemoryStorage**: Volatile storage for testing and development
2. **FileStorage**: File-based persistent storage with versioning

### StorageExtensions Trait

The system also provides a `StorageExtensions` trait that adds convenient JSON serialization/deserialization methods:

```rust
trait StorageExtensions: StorageBackend {
    fn get_json<T: DeserializeOwned>(&self, auth: Option<&AuthContext>, namespace: &str, key: &str) -> StorageResult<T>;
    fn set_json<T: Serialize>(&mut self, auth: Option<&AuthContext>, namespace: &str, key: &str, value: &T) -> StorageResult<()>;
}
```

This trait is automatically implemented for any type that implements `StorageBackend`, allowing you to easily serialize and deserialize structured data:

## VM Operations

The storage system adds several operations to the VM:

### Basic Storage Operations

```
# Store a value in persistent storage
push 100.0
storep "org/treasury/balance"

# Load a value from persistent storage
loadp "org/treasury/balance"
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

### Namespace Usage Example

```
# Store values in different namespaces
push 100.0
storep "accounts/alice/balance"

push 200.0
storep "accounts/bob/balance"

# Transaction example with namespaces
begintx
    # Load and update Alice's balance
    loadp "accounts/alice/balance"
    push 50.0
    add
    storep "accounts/alice/balance"
    
    # Load and update Bob's balance
    loadp "accounts/bob/balance"
    push 50.0
    sub
    storep "accounts/bob/balance"
committx

# Handle errors with transactions
onerror:
    rollbacktx
    emit "Transfer failed"
enderr
```

## Integration with Identity System

The storage system integrates with the identity system via the `AuthContext` object, which:

1. **Provides Identity Information**: User ID and roles for permission checking
2. **Enables Access Control**: Different roles can access different namespaces
3. **Supports Auditing**: All operations are logged with identity information

The `StorageBackend` methods accept an optional `auth: Option<&AuthContext>` parameter to implement:

1. **Role-Based Access Control**: Different roles (admin, writer, reader) have different permissions
2. **Namespace Restrictions**: Users can only access namespaces they have permissions for
3. **Resource Quotas**: Track and limit storage usage per user or namespace

Example storage operations with identity integration:

```rust
// Create an auth context for a user with specific roles
let mut auth = AuthContext::new("user_123");
auth.add_role("default", "reader");
auth.add_role("default", "writer");

// Storage operations use the auth context for permission checking
storage.set(&auth, "default", "key1", value)?;
storage.get(&auth, "default", "key1")?;
```

In DSL, the identity system affects storage operations implicitly through the VM's current auth context:

```
# The VM has an auth context set up with appropriate roles
push 100.0
storep "org/treasury/balance"  # This will check permissions using current auth context
```

## Namespacing Conventions

The storage system uses hierarchical namespaces to organize data and implement proper access controls:

1. **Default namespace**: `default` - For general storage
2. **User namespaces**: Named after a user ID - For user-specific data
3. **Organization namespaces**: `org`, `coop`, etc. - For organization-wide data
4. **Module namespaces**: `governance`, `voting`, etc. - For module-specific data
5. **Nested namespaces**: Can be created with parent/child relationships

## File-based Storage Implementation

The `FileStorage` backend organizes data in a structured directory hierarchy:

```
storage_root/
├── namespaces/
│   ├── default/
│   │   ├── keys/
│   │   │   ├── key1/
│   │   │   │   ├── v1.data
│   │   │   │   ├── v2.data
│   │   │   │   └── metadata.json
│   │   │   └── key2/
│   │   │       ├── v1.data
│   │   │       └── metadata.json
│   │   └── namespace_metadata.json
│   └── governance/
│       ├── keys/
│       │   ├── proposal_123/
│       │   │   ├── v1.data
│       │   │   └── metadata.json
│       ├── namespace_metadata.json
├── accounts/
│   ├── user1.json
│   └── user2.json
└── audit_logs/
    └── 2023-12-01.log
```

This structure provides:
- Namespace isolation for security
- Efficient key-based lookups
- Version tracking for each key
- Resource usage accounting