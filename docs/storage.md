# ICN-COVM Storage System

The ICN Cooperative Virtual Machine (COVM) includes a powerful storage system that enables programs to store and retrieve data persistently. This document describes the storage system architecture, available backends, and how to use storage in DSL programs.

## Storage Backends

The COVM supports multiple storage backends through a common interface defined by the `StorageBackend` trait. Currently, two implementations are available:

### InMemoryStorage

- Stores data in memory during program execution
- Data is lost when the program terminates
- Useful for testing, development, and ephemeral data

### FileStorage

- Stores data on disk in a structured directory hierarchy
- Data persists between program runs
- Supports versioning, audit logs, and transaction rollback
- Implements resource usage tracking and quotas
- Provides robust permission checking
- Includes file locking for concurrent access safety
- Features comprehensive error handling with context

## Selecting a Storage Backend

The COVM CLI supports selecting which storage backend to use through command-line options:

```bash
# Use in-memory storage (default)
cargo run -- run --program your_program.dsl --storage-backend memory

# Use file-based storage with a specified directory
cargo run -- run --program your_program.dsl --storage-backend file --storage-path ./storage_dir
```

## Storage Inspection

The COVM CLI provides commands for inspecting storage:

```bash
# List keys in a namespace
cargo run -- storage list-keys demo --storage-backend file --storage-path ./storage

# Optionally filter by prefix
cargo run -- storage list-keys demo --prefix user_ --storage-backend file --storage-path ./storage

# Get a value from storage
cargo run -- storage get-value demo counter --storage-backend file --storage-path ./storage
```

## Authorization Model

The storage system implements an identity-aware authorization model with the following components:

1. **Users**: Identified by a user ID string
2. **Namespaces**: Logical containers for data, similar to directories
3. **Roles**: Assigned to users for specific namespaces:
   - `reader`: Permission to read data
   - `writer`: Permission to write data
   - `admin`: Permission to manage namespace configuration
   - `global admin`: Permission to manage accounts and create namespaces

The storage API uses the `Option<&AuthContext>` pattern for flexible authentication handling:

```rust
fn get(&self, auth: Option<&AuthContext>, namespace: &str, key: &str) -> StorageResult<Vec<u8>>;
fn set(&mut self, auth: Option<&AuthContext>, namespace: &str, key: &str, value: &[u8]) -> StorageResult<()>;
fn delete(&mut self, auth: Option<&AuthContext>, namespace: &str, key: &str) -> StorageResult<()>;
```

This allows operations to be performed with or without authentication context, while ensuring proper permission checks when auth is provided.

## Usage in DSL Programs

### Storing Data

To store a value in persistent storage:

```
# First push the value on the stack
push 42.0
# Then store it with a key
storep my_counter
```

### Loading Data

To load a value from persistent storage:

```
# Load the value onto the stack
loadp my_counter
```

### Conditional Storage Patterns

A common pattern for initializing a counter:

```
# Try to load the counter
loadp counter
# Check if it exists (0.0 = falsy)
push 0.0
eq
if:
  # First run: initialize counter to 0
  push 0.0
  storep counter
  # Load the counter for following operations
  loadp counter
else:
  # Counter already exists, proceed with existing value
  emit "Counter exists with value:"
  dumpstack
```

## Storage Backend Implementation Details

### InMemoryStorage

The `InMemoryStorage` backend provides a simple in-memory implementation that is useful for:
- Development and testing
- Programs that don't need persistence
- Benchmarking and performance testing

Data is stored in memory and is lost when the program terminates.

### FileStorage

The `FileStorage` backend implements a persistent storage solution with the following features:

- **File Organization**:
  - Root directory contains `namespaces/`, `accounts/`, `audit_logs/`, and `transactions/` directories
  - Each namespace has its own directory structure
  - Keys are stored in versioned files with metadata

- **Versioning**:
  - Every update to a key creates a new numbered version
  - Previous versions are retained and can be accessed
  - Metadata is stored alongside data, including creation timestamps and authors

- **Transactions**:
  - Supports beginning, committing, and rolling back transactions
  - Transaction logs track all operations for potential rollback
  - Ensures data consistency across related operations

- **Resource Management**:
  - User accounts have configurable storage quotas
  - Storage usage is tracked for each user
  - Prevents users from exceeding their allocated quota

- **Audit Logs**:
  - All operations are logged with timestamps, user IDs, and operation details
  - Logs can be queried for audit and debugging purposes

- **Concurrency Safety**:
  - Implements file locking using the `fs2` crate
  - Prevents concurrent modification of the same data
  - Handles lock timeout and recovery gracefully

- **Error Handling**:
  - Comprehensive error types with detailed context
  - Helpful error messages for debugging
  - Clear distinction between permission errors, I/O errors, and logical errors

## Example Programs

The COVM includes several example programs that demonstrate the storage system:

### Persistent Counter

The `demo/storage/persistent_counter.dsl` program demonstrates a simple counter that increments each time it's run. Run it with:

```bash
# With in-memory storage (resets each run)
cargo run -- run --program demo/storage/persistent_counter.dsl --storage-backend memory

# With file storage (persists between runs)
cargo run -- run --program demo/storage/persistent_counter.dsl --storage-backend file --storage-path ./filestorage
```

### Shopping Cart

The `demo/storage/cart.dsl` program demonstrates a simple shopping cart that adds items with each run. Run it with:

```bash
# First run - creates an empty cart and adds first item
cargo run -- run --program demo/storage/cart.dsl --storage-backend file --storage-path ./filestorage

# Subsequent runs - adds more items to existing cart
cargo run -- run --program demo/storage/cart.dsl --storage-backend file --storage-path ./filestorage
```

### Economic Operations

The economic operations in COVM (`CreateResource`, `Mint`, `Transfer`, `Burn`, `Balance`) rely heavily on the storage system to maintain resource data and account balances between executions.

Key storage patterns for economic operations:

- Resource metadata is stored at `resources/{resource_id}`
- Account balances are stored at `resources/{resource_id}/balances` as a JSON object
- All operations generate audit events in the "economic" category
- Atomic transactions ensure data consistency during operations like transfers

Example programs in `demo/economic/` demonstrate these storage patterns:

```bash
# Create resources and perform economic operations
cargo run -- run --program demo/economic/create_resource.dsl --storage-backend file --storage-path ./filestorage

# Basic transfer example
cargo run -- run --program demo/economic/basic_transfer.dsl --storage-backend file --storage-path ./filestorage
```

See `docs/economic_operations.md` for comprehensive documentation on these operations.

### Basic Storage Test

The `demo/storage/basic_storage_test.dsl` program tests basic storage functionality:

```bash
# Test with file storage
cargo run -- run --program demo/storage/basic_storage_test.dsl --storage-backend file --storage-path ./filestorage
```

## Limitations

1. The `InMemoryStorage` backend does not persist data across program runs.
2. The `FileStorage` backend is currently not optimized for very large datasets or high concurrency.
3. Complex queries and indexing are not supported directly by the storage backends.

## Performance Considerations

- For small programs with minimal storage needs, both backends offer good performance.
- For programs with large data requirements, the `FileStorage` backend may have increased latency compared to `InMemoryStorage`.
- The `FileStorage` backend uses caching for namespace and account metadata to improve performance.

## Future Enhancements

Planned improvements to the storage system include:
- Database-backed storage options (SQL, NoSQL)
- Enhanced query capabilities beyond simple key-value lookups
- Additional optimization for the `FileStorage` backend
- Network-distributed storage options