# icn-covm Architecture

This document provides an overview of the icn-covm architecture, including its execution flow, compiler phases, and governance operations.

## System Overview

icn-covm is a stack-based virtual machine that executes a Domain-Specific Language (DSL) with governance-oriented operations. The system is designed to be modular, secure, and predictable, with features like memory isolation, recursion protection, and error propagation.

Key components include:
- **Modular compiler** that transforms DSL code into operations
- **Stack-based virtual machine** that executes operations
- **Bytecode compiler and interpreter** for optimized execution
- **Optional typed value system** for enhanced type safety
- **Standard library** of common functions

## Execution Flow

The execution flow of icn-covm consists of the following steps:

1. **Parsing**: The source code is parsed into operations using the compiler module
2. **Execution Mode Selection**: The system chooses between:
   - AST interpretation (default): Direct execution of the parsed operation tree
   - Bytecode execution: First compiles to bytecode, then executes the bytecode

3. **Operation Execution**: Based on the selected mode:
   - AST interpreter processes operations recursively
   - Bytecode interpreter processes instructions sequentially

4. **Result Production**: Execution results are available through:
   - Final stack state (top value is considered the 'return value')
   - Memory state (variables and their values)
   - Emitted events (for logging and debugging)

Here's a diagram of the execution flow:

```
           DSL Source Code
                 │
                 ▼
           Parsing Phase
                 │
                 ▼
         Operation Tree (AST)
                 │
        ┌────────┴────────┐
        │                 │
        ▼                 ▼
   AST Interpreter   Bytecode Compiler
        │                 │
        │                 ▼
        │            Bytecode
        │                 │
        │                 ▼
        │         Bytecode Interpreter
        │                 │
        └────────┬────────┘
                 │
                 ▼
         Execution Results
```

## Compiler Phases

The compiler transforms DSL source code into operations through these phases:

1. **Lexical Analysis**: Identifies tokens in the source code
   - Tokenizes commands, literals, and block structure
   - Handles indentation for block scoping

2. **Parsing**: Transforms tokens into an abstract syntax tree (AST)
   - Basic operations (push, add, etc.)
   - Control flow structures (if, loop, while)
   - Function definitions and calls

3. **Optional Bytecode Compilation**: Transforms the AST into bytecode
   - Flattens the AST into a linear sequence of instructions
   - Resolves jump addresses for control flow
   - Optimizes operations when possible

Here's an example of how a DSL program is transformed:

**DSL Source:**
```
push 10
push 20
add
if:
    push 30
    push 2
    mul
```

**Parsed AST:**
```
[
  Push(10.0),
  Push(20.0),
  Add,
  If {
    condition: [],
    then: [
      Push(30.0),
      Push(2.0),
      Mul
    ],
    else_: None
  }
]
```

**Compiled Bytecode:**
```
0000: Push(10.0)
0001: Push(20.0)
0002: Add
0003: JumpIfZero(7)
0004: Push(30.0)
0005: Push(2.0)
0006: Mul
0007: ...
```

## Virtual Machine

The VM is responsible for executing operations. It maintains:

1. **Stack**: The primary data structure for operation execution
2. **Memory**: Storage for variables (key-value pairs)
3. **Function Table**: Registry of defined functions
4. **Call Frames**: Stack frames for function calls

### AST Interpreter

The AST interpreter executes operations recursively:

1. Each operation is processed based on its type
2. Complex operations (if, loop, etc.) recursively process their sub-operations
3. Function calls create new memory frames for parameter isolation

### Bytecode Interpreter

The bytecode interpreter executes operations sequentially:

1. Maintains a program counter (PC) pointing to the current instruction
2. Processes instructions one at a time, updating the PC
3. Uses jump instructions for control flow
4. Maintains a call stack for function calls

## Typed Value System

When enabled via the `typed-values` feature flag, the VM supports a basic type system:

- **Number**: 64-bit floating-point values
- **Boolean**: true/false values
- **String**: Text values
- **Null**: Represents absence of a value

The typed system includes:
- Type coercion rules for mixed-type operations
- Type-specific operations (e.g., string concatenation)
- Type error handling and reporting

## Governance Operations

The VM includes several governance-oriented operations:

### Match Statements

Match statements allow decision-making based on specific values:

```
push 2
match:
  1:
    emit "Value is 1"
  2:
    emit "Value is 2"
  default:
    emit "Unknown value"
```

### Event Emission

Events can be emitted with categories for structured logging:

```
emitevent "governance" "Proposal 123 executed"
```

### Assertions

Assertions verify invariants during execution:

```
push 10
asserttop 10  # Verifies the top of stack is 10
```

```
push 5
store counter
assertmemory counter 5  # Verifies a memory value
```

### Economic Operations

Economic operations provide resource management capabilities:

```
# Create a new resource
createresource "community_token"

# Mint tokens to an account
mint "community_token" "treasury" 1000.0 "Initial allocation"

# Transfer tokens between accounts
transfer "community_token" "treasury" "project_a" 100.0 "Project funding"

# Check account balance
balance "community_token" "treasury"

# Burn tokens
burn "community_token" "project_a" 25.0 "Spent on community event"
```

These operations integrate with the storage system to maintain persistent balances and resource metadata.

## Bytecode Format

The bytecode format consists of a sequence of instructions with these components:

1. **Header**: Metadata about the bytecode (version, timestamp)
2. **Function Table**: Maps function names to instruction addresses
3. **Instructions**: The actual bytecode instructions
4. **Constant Pool**: Shared constants used by instructions

Each instruction includes:
- **Opcode**: The type of operation
- **Operands**: Additional data needed for the operation

## Benchmarking and Performance

icn-covm provides tools for performance measurement:

1. **Execution Mode Comparison**: Compare AST and bytecode execution times
2. **Memory Usage Tracking**: Monitor stack and memory usage
3. **Instruction Counting**: Track the number of instructions executed

Bytecode execution generally offers better performance for:
- Repeated execution of the same program
- Programs with loops and conditional logic
- Larger programs with many operations

## Security and Safety Features

icn-covm includes several security features:

1. **Memory Isolation**: Function calls have isolated memory frames
2. **Recursion Protection**: Prevents stack overflow from excessive recursion
3. **Bounded Loops**: Ensures loops have explicit bounds
4. **Error Propagation**: Robust error handling with clear messages

## Standard Library

The standard library provides common functions:

```
# Define absolute value
def abs(x):
    load x
    dup
    push 0
    lt
    if:
        negate
    return
```

## Examples

### Simple Calculation
```
push 3
push 4
mul
push 2
add
# Result: 14
```

### Function Definition and Call
```
def add_squared(a b):
    load a
    load a
    mul
    load b
    load b 
    mul
    add
    return

push 3
push 4
call add_squared
# Result: 25 (3² + 4²)
```

### Governance Example
```
# Simple voting system
push 0
store yes_votes
push 0
store no_votes

# Register votes
push 1
store yes_votes

push 1
store no_votes

# Count votes
load yes_votes
load no_votes
gt
if:
    emitevent "governance" "Proposal passed"
else:
    emitevent "governance" "Proposal rejected"
```

### Bytecode Optimization
The bytecode compiler performs optimizations like:
- Constant folding
- Jump optimization
- Instruction combining

For example, the sequence:
```
push 2
push 3
add
```

Could be optimized to:
```
push 5
```

## Future Directions
- JIT compilation for frequently executed code paths
- Serializable bytecode for cross-platform execution
- Extended type system with custom types
- Interoperability with host language (Rust) functions 