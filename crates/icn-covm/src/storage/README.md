# Storage Module

## Overview

The Storage module provides persistent, versioned, identity-aware data storage for the Cooperative Value Network. It enables secure, auditable operations on resources, balances, and arbitrary key-value data with support for multiple backend implementations.

## Core Features

1. **Identity-Aware Access Control**
   - Authentication based on cryptographic identities
   - Fine-grained permission system
   - Role-based access control
   - Audit logging of all operations

2. **Resource Management**
   - Create, mint, transfer, and burn resources
   - Track balances across accounts
   - Transaction history and audit trail
   - Secure validation of operations

3. **Versioned Key-Value Store**
   - Automatic versioning of stored values
   - Version history browsing
   - Difference computation between versions
   - Support for various data types

4. **Multiple Storage Backends**
   - In-memory storage for testing
   - File-based storage for persistence
   - SQL backend for production
   - Custom backend support

## Architecture

The storage module is designed around a set of traits that define the storage interface, with multiple implementations for different backends:

```
┌───────────────────────────────────────────────────────┐
│                 Storage Consumers                     │
│  (VM, Governance, CLI, Federation, Identity, etc.)    │
└─────────────────────────┬─────────────────────────────┘
                          │
                          │ Uses
                          ▼
┌───────────────────────────────────────────────────────┐
│                  Storage Traits                       │
│                                                       │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │  Storage    │  │StorageBackend│  │StorageExtend │  │
│  └─────────────┘  └──────────────┘  └──────────────┘  │
└─────────────────────────┬─────────────────────────────┘
                          │
                          │ Implements
                          ▼
┌───────────────────────────────────────────────────────┐
│               Storage Implementations                 │
│                                                       │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │InMemStorage │  │FileStorage   │  │SQLStorage    │  │
│  └─────────────┘  └──────────────┘  └──────────────┘  │
└───────────────────────────────────────────────────────┘
```

### Key Components

1. **Storage Trait**: Main interface for storage operations
2. **StorageBackend Trait**: Low-level operations for different backends
3. **StorageExtensions Trait**: Additional specialized operations
4. **AuthContext**: Authentication and authorization context
5. **Identity Store**: Management of identities and their permissions

## Core APIs

### Basic Storage Operations

```rust
// Create a storage instance
let storage = FileStorage::new("data_dir")?;

// Store a value
storage.store_string("key", "value", auth_context, "namespace")?;

// Load a value
let value = storage.load_string("key", auth_context, "namespace")?;

// Delete a value
storage.delete("key", auth_context, "namespace")?;

// Check if a key exists
let exists = storage.exists("key", auth_context, "namespace")?;
```

### Resource Management

```rust
// Create a resource
storage.create_resource("community_token", auth_context, "resources")?;

// Mint tokens to an account
storage.mint(
    "community_token", 
    "user123", 
    100.0, 
    "Initial allocation", 
    auth_context, 
    "resources"
)?;

// Transfer tokens between accounts
storage.transfer(
    "community_token",
    "user123",
    "user456",
    25.0,
    "Payment for services",
    auth_context,
    "resources"
)?;

// Check balance
let balance = storage.balance("community_token", "user123", auth_context, "resources")?;

// Burn tokens
storage.burn(
    "community_token",
    "user123",
    10.0,
    "Fee payment",
    auth_context,
    "resources"
)?;
```

### Version Management

```rust
// Get the history of a key
let versions = storage.version_history("key", auth_context, "namespace")?;

// Load a specific version
let old_value = storage.load_version("key", 3, auth_context, "namespace")?;

// Compare versions
let diff = storage.diff_versions("key", 2, 5, auth_context, "namespace")?;
```

### Authentication and Authorization

```rust
// Create an authentication context with an identity
let auth_context = AuthContext::new()
    .with_identity(identity)
    .with_signature(signature);

// Check permissions
if storage.check_permission("user123", "write", "key", "namespace")? {
    // User has permission
}

// Grant permissions
storage.grant_permission("user123", "write", "key", "namespace", admin_auth)?;
```

## Specialized Stores

The module includes specialized stores built on top of the base storage interface:

### Identity Store

```rust
// Create an identity store
let identity_store = IdentityStore::new(storage.clone());

// Store an identity
identity_store.store_identity(&identity, auth_context)?;

// Retrieve an identity
let identity = identity_store.get_identity(identity_id, auth_context)?;

// Check membership
let is_member = identity_store.check_membership(identity_id, "organization", auth_context)?;
```

### Governance Store

```rust
// Create a governance store
let governance_store = GovernanceStore::new(storage.clone());

// Store a proposal
governance_store.store_proposal(&proposal, auth_context)?;

// Record a vote
governance_store.record_vote(proposal_id, voter_id, vote, auth_context)?;

// Get proposal status
let status = governance_store.get_proposal_status(proposal_id, auth_context)?;
```

## Error Handling

The storage module uses a comprehensive error type that covers various failure scenarios:

```rust
match storage.load_string("key", auth_context, "namespace") {
    Ok(value) => {
        // Process value
    },
    Err(StorageError::ResourceNotFound { key, namespace }) => {
        println!("Key '{}' not found in namespace '{}'", key, namespace);
    },
    Err(StorageError::PermissionDenied { user_id, action, key }) => {
        println!("User '{}' does not have permission to {} key '{}'", user_id, action, key);
    },
    Err(err) => {
        eprintln!("Storage error: {}", err);
    }
}
```

## Transaction Support

The storage module provides transaction support for atomic operations:

```rust
// Begin a transaction
storage.begin_transaction()?;

// Perform multiple operations
storage.store_string("key1", "value1", auth_context, "namespace")?;
storage.store_string("key2", "value2", auth_context, "namespace")?;

// Commit or rollback
if all_operations_successful {
    storage.commit_transaction()?;
} else {
    storage.rollback_transaction()?;
}
```

## Configuration Options

Storage backends can be configured with various options:

```rust
let storage_config = FileStorageConfig {
    data_dir: PathBuf::from("data"),
    auto_create_dirs: true,
    sync_writes: true,
    max_versions: 100,
    compression: true,
};

let storage = FileStorage::with_config(storage_config)?;
```

## CLI Integration

The storage module provides CLI commands for management:

```
# List all keys in a namespace
covm storage list --namespace resources

# View a specific key
covm storage get mykey --namespace default

# Store a value
covm storage set mykey myvalue --namespace default

# View version history
covm storage history mykey --namespace default

# Compare versions
covm storage diff mykey 1 2 --namespace default
```

## Example: Resource Ledger

```rust
// Create a storage instance
let storage = FileStorage::new("data_dir")?;

// Create a resource
storage.create_resource("community_token", admin_auth, "resources")?;

// Set up initial allocation
for (member, amount) in initial_allocation {
    storage.mint(
        "community_token",
        member,
        amount,
        "Initial allocation",
        admin_auth,
        "resources"
    )?;
}

// Transfer tokens
storage.transfer(
    "community_token",
    "alice",
    "bob",
    50.0,
    "Payment for services",
    alice_auth,
    "resources"
)?;

// Get current balances
let alice_balance = storage.balance("community_token", "alice", alice_auth, "resources")?;
let bob_balance = storage.balance("community_token", "bob", bob_auth, "resources")?;

println!("Alice: {}, Bob: {}", alice_balance, bob_balance);
``` 