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
- Comprehensive test coverage

## Integration Steps

### 1. Add a Command Line Option

First, add a command-line flag to specify the storage backend and location:

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
    let storage = match storage_backend {
        "file" => {
            if verbose {
                println!("Using FileStorage backend at: {}", storage_path);
            }
            let storage = FileStorage::new(storage_path)
                .map_err(|e| AppError::Other(format!("Failed to create file storage: {}", e)))?;
            prepare_storage(&auth_context, storage)?
        },
        _ => {
            if verbose {
                println!("Using in-memory storage backend");
            }
            prepare_storage(&auth_context, InMemoryStorage::new())?
        }
    };
    
    // ... rest of function ...
}

// Helper to prepare any storage backend
fn prepare_storage<T: StorageBackend + 'static>(
    auth: &AuthContext, 
    mut storage: T
) -> Result<T, AppError> {
    // Create user account
    if let Err(e) = storage.create_account(Some(auth), &auth.user_id, 1024 * 1024) {
        println!("Warning: Failed to create account: {:?}", e);
    }
    
    // Create namespace
    if let Err(e) = storage.create_namespace(Some(auth), "demo", 1024 * 1024, None) {
        println!("Warning: Failed to create namespace: {:?}", e);
    }
    
    Ok(storage)
}
```

### 3. Update Command Handling

Update the command handling code to pass the storage backend options:

```rust
// In main.rs main() function
match matches.subcommand() {
    Some(("run", run_matches)) => {
        // Get program file and verbosity setting
        let program_path = run_matches.get_one::<String>("program").unwrap();
        let verbose = run_matches.get_flag("verbose");
        let use_bytecode = run_matches.get_flag("bytecode");
        let benchmark = run_matches.get_flag("benchmark");
        let use_stdlib = run_matches.get_flag("stdlib");
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

### 4. Update Interactive and Benchmark Modes

Apply similar changes to the `run_interactive` and `run_benchmark` functions to support file-based storage.

## Example Usage

Once implemented, users can run programs with persistent storage:

```bash
# Run with file storage
cargo run -- run --program demo/storage/persistent_counter.dsl --verbose --storage-backend file --storage-path ./data/storage

# Run again - counter value will persist!
cargo run -- run --program demo/storage/persistent_counter.dsl --verbose --storage-backend file --storage-path ./data/storage
```

## Testing

To test the integration:

1. Run the persistent counter example with file storage
2. Verify that the counter value increments across runs
3. Examine the storage directory to see the stored data files

## Troubleshooting

Common issues:

- **Permission errors**: Ensure the storage directory is writable
- **Missing directories**: The FileStorage implementation should create required directories
- **Transaction failures**: Check for disk space issues if transaction operations fail

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