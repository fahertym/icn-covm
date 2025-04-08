# ICN-COVM

The Intercooperative Network Cooperative Virtual Machine (ICN-COVM) is a toolkit and runtime for democratic governance and interoperation between cooperative organizations.

## Current Status: v0.7.0 "Federation Foundation"

The ICN-COVM project is now focused on federation capabilities, allowing multiple cooperative nodes to discover and communicate with each other securely across the network.

Key features in the current release:
- Foundation for federated governance using libp2p networking
- Persistent storage with enhanced transaction support and error handling
- Identity-based access control
- Democratic voting mechanisms (RankedVote, LiquidDelegation)

## Quick Start

To get started with ICN-COVM:

```bash
# Clone the repository
git clone https://github.com/cooperative-computing/icn-covm.git
cd icn-covm

# Build the project
cargo build

# Run the tests
cargo test

# Try a demo program
cargo run -- run demo/ranked_vote/demo.icn
```

## Federation Support

ICN-COVM now supports federation between multiple nodes:

```bash
# Run a node in federation mode
cargo run -- federation --listen /ip4/0.0.0.0/tcp/4001

# Connect to another node
cargo run -- federation --connect /ip4/192.168.1.100/tcp/4001/p2p/QmNodePeerId
```

For more details on federation, see the [Federation Guide](docs/federation_guide.md).

## Project Documentation

* [Architecture Overview](docs/architecture.md)
* [Storage Integration Guide](docs/storage_integration_guide.md)
* [Federation Guide](docs/federation_guide.md)
* [Roadmap](docs/roadmap.md)
* [Contributing](CONTRIBUTING.md)

## Features

* **Democratic Governance**: Built-in primitives for various forms of voting (ranked-choice, delegative, etc.)
* **Identity & Authorization**: Cryptographic identity verification and role-based access control
* **Persistent Storage**: Maintain state between executions with transaction support
* **Federation**: Connect multiple ICN-COVM nodes into a cooperative network
* **Programming Language**: A specialized DSL designed for governance operations

## License

ICN-COVM is licensed under the [MIT License](LICENSE).

## Contributing

We welcome contributions! Please see our [contributing guidelines](CONTRIBUTING.md) for details.

# icn-covm

A lightweight cooperative virtual machine for executing a domain-specific language (DSL) with support for stack-based operations, memory isolation, functions, loops, conditionals, and governance-oriented extensions.

## Features

- **Bytecode Compiler & Interpreter**: Faster execution through bytecode compilation
- **Typed Value System**: Optional support for multiple data types (numbers, booleans, strings)
- **Identity-Aware Execution**: Support for authenticated operations and permission checks
- **Persistent Storage**: Multiple backends (in-memory, file) with versioning, transactions, and improved error handling
- **Economic Operations**: Token-based resource creation, transfer, and management
- **Federation Layer**: Peer-to-peer networking using libp2p for node discovery and communication
- **Comprehensive Documentation**: Improved inline documentation and architecture docs
- **Performance Benchmarking**: Compare AST interpretation vs bytecode execution

---

## Developer Workflow

Common tasks are available via `make`:

| Command        | Description                                       |
|----------------|---------------------------------------------------|
| `make all`     | Run format check, linting, and tests              |
| `make build`   | Build the project                                 |
| `make test`    | Run tests                                         |
| `make clippy`  | Run linter                                        |
| `make fmt`     | Check code formatting                             |
| `make doc`     | Build and open documentation                      |
| `make benchmark`| Run performance benchmarks                       |
| `make clean`   | Remove build artifacts                            |

You can also run specific demo files manually:

```bash
# Run with AST interpreter (default)
cargo run -- run --program demo/functions/factorial.dsl --stdlib

# Run with bytecode compiler and interpreter
cargo run -- run --program demo/functions/factorial.dsl --stdlib --bytecode

# Run benchmarks comparing both execution modes
cargo run -- run --program demo/benchmark/fibonacci.dsl --benchmark
```

---

## Execution Modes

icn-covm supports two execution modes:

1. **AST Interpreter** (default): Directly interprets the parsed operation tree
2. **Bytecode Execution**: Compiles operations to bytecode for faster execution

To use the bytecode mode, add the `--bytecode` flag:

```bash
cargo run -- run --program your_program.dsl --bytecode
```

To compare performance between modes, use the `--benchmark` flag:

```bash
cargo run -- run --program demo/benchmark/loop.dsl --benchmark
```

---

## Typed Value System

The typed value system extends icn-covm with support for multiple data types:

- Numbers (f64)
- Booleans (true/false)
- Strings (text)
- Null (absence of a value)

This feature is disabled by default for backward compatibility. To enable it:

```bash
# Build with typed values support
cargo build --features typed-values

# Run with typed values support
cargo run --features typed-values -- run --program demo/typed/string_operations.dsl
```

---

## Identity System

The identity system provides secure authentication and authorization:

- Identity verification with cryptographic signatures
- Role-based access control for operations
- Delegation chains for actions on behalf of others
- Integration with storage backends for permission validation

To use the identity system:

```bash
# Run a program with an identity context
cargo run -- run --program demo/identity/basic_auth.dsl --identity member1

# Run with identity and roles
cargo run -- run --program demo/identity/role_check.dsl --identity member1 --roles admin,member
```

---

## Storage System

The storage system enables persistent state across executions:

- Key-value storage with namespaces
- File-based and in-memory storage backends
- Transaction support for atomic operations
- Identity-aware access control with `Option<&AuthContext>` API
- File locking and robust error handling

Storage backends can be specified at runtime:

```bash
# Use in-memory storage (default)
cargo run -- run --program demo/storage/persistent_counter.dsl --storage-backend memory

# Use file-based storage with specified path
cargo run -- run --program demo/storage/persistent_counter.dsl --storage-backend file --storage-path ./filestorage
```

Storage inspection commands:

```bash
# List keys in a namespace
cargo run -- storage list-keys demo --storage-backend file --storage-path ./storage

# Get a value from storage
cargo run -- storage get-value demo counter --storage-backend file --storage-path ./storage
```

To learn more about the storage system, see the [Storage System Documentation](docs/storage.md).

---

## Federation Layer

The federation layer enables communication between ICN-COVM nodes:

- Peer-to-peer networking using libp2p
- Node discovery via Kademlia DHT and mDNS
- Secure channels with Noise protocol
- Basic message exchange

To run a node with federation enabled:

```bash
# Run a node with federation enabled
cargo run -- run --enable-federation --federation-port 8000 --node-name "node1"

# Run a node that connects to a bootstrap node
cargo run -- run --enable-federation --federation-port 8001 --bootstrap-nodes "/ip4/192.168.1.1/tcp/8000/p2p/12D3KooWX...Z9PcBJP5" --node-name "node2"
```

For multi-node testing, see the [Federation Testing Environment](README_FEDERATION.md).

---

## Documentation

Comprehensive documentation is available:

- **Command-line Help**: `cargo run -- --help`
- **API Documentation**: `make doc` or `cargo doc --open`
- **Architecture Overview**: `docs/architecture.md`
- **Bytecode System**: `docs/bytecode.md`
- **Identity System**: `docs/identity.md`
- **Storage System**: `docs/storage.md`
- **Federation Layer**: `docs/federation.md`
- **Typed Value System**: `docs/typed-values.md`

---

## Folder Structure

```bash
.
├── demo/                    # Example DSL programs
│   ├── benchmark/           # Performance benchmark programs
│   ├── economic/            # Economic operations examples
│   ├── functions/           # Function examples
│   ├── governance/          # Governance primitive examples
│   ├── identity/            # Identity verification examples
│   ├── parser/              # Parser test cases
│   ├── storage/             # Storage operation examples
│   ├── stdlib/              # Standard library demos
│   └── typed/               # Typed value system demos
├── docs/                    # Documentation
│   ├── architecture.md      # System architecture overview
│   ├── bytecode.md          # Bytecode system documentation
│   ├── economic_operations.md # Economic operations documentation
│   ├── federation.md        # Federation layer documentation
│   ├── governance.md        # Governance primitives documentation
│   ├── identity.md          # Identity system documentation
│   ├── storage.md           # Storage system documentation
│   └── typed-values.md      # Typed value system documentation
├── scripts/                 # Dev utility scripts
├── src/                     # Source code
│   ├── bytecode.rs          # Bytecode compiler and interpreter
│   ├── compiler/            # Modular parser components
│   ├── events.rs            # Event system for logging
│   ├── federation/          # Networking and node communication
│   ├── identity/            # Identity and authorization system
│   ├── lib.rs               # Library exports
│   ├── main.rs              # Command-line interface
│   ├── storage/             # Storage system implementations
│   ├── typed.rs             # Typed value system (feature-flagged)
│   └── vm.rs                # Virtual machine implementation
├── Cargo.toml               # Rust project manifest
├── Makefile                 # Build automation for common tasks
└── README.md                # You're here
```

---

## Interactive REPL

icn-covm includes an interactive REPL mode for experimentation:

```bash
# Start REPL with AST interpreter
cargo run -- run --interactive

# Start REPL with bytecode execution
cargo run -- run --interactive --bytecode

# Start REPL with identity context
cargo run -- run --interactive --identity member1
```

REPL commands:
- `help` - Show available commands
- `stack` - Show current stack contents
- `memory` - Show memory contents
- `mode ast` - Switch to AST interpreter mode
- `mode bytecode` - Switch to bytecode mode
- `exit` or `quit` - Exit the REPL

---

## License

MIT © Matt Faherty and contributors

---

> **Note:** This project was formerly called `nano-cvm`. The rename reflects its expanded role as the core virtual machine of the Intercooperative Network (ICN).

