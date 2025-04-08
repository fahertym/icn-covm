# Storage Backend Integration Guide

This guide explains how to integrate the FileStorage backend into your ICN-COVM applications for persistent storage.

## Overview

The ICN-COVM includes two primary storage backend implementations:

1. **InMemoryStorage**: Used for testing and demos, data is lost when the process ends
2. **FileStorage**: Provides persistent storage to disk, suitable for production use

Currently, the CLI defaults to using InMemoryStorage, which is why data doesn't persist between runs. This guide shows how to properly integrate FileStorage.

## Prerequisites

The FileStorage implementation is already complete, including:

- Full implementation of the StorageBackend trait
- Support for namespaces, keys, and versioning
- Transaction support with commit/rollback
- Identity-aware authorization
- Resource accounting
- File locking for concurrent access
- Comprehensive error handling
- Thorough test coverage

## Integration Steps

### 1. Add Command Line Options

First, add command-line flags to specify the storage backend and location:

```rust
// In src/main.rs
.arg(
    Arg::new("storage-backend")
        .long("storage-backend")
        .value_name("TYPE")
        .help("Storage backend type (memory or file)")
        .default_value("memory"),
)
.arg(
    Arg::new("storage-path")
        .long("storage-path")
        .value_name("PATH")
        .help("Path for file storage backend")
        .default_value("./storage"),
)
```

### 2. Create the Selected Storage Backend

Update your `run_program` function to use the specified backend:

```rust
// In src/main.rs
fn run_program(
    program_path: &str,
    verbose: bool,
    use_stdlib: bool,
    parameters: HashMap<String, String>,
    use_bytecode: bool,
    storage_backend: &str,
    storage_path: &str,
) -> Result<(), AppError> {
    // ... existing code ...

    // Set up the appropriate storage backend
    let auth_context = create_demo_auth_context();
    
    // Select the appropriate storage backend
    if storage_backend == "file" {
        if verbose {
            println!("Using FileStorage backend at {}", storage_path);
        }
        
        // Create the storage directory if it doesn't exist
        let storage_dir = Path::new(storage_path);
        if !storage_dir.exists() {
            if verbose {
                println!("Creating storage directory: {}", storage_path);
            }
            fs::create_dir_all(storage_dir)
                .map_err(|e| AppError::Other(format!("Failed to create storage directory: {}", e)))?;
        }
        
        // Initialize the FileStorage backend
        match FileStorage::new(storage_path) {
            Ok(mut storage) => {
                initialize_storage(&auth_context, &mut storage, verbose)?;
                
                // ... rest of function with storage ...
            },
            Err(e) => {
                return Err(AppError::Other(format!("Failed to initialize file storage: {}", e)));
            }
        }
    } else {
        // Use InMemoryStorage (default)
        if verbose {
            println!("Using InMemoryStorage backend");
        }
        
        // Initialize InMemoryStorage
        let mut storage = InMemoryStorage::new();
        initialize_storage(&auth_context, &mut storage, verbose)?;
        
        // ... rest of function with storage ...
    }
    
    // ... rest of function ...
}

// Helper function to initialize any storage backend
fn initialize_storage<T: StorageBackend>(
    auth_context: &AuthContext,
    storage: &mut T,
    verbose: bool,
) -> Result<(), AppError> {
    // Create user account
    if let Err(e) = storage.create_account(Some(auth_context), &auth_context.user_id, 1024 * 1024) {
        if verbose {
            println!("Warning: Failed to create account: {:?}", e);
        }
    }
    
    // Create namespace
    if let Err(e) = storage.create_namespace(Some(auth_context), "demo", 1024 * 1024, None) {
        if verbose {
            println!("Warning: Failed to create namespace: {:?}", e);
        }
    }
    
    Ok(())
}
```

### 3. Update Command Handling

Update the command handling code to pass the storage backend options:

```rust
// In main.rs main() function
match matches.subcommand() {
    Some(("run", run_matches)) => {
        // ... existing code ...
        
        let storage_backend = run_matches.get_one::<String>("storage-backend").unwrap();
        let storage_path = run_matches.get_one::<String>("storage-path").unwrap();
        
        // ... existing code ...
        
        if let Err(err) = run_program(
            program_path, 
            verbose, 
            use_stdlib, 
            parameters, 
            use_bytecode,
            storage_backend,
            storage_path
        ) {
            // ... error handling ...
        }
    }
    // ... rest of function ...
}
```

### 4. Update Storage API Usage

When using the storage backend directly, ensure you're using the `Option<&AuthContext>` pattern:

```rust
// Example storage operations with the new API pattern
let auth_context = create_demo_auth_context();

// Create operations with auth context
storage.set(Some(&auth_context), "demo", "key1", "value1".as_bytes())?;

// Read operations can specify auth context for permission checks
let data = storage.get(Some(&auth_context), "demo", "key1")?;

// Or omit auth context when appropriate (permission checks will not be performed)
// Note: This is generally only appropriate for system-level operations or testing
storage.get(None, "demo", "key2")?;
```

### 5. Implement Storage Inspection Commands

Add CLI commands for inspecting storage contents:

```rust
.subcommand(
    Command::new("storage")
        .about("Storage inspection commands")
        .arg(
            Arg::new("storage-backend")
                .long("storage-backend")
                .value_name("TYPE")
                .help("Storage backend type (memory or file)")
                .default_value("file"),
        )
        .arg(
            Arg::new("storage-path")
                .long("storage-path")
                .value_name("PATH")
                .help("Path for file storage backend")
                .default_value("./storage"),
        )
        .subcommand(
            Command::new("list-keys")
                .about("List all keys in a namespace")
                .arg(
                    Arg::new("namespace")
                        .help("Namespace to list keys from")
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::new("prefix")
                        .short('p')
                        .long("prefix")
                        .help("Only list keys with this prefix")
                        .value_name("PREFIX"),
                )
        )
        .subcommand(
            Command::new("get-value")
                .about("Get a value from storage")
                .arg(
                    Arg::new("namespace")
                        .help("Namespace to get value from")
                        .required(true)
                        .index(1),
                )
                .arg(
                    Arg::new("key")
                        .help("Key to get value for")
                        .required(true)
                        .index(2),
                )
        )
)
```

## Example Usage

Once implemented, users can run programs with persistent storage:

```bash
# Run with file storage
cargo run -- run --program demo/storage/persistent_counter.dsl --verbose --storage-backend file --storage-path ./data/storage

# Run again - counter value will persist!
cargo run -- run --program demo/storage/persistent_counter.dsl --verbose --storage-backend file --storage-path ./data/storage

# List keys in a namespace
cargo run -- storage list-keys demo --storage-backend file --storage-path ./data/storage

# Get a value from storage
cargo run -- storage get-value demo counter --storage-backend file --storage-path ./data/storage
```

## Testing

To test the integration:

1. Run the persistent counter example with file storage
2. Verify that the counter value increments across runs
3. Examine the storage directory to see the stored data files
4. Use the storage inspection commands to view the stored data

## Troubleshooting

Common issues:

- **Permission errors**: Ensure the storage directory is writable
- **Missing directories**: The FileStorage implementation should create required directories
- **Transaction failures**: Check for disk space issues if transaction operations fail
- **Locking errors**: If you see errors about failing to acquire lock, check for stale locks

## Implementation Details

The FileStorage backend organizes data as follows:

```
storage_path/
├── namespaces/
│   ├── demo/
│   │   ├── keys/
│   │   │   ├── counter/
│   │   │   │   ├── v1.data
│   │   │   │   ├── v2.data
│   │   │   │   └── metadata.json
│   │   └── namespace_metadata.json
├── accounts/
│   └── demo_user.json
├── audit_logs/
└── transactions/
```

Each value is stored in its own versioned file, with metadata tracking the history of changes. 

The system uses file locks to prevent concurrent modifications, which is crucial for maintaining data integrity in multi-threaded or multi-process environments. 