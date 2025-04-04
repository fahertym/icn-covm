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

## Selecting a Storage Backend

The COVM CLI supports selecting which storage backend to use through command-line options:

```bash
# Use in-memory storage (default)
cargo run -- run --program your_program.dsl --storage-backend memory

# Use file-based storage with a specified directory
cargo run -- run --program your_program.dsl --storage-backend file --storage-path ./storage_dir
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