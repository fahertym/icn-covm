# ICN-COVM Storage Demo

This demo demonstrates the cooperative storage system with role-based access control (RBAC), resource accounting, and voting for the ICN-COVM project.

## Features Demonstrated

1. **Role-Based Access Control (RBAC)** - Different user roles (admin, member, observer) with varying access permissions
2. **Namespace Organization** - Storage keys organized in logical governance namespaces
3. **Resource Accounting** - Tracking resource usage for each account
4. **Transaction Support** - Atomic operations with commit/rollback capabilities
5. **Versioning** - Storage with version history for auditing
6. **Liquid Democracy** - Vote delegation mechanism
7. **Audit Logging** - Complete activity trail of all storage operations

## Running the Demo

The demo is contained in a standalone Rust file that does not depend on the main ICN-COVM project. This allows you to run it even if the main project has build issues.

### Prerequisites

You need to have Rust and Cargo installed. The demo uses the `serde` and `serde_json` crates for serialization.

### Setup and Run

1. Create a new Cargo project:

```bash
mkdir storage-demo
cd storage-demo
cargo init
```

2. Add the following dependencies to your `Cargo.toml`:

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

3. Replace the contents of `src/main.rs` with the contents of the `demo_storage.rs` file.

4. Run the demo:

```bash
cargo run
```

Alternatively, you can run the standalone file directly with:

```bash
rustc -o storage_demo demo_storage.rs --extern serde=PATH_TO_SERDE_RLIB --extern serde_json=PATH_TO_SERDE_JSON_RLIB
./storage_demo
```

## Demo Scenario

The demo demonstrates a complete cooperative governance workflow:

1. Setup of different user roles (admin, member, observer)
2. Creation of resource accounts with quotas
3. Storage of member data with role-based access control
4. Demonstration of vote delegation (liquid democracy)
5. Creation of a governance proposal
6. Testing of access control as an observer (denied access)
7. Casting of votes with transaction support
8. Checking the version history of a proposal
9. Reviewing resource usage accounting
10. Displaying the audit trail of all storage operations
11. Calculating the final status of a proposal based on votes and delegations

This demonstrates how the storage system supports cooperative governance principles in the ICN-COVM project. 