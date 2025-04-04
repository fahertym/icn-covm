# ICN-COVM Virtual Machine (VM) Documentation

## Overview

The ICN Cooperative Virtual Machine (ICN-COVM) is a stack-based virtual machine designed to execute Domain-Specific Language (DSL) operations for cooperative governance mechanisms. This document focuses on the VM's architecture, operation handling, and execution model with special emphasis on function execution.

## VM Architecture

### Core Components

The VM consists of these key components:

1. **Stack**: A LIFO data structure for temporary values
2. **Memory**: A key-value store for variables (analogous to RAM)
3. **Function Table**: Registry of function definitions (name, parameters, body)
4. **Events System**: Mechanism for emitting messages and events
5. **Persistent Storage**: Interface for durable storage (optional)
6. **Authentication Context**: Identity and permission management (optional)

### Data Types

The VM primarily operates with 64-bit floating-point values (`f64`) for simplicity and consistency. In this system:

- `0.0` represents TRUE in conditional operations (a conversion from traditional boolean logic)
- Any non-zero value represents FALSE
- Numbers are used both for computation and for logical operations

## Operation Execution

The VM executes operations sequentially, with complex operations (like conditionals and functions) managing their own nested execution flow.

### Basic Operation Flow

1. Operation is evaluated based on its type
2. Required values are popped from the stack
3. The operation is performed
4. Results (if any) are pushed back onto the stack

### Stack Manipulation

The stack is the primary workspace for the VM, where:

- `Push` adds values
- Arithmetic operations consume values and produce results
- Conditional operations evaluate based on stack values
- Function calls pull parameters from the stack and return results to it

## Function Execution Model

### Function Definition (`Op::Def`)

When a function is defined using the `def` operation:

1. The function name, parameter list, and operation body are registered in the VM's function table
2. No immediate execution occurs

Example:
```
def add_numbers(a, b):
    load a
    load b
    add
    return
```

### Function Call Flow (`Op::Call`)

When a function is called:

1. **Parameter Preparation**: Parameters are pushed onto the stack in reverse order (last parameter first)
2. **Call Operation**: The function name is specified in the `call` operation
3. **Memory Isolation**:
   - The current memory context is saved
   - A new function-local memory context is created
4. **Parameter Binding**:
   - Values are popped from the stack
   - Each value is bound to its corresponding parameter name in the function's memory context
5. **Function Execution**:
   - The function body operations are executed
   - Local variables exist only in this memory context
6. **Return Handling**:
   - Function execution continues until a `return` operation or the end of the function body
   - The top value on the stack becomes the function's return value
7. **Memory Restoration**:
   - The original memory context is restored
   - The function's return value remains on the stack

```
┌────────────────────────────────────────────────────────────────────┐
│                        Function Call Execution                      │
├────────────────────────────────────────────────────────────────────┤
│                                                                    │
│  Initial state:                                                    │
│  ┌──────────┐      ┌───────────────┐                               │
│  │ Stack    │      │ Memory        │                               │
│  ├──────────┤      ├───────────────┤                               │
│  │ param_n  │      │ x: 10         │                               │
│  │ ...      │      │ y: 20         │                               │
│  │ param_1  │      │ ...           │                               │
│  └──────────┘      └───────────────┘                               │
│                                                                    │
│  1. Call function:                                                 │
│     - Pop parameter values from stack                              │
│     - Save original memory context                                 │
│                                                                    │
│  2. Setup function memory context:                                 │
│  ┌──────────┐      ┌───────────────┐                               │
│  │ Stack    │      │ Memory        │                               │
│  ├──────────┤      ├───────────────┤                               │
│  │          │      │ param_1: val1 │                               │
│  │          │      │ param_2: val2 │                               │
│  │          │      │ ...           │                               │
│  └──────────┘      └───────────────┘                               │
│                                                                    │
│  3. Execute function body:                                         │
│     - Operations manipulate stack and function-local memory        │
│     - Local variables only exist in function memory                │
│                                                                    │
│  4. Return from function:                                          │
│  ┌──────────┐      ┌───────────────┐                               │
│  │ Stack    │      │ Memory        │                               │
│  ├──────────┤      ├───────────────┤                               │
│  │ result   │      │ x: 10         │                               │
│  └──────────┘      │ y: 20         │                               │
│                    │ ...           │                               │
│                    └───────────────┘                               │
│                                                                    │
└────────────────────────────────────────────────────────────────────┘
```

### Nested Function Calls

The VM supports nested function calls with proper memory isolation:

1. Each function call creates its own isolated memory context
2. Nested calls further stack these contexts
3. When a function returns, its memory context is discarded and the caller's context is restored
4. All memory contexts form an implicit call stack through this nesting

### Memory Isolation and Scoping

Memory isolation is a critical aspect of the function execution model:

- **Global Memory**: Variables defined outside any function
- **Function Memory**: Variables and parameters local to a function call
- **Nested Memory**: Each nested function call has its own isolated memory
- **Memory Persistence**: When a function returns, its local memory is discarded

The VM ensures proper isolation by:
- Saving the caller's memory context before a function call
- Creating a new memory context for each function call
- Restoring the original memory context when the function returns

This mechanism prevents:
- Functions from accidentally modifying variables in the caller's scope
- Memory leaks from function-local variables
- Variable shadowing conflicts between different function calls

## Conditional Logic

The VM's conditional operations follow a "zero is true" convention:

- `0.0` is considered TRUE
- Any non-zero value is considered FALSE

This applies to all conditional operations:

- `If` executes the "then" branch if condition evaluates to `0.0`
- `While` continues looping while condition evaluates to `0.0`
- Comparison operators (`Gt`, `Lt`, `Eq`) push `0.0` for true conditions

Example:
```
push 10
push 5
gt      # Pushes 0.0 because 10 > 5 is true
if:
    emit "This message is shown for true condition"
end
```

## Error Handling

The VM provides comprehensive error handling for various failure scenarios:

- **StackUnderflow**: Not enough values on stack for an operation
- **VariableNotFound**: Attempt to access an undefined variable
- **FunctionNotFound**: Call to an undefined function
- **DivisionByZero**: Arithmetic division by zero
- **InvalidCondition**: Malformed condition in control flow
- **AssertionFailed**: Failed assertion in program

Errors terminate execution unless specifically caught and handled.

## Storage Operations

The VM supports both transient and persistent storage:

- **Transient (Memory)**: Using `store` and `load` operations
- **Persistent**: Using `storep` and `loadp` operations

The persistent storage implementation:
1. Verifies the authentication context
2. Checks permissions for the storage operation
3. Performs the storage operation through the configured backend

## Event Emission

The VM provides mechanisms for emitting events:

- `emit`: Outputs a simple message
- `emitevent`: Emits a categorized event (category, message)
- Various debug operations: `dumpstack`, `dumpmemory`, `dumpstate`

These events can be used for debugging, logging, or integrating with external systems.

## Governance Operations

The VM includes specialized operations for cooperative governance:

- **RankedVote**: Implements ranked-choice voting with instant runoff
- **LiquidDelegate**: Enables delegation of voting power
- **VoteThreshold**: Verifies sufficient support for a proposal
- **QuorumThreshold**: Confirms adequate participation

## Identity and Authentication

The VM supports identity verification and access control:

- **AuthContext**: Provides identity and permission information
- **VerifyIdentity**: Validates cryptographic signatures
- **CheckMembership**: Verifies membership in specific namespaces
- **CheckDelegation**: Confirms delegation relationships

## Bytecode Execution

For optimized performance, operations can be compiled to bytecode:

1. The AST representation is converted to linear bytecode
2. Control flow is implemented using jumps
3. Function calls use explicit stack frames
4. Execution is sequential rather than recursive

## Best Practices

### Function Design

1. **Parameter Ordering**: Push parameters in reverse order (last parameter first)
2. **Return Values**: Leave exactly one value on the stack before returning
3. **Memory Isolation**: Rely on memory isolation for clean function boundaries
4. **Error Handling**: Check for edge cases and handle errors gracefully

### Memory Management

1. **Scope Awareness**: Remember that function-local variables exist only during the function call
2. **Naming Conventions**: Use clear naming to avoid confusion between global and local variables
3. **Stack Hygiene**: Keep the stack clean, don't leave unnecessary values

### Performance Considerations

1. **Stack Operations**: Minimize unnecessary stack operations
2. **Function Size**: Keep functions reasonably sized for better performance
3. **Memory Usage**: Be mindful of memory consumption in large loops
4. **Recursion**: Be cautious with recursion depth to avoid stack overflow 