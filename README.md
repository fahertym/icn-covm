# icn-covm

A lightweight cooperative virtual machine for executing a domain-specific language (DSL) with support for stack-based operations, memory isolation, functions, loops, conditionals, and governance-oriented extensions.

## New Features

- **Bytecode Compiler & Interpreter**: Faster execution through bytecode compilation
- **Storage Integration**: Persistent storage with authentication, roles, and transactions
- **Identity System**: Authentication, role-based access control, and signature verification
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
cargo run -- --program demo/functions/factorial.dsl --stdlib

# Run with bytecode compiler and interpreter
cargo run -- --program demo/functions/factorial.dsl --stdlib --bytecode

# Run benchmarks comparing both execution modes
cargo run -- --program demo/benchmark/fibonacci.dsl --benchmark
```

---

## Execution Modes

nano-cvm supports two execution modes:

1. **AST Interpreter** (default): Directly interprets the parsed operation tree
2. **Bytecode Execution**: Compiles operations to bytecode for faster execution

To use the bytecode mode, add the `--bytecode` flag:

```bash
cargo run -- --program your_program.dsl --bytecode
```

To compare performance between modes, use the `--benchmark` flag:

```bash
cargo run -- --program demo/benchmark/loop.dsl --benchmark
```

---

## Typed Value System

The VM includes JSON-based typed values for storage operations:

- Numbers (f64)
- Booleans (true/false)
- Strings (text)
- Null (absence of a value)

These types are handled through JSON serialization in storage operations like `StorePTyped` and `LoadPTyped`, ensuring type safety when storing and retrieving data.

---

## Documentation

Comprehensive documentation is available:

- **Command-line Help**: `cargo run -- --help`
- **API Documentation**: `make doc` or `cargo doc --open`
- **Architecture Overview**: `docs/architecture.md`
- **Bytecode System**: `docs/bytecode.md`
- **Storage System**: `docs/icn_storage_architecture.md`
- **Identity System**: `docs/identity_system_plan.md`

---

## Folder Structure

```bash
.
├── demo/                    # Example DSL programs
│   ├── benchmark/           # Performance benchmark programs
│   ├── functions/           # Function examples
│   ├── parser/              # Parser test cases
│   ├── stdlib/              # Standard library demos
│   └── storage/             # Storage and identity demos
├── docs/                    # Documentation
│   ├── architecture.md      # System architecture overview
│   ├── bytecode.md          # Bytecode system documentation
│   ├── icn_storage_architecture.md # Storage system documentation
│   └── identity_system_plan.md # Identity system documentation
├── scripts/                 # Dev utility scripts
├── src/                     # Source code
│   ├── bytecode.rs          # Bytecode compiler and interpreter
│   ├── compiler/            # Modular parser components
│   ├── events.rs            # Event system for logging
│   ├── storage/             # Storage implementations
│   ├── lib.rs               # Library exports
│   └── vm.rs                # Virtual machine implementation
├── Cargo.toml               # Rust project manifest
├── Makefile                 # Build automation for common tasks
└── README.md                # You're here
```

---

## Interactive REPL

nano-cvm includes an interactive REPL mode for experimentation:

```bash
# Start REPL with AST interpreter
cargo run -- --interactive

# Start REPL with bytecode execution
cargo run -- --interactive --bytecode
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

