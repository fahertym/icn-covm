# Virtual Machine Module

## Overview

The VM module implements a stack-based virtual machine designed for executing operations in the Cooperative Value Network. It provides a secure, deterministic execution environment for running programs that interact with various subsystems including storage, identity, and governance.

## Architecture

The VM is built with a modular architecture to improve maintainability, testability, and extensibility:

```
                ┌───────────────┐
                │      VM       │
                │ (Orchestrator)│
                └───────┬───────┘
                        │
         ┌──────────────┼──────────────┐
         │              │              │
┌────────▼─────┐ ┌──────▼───────┐ ┌────▼──────────┐
│   StackOps    │ │  MemoryScope  │ │   ExecutorOps  │
│  (VMStack)    │ │  (VMMemory)   │ │  (VMExecution) │
└──────────────┘ └───────────────┘ └────────┬───────┘
                                            │
                      ┌────────────────────┬┴────────────────────┐
                      │                    │                     │
             ┌────────▼────────┐  ┌────────▼────────┐  ┌─────────▼─────────┐
             │  StorageOpHandler│  │GovernanceOpHandler│  │  IdentityOpHandler │
             └─────────────────┘  └─────────────────┘  └───────────────────┘
```

### Core Components

1. **VM**: The main orchestrator that coordinates the components and provides the public API.
2. **Stack**: Manages the execution stack with push/pop operations and value manipulation (implemented by `VMStack`).
3. **Memory**: Handles variable storage, function definitions, scope management and call frames (implemented by `VMMemory`).
4. **Execution**: Implements operation execution logic (implemented by `VMExecution`).

### Operation Handlers

Operations are grouped by domain into specialized handlers:

1. **StorageOpHandler**: Operations related to persistent storage
2. **GovernanceOpHandler**: Operations related to governance and resource management
3. **IdentityOpHandler**: Operations related to identity management
4. **ArithmeticOpHandler**: Operations for arithmetic calculations
5. **ComparisonOpHandler**: Operations for comparison and logical operations

### Benefits of This Design

- **Separation of Concerns**: Each component focuses on a specific responsibility
- **Independent Testing**: Components can be tested independently
- **Extensibility**: New features can be added with minimal impact on other components
- **Maintainability**: Code organization follows logical boundaries
- **Performance Optimization**: Components can be optimized independently

## Core APIs

### VM Creation and Initialization

```rust
// Create a new VM instance
let mut vm = VM::new();

// Configure storage backend
vm.set_storage_backend(storage);

// Set authentication context
vm.set_auth_context(auth_context);

// Set storage namespace
vm.set_namespace("my_namespace");
```

### Program Execution

```rust
// Execute a sequence of operations
let result = vm.execute(&operations);

// Execute a program from a string representation
let result = vm.execute_program(program_string);

// Execute a transaction (with automatic rollback on error)
let result = vm.execute_transaction(&operations);
```

### Stack Manipulation

```rust
// Push a value onto the stack
vm.push(42.0);

// Pop a value from the stack
let value = vm.pop()?;

// Get stack depth
let depth = vm.stack_depth();
```

### Variable Management

```rust
// Store a value in memory
vm.store("variable_name", 42.0);

// Load a value from memory
let value = vm.load("variable_name")?;
```

### Function Management

```rust
// Define a function
vm.define_function("my_function", params, body);

// Call a function
vm.call_function("my_function");
```

## Extended Features

### Typed Values

When the `typed-values` feature is enabled, the VM supports operations on typed values (not just numeric values):

- Number: Floating-point numeric values
- String: Text values
- Boolean: True/false values
- Null: Absence of a value

### Transaction Support

The VM supports transactions with automatic rollback on error:

```rust
// Begin a transaction
vm.begin_transaction();

// Execute operations in the transaction
vm.execute(&operations)?;

// Commit the transaction
vm.commit_transaction();

// Or rollback on error
vm.rollback_transaction();
```

## Usage Examples

### Basic Arithmetic

```rust
use icn_covm::vm::{VM, Op};

let mut vm = VM::new();
let ops = vec![
    Op::Push(5.0),
    Op::Push(3.0),
    Op::Add,     // 5 + 3 = 8
    Op::Push(2.0),
    Op::Mul,     // 8 * 2 = 16
];

vm.execute(&ops).unwrap();
assert_eq!(vm.pop().unwrap(), 16.0);
```

### Conditional Logic

```rust
use icn_covm::vm::{VM, Op};

let mut vm = VM::new();
let ops = vec![
    Op::Push(10.0),
    Op::Push(5.0),
    Op::If {
        condition: vec![Op::Gt],  // 10 > 5?
        then: vec![Op::Push(1.0)],
        else_: Some(vec![Op::Push(0.0)]),
    },
];

vm.execute(&ops).unwrap();
assert_eq!(vm.pop().unwrap(), 1.0);  // Condition was true
```

## Extending the VM

To add new operation types:

1. Add a new variant to the `Op` enum in `types.rs`
2. Add a handler for the operation in the appropriate handler trait
3. Implement the handler in the corresponding implementation
4. Update the VM to dispatch to the new operation handler

See the existing operation handlers in the `ops/` directory for examples. 