# Nano-CVM: Cooperative Virtual Machine

A secure, stack-based virtual machine in Rust for the Intercooperative Network (ICN), featuring governance-inspired operations, memory-isolated execution, and a custom DSL parser.

## Features

- **Stack-based architecture**: Simple and secure execution model
- **Memory isolation**: Each function call has isolated memory space
- **Custom DSL**: Human-readable text format for programs
- **JSON serialization**: Machine-readable format for program interchange
- **Governance-inspired opcodes**: Special operations for cooperative decision processes

## Opcodes

### Core Operations
- Basic arithmetic: `Add`, `Sub`, `Mul`, `Div`, `Mod`
- Stack manipulation: `Push`, `Pop`, `Dup`, `Swap`, `Over`
- Memory: `Store`, `Load`
- Control flow: `If`, `Loop`, `While`, `Return`
- Functions: `Def`, `Call`

### Governance-Inspired Operations
- **Match**: Pattern-style branching for multi-way decisions
- **Break** and **Continue**: Fine-grained loop control
- **EmitEvent**: Trigger governance logs with categorized messages
- **AssertEqualStack**: Validate consensus/coordination by ensuring stack uniformity

## Usage

### Installation

```bash
cargo build --release
```

### Running Programs

The VM supports both the DSL text format and JSON serialized programs:

```bash
# Run a DSL program
./target/release/nano-cvm --program program.dsl

# Run a JSON program
./target/release/nano-cvm --program program.json

# Verbose output
./target/release/nano-cvm --program program.dsl --verbose
```

## DSL Syntax

```
# Define a function
def function_name(param1, param2):
    # Function body
    load param1
    load param2
    add
    return

# Invoke Match operation
match:
    value:
        # Instructions that leave a value on the stack
        push 1
        push 2
        add
    case 3:
        # Execute if value is 3
        push 30
    case 4:
        # Execute if value is 4
        push 40
    default:
        # Execute if no case matches
        push 999

# Break and Continue
while:
    push 1  # Infinite loop
    if:
        # Condition to break
        break
    if:
        # Condition to skip to next iteration
        continue

# Emit governance events
emitevent "category" "message"

# Validate stack consensus
push 42
push 42
push 42
assertequalstack 3  # Verify last 3 values are equal
```

## Security Features

- No `unsafe` Rust code
- Memory isolation between function calls
- Comprehensive stack underflow checks
- Detailed error messages
- Limited recursion depth protection

## Example

See `program.dsl` and `program.json` for example programs demonstrating the governance-inspired opcodes.

## License

MIT License 