# nano-cvm

A lightweight cooperative virtual machine for executing a domain-specific language (DSL) with support for stack-based operations, memory isolation, functions, loops, conditionals, and governance-oriented extensions.

---

## Developer Workflow

Common tasks are available via `make`:

| Command        | Description                                       |
|----------------|---------------------------------------------------|
| `make all`     | Run format check, type check, tests, and dump     |
| `make dump`    | Generate `full_project_dump.txt` for LLM usage    |
| `make demo`    | Run all DSL files in the `demo/` folder           |
| `make clean`   | Remove generated dumps and other temp files       |

You can also run specific demo files manually:

```bash
./scripts/run_demo.sh demo/stdlib/stdlib_demo.dsl
```

---

## Folder Structure

```bash
.
├── demo/                    # Example DSL programs
│   ├── functions/
│   ├── parser/
│   └── stdlib/
├── scripts/                 # Dev utility scripts
├── src/                     # Main VM and compiler implementation
│   └── compiler/            # Modular parser components
├── Cargo.toml               # Rust project manifest
├── Makefile                 # Build automation for common tasks
└── README.md                # You're here
```

---

## Scripts

- `scripts/generate_full_dump.sh` – Recursively dumps all project code into one file for LLM ingestion.
- `scripts/run_demo.sh` – Run one or more `.dsl` files through the VM with optional standard library support.

---

## Quickstart

```bash
# Clone and build
git clone https://github.com/your-user/nano-cvm
cd nano-cvm
cargo build

# Run a demo program
cargo run -- -p demo/stdlib/stdlib_demo.dsl --stdlib
```

---

## License

MIT © Matt Faherty and contributors

