# Cursor AI Rules for `nano-cvm`

## 🦀 Rust Code Standards

- Always write safe, idiomatic Rust
- Use pattern matching and enums over magic values
- Prefer small, focused functions and clean match arms
- Derive `Debug` and `Serialize`/`Deserialize` for data structures where appropriate
- Avoid unsafe blocks unless explicitly instructed

## 🧠 Virtual Machine Design

- The VM uses a stack-based architecture with memory and control flow
- All opcodes (`Op` enum) must be serializable with Serde
- Every new `Op` should include:
  - Execution logic inside `execute()`
  - A JSON-compatible serialization format
  - Unit tests in `#[cfg(test)]` module
  - Optional usage in `program.json` for demonstration

## 🧪 Testing Expectations

- Every new operation must be tested:
  - Functionality (`test_opcode_name`)
  - Edge cases (stack underflow, invalid memory, etc.)
  - Nested/recursive use if applicable (e.g., loops, conditionals)
- Do not add new logic without corresponding tests unless explicitly instructed

## 🔄 Workflow Rules

- New logic should generally be added on a feature branch
- Prompt user to write clean, concise commit messages
- Encourage testing before merge
- Use semantic and descriptive branch names (e.g., `feature/emit`, `fix/loop-bug`)

## 📝 JSON DSL Rules

- JSON input programs must match the `Vec<Op>` structure
- Each opcode should have a clear JSON form (e.g., `{ "Emit": "hello" }`)
- Maintain simplicity and readability of example JSON programs
- Nesting (e.g., `IfZero`, `Loop`) should be kept consistent and easy to interpret

## 🗣 Prompt Behavior

- When user asks to add a new `Op`, always include:
  - Enum addition
  - Execute match logic
  - Serde support
  - Unit tests
  - JSON usage demo
- Ask the user if they want CLI, REPL, export, or interactive features when appropriate
- Be mindful of execution state (stack/memory) when designing new logic

## 📦 Running the Program

To run the program, use the following command:

```bash
cargo run -p icn-covm
```

This will start the calculation and output the final result.

## 📁 Project Structure

Always work with the `crates/icn-covm/src/` directory which contains the working code, not the legacy `src/` directory at the root.

## 🚫 Legacy Structure Cleanup

- Do not create or reference a top-level `./src/` directory.
- All source code lives under `crates/icn-covm/` and `crates/icn-ledger/`.
- Always operate within the workspace crates declared in the root `Cargo.toml`.

## 🧭 Cursor Code Navigation Rules

- All VM logic: `crates/icn-covm/src/vm/`
- CLI commands: `crates/icn-covm/src/cli/`
- Governance logic: `crates/icn-covm/src/governance/`
- Federation: `crates/icn-covm/src/federation/`
- Typed values: `crates/icn-covm/src/typed.rs`

Cursor agents should avoid:
- Opening or writing to `./src/`
- Creating duplicate main.rs or lib.rs files outside the crates
