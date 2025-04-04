# icn-covm Bytecode System

This document explains the bytecode system in icn-covm, including its design, instruction set, and usage.

## Overview

The bytecode system provides an optimized execution layer for icn-covm programs. Instead of directly interpreting the Abstract Syntax Tree (AST) representation, the bytecode compiler translates operations into a more compact, sequential form that can be executed more efficiently.

Benefits of the bytecode system include:
- Faster execution for repeated program runs
- Reduced memory usage during execution
- Potential for further optimizations
- Serializable representation for storage or transmission

## Architecture

The bytecode system consists of three main components:

1. **BytecodeCompiler**: Converts AST operations to bytecode
2. **BytecodeProgram**: Data structure holding compiled bytecode and metadata
3. **BytecodeInterpreter**: Executes the compiled bytecode

### Compilation Process

The compilation process transforms AST operations into a linear sequence of bytecode instructions:

1. **Function Pre-processing**: Identifies function definitions and their entry points
2. **Operation Translation**: Converts each AST operation to one or more bytecode instructions
3. **Control Flow Resolution**: Adds jumps and resolves jump targets for control flow

### Bytecode Format

The bytecode consists of:

- **Instructions**: Individual bytecode operations with their arguments
- **Function Table**: Maps function names to their entry points in the instruction stream
- **Metadata**: Optional debugging information, including the original AST

## Instruction Set

The bytecode instruction set is designed to be simple yet complete. Each instruction performs a specific operation on the VM state.

### Stack Operations

- `Push(value)`: Push a value onto the stack
- `Pop`: Remove the top value from the stack
- `Dup`: Duplicate the top value
- `Swap`: Swap the top two values
- `Over`: Copy the second value to the top

### Arithmetic Operations

- `Add`: Add the top two values
- `Sub`: Subtract the top value from the second
- `Mul`: Multiply the top two values
- `Div`: Divide the second value by the top
- `Mod`: Compute modulo of the second value by the top
- `Negate`: Negate the top value

### Memory Operations

- `Store(name)`: Store the top value in a variable
- `Load(name)`: Load a variable onto the stack

### Control Flow

- `Jump(addr)`: Unconditional jump to an address
- `JumpIfZero(addr)`: Jump if the top value is zero
- `FunctionEntry(name, params)`: Mark a function entry point
- `FunctionExit`: Return from a function
- `Call(name)`: Call a function
- `Return`: Return from the current function
- `Break`: Break out of a loop
- `Continue`: Skip to the next loop iteration

### Logic Operations

- `Eq`: Compare for equality
- `Gt`: Greater than comparison
- `Lt`: Less than comparison
- `Not`: Logical NOT
- `And`: Logical AND
- `Or`: Logical OR

### Debugging and Output

- `Emit(msg)`: Output a message
- `EmitEvent(category, message)`: Emit a categorized event
- `DumpStack`: Display the stack
- `DumpMemory`: Display memory contents
- `DumpState`: Display the VM state
- `AssertTop(value)`: Assert the top value
- `AssertMemory(key, value)`: Assert a memory value
- `AssertEqualStack(depth)`: Assert that values in the stack are equal

## Function Implementation in Bytecode

The bytecode system implements functions using a combination of function entries, calls, returns, and memory management operations.

### Function Definition Compilation

When a function definition is encountered in the AST:

1. The compiler marks the entry point with `FunctionEntry(name, params)`
2. The function body is compiled into a sequence of bytecode instructions
3. The function ends with a `Return` instruction
4. The function entry address is registered in the function table

Example DSL:
```
def add(a, b):
    load a
    load b
    add
    return
```

Compiled bytecode:
```
0000: FunctionEntry("add", ["a", "b"])
0001: Load("a")
0002: Load("b")
0003: Add
0004: Return
```

### Function Call Implementation

Function calls in bytecode involve:

1. **Parameter Setup**: Values are pushed onto the stack
2. **Function Call**: The `Call(name)` instruction saves current execution context and jumps to function entry
3. **Memory Context Management**:
   - The current memory context is saved
   - A new memory context is created for the function
   - Parameters are popped from the stack and stored in the new context
4. **Execution**: The function body is executed
5. **Return**: The `Return` instruction restores the original context and jumps back to the caller

Example DSL:
```
push 5
push 10
call add
```

Compiled bytecode:
```
0100: Push(5.0)
0101: Push(10.0)
0102: Call("add")
```

### Memory Context During Function Calls

The bytecode interpreter maintains a stack of memory contexts for handling function calls:

1. **Context Saving**: When a function is called, the current memory context is pushed onto a context stack
2. **New Context Creation**: A fresh memory context is created for the function
3. **Parameter Binding**: Parameters are popped from the value stack and stored in the new context
4. **Context Restoration**: When a function returns, the previous context is popped and restored

This mechanism ensures proper memory isolation between function calls, preventing unintended variable access or modification across function boundaries.

### Call Stack Management

The bytecode interpreter manages a call stack recording the return addresses:

1. **Call Instruction**: Pushes the next instruction address onto the call stack
2. **Return Instruction**: Pops the return address from the call stack and jumps to it

This enables proper execution flow during nested function calls and ensures each function returns to its correct caller.

## Bytecode Example with Functions

Consider this DSL program using nested functions:

```
def multiply(a, b):
    load a
    load b
    mul
    return

def calculate(x, y):
    load x
    load y
    add
    push 2
    push 3
    call multiply
    add
    return

push 5
push 7
call calculate
```

The compiled bytecode would look like:

```
# Function definitions
0000: FunctionEntry("multiply", ["a", "b"])
0001: Load("a")
0002: Load("b")
0003: Mul
0004: Return

0005: FunctionEntry("calculate", ["x", "y"])
0006: Load("x")
0007: Load("y")
0008: Add
0009: Push(2.0)
0010: Push(3.0)
0011: Call("multiply")
0012: Add
0013: Return

# Main program
0014: Push(5.0)
0015: Push(7.0)
0016: Call("calculate")
```

Execution flow:
1. Push 5 and 7 onto stack
2. Call `calculate` with x=7, y=5
3. In `calculate`, load x and y, add them (result: 12)
4. Push 2 and 3 onto stack
5. Call `multiply` with a=3, b=2
6. In `multiply`, load a and b, multiply them (result: 6)
7. Return to `calculate` with 6 on stack
8. Add 12 + 6 (result: 18)
9. Return to main program with 18 on stack

## Usage

### Command Line

Use the `--bytecode` flag to use bytecode execution:

```bash
cargo run -- --program example.dsl --bytecode
```

Use the `--benchmark` flag to compare AST and bytecode performance:

```bash
cargo run -- --program example.dsl --benchmark
```

### Programmatic Usage

```rust
use nano_cvm::{compiler, bytecode, vm};

// Parse DSL source
let ops = compiler::parse_dsl(source)?;

// Compile to bytecode
let mut compiler = bytecode::BytecodeCompiler::new();
let program = compiler.compile(&ops);

// Execute bytecode
let mut interpreter = bytecode::BytecodeInterpreter::new(program);
interpreter.execute()?;

// Access results
if let Some(result) = interpreter.vm().top() {
    println!("Result: {}", result);
}
```

## Advanced Features

### Bytecode Serialization

The bytecode can be serialized to JSON for storage or transmission:

```rust
// Serialize bytecode to JSON
let bytecode_json = serde_json::to_string(&program)?;

// Deserialize bytecode from JSON
let program: BytecodeProgram = serde_json::from_str(&bytecode_json)?;
```

### Bytecode Inspection

The bytecode program provides methods for inspection:

```rust
// Print bytecode disassembly
println!("{}", program.dump());
```

### Optimization Opportunities

The bytecode compiler could implement several optimizations:

- **Constant Folding**: Pre-compute operations with constant operands
- **Dead Code Elimination**: Remove unreachable code
- **Peephole Optimization**: Replace instruction sequences with more efficient versions
- **Register Allocation**: Use virtual registers instead of stack operations when possible
- **Tail Call Optimization**: Optimize tail-recursive function calls

## Performance Considerations

The bytecode system offers best performance advantages in these scenarios:

1. **Repeated Execution**: When the same program runs multiple times
2. **Loop-Heavy Code**: Programs with many iterations
3. **Function Calls**: Programs with many function calls
4. **Large Programs**: Programs with many operations

The compilation step adds initial overhead, so for very short or one-time programs, the AST interpreter might be faster.

### Function Call Performance

Function calls in bytecode mode benefit from:

1. **Direct Jumps**: Using instruction addresses rather than looking up functions by name
2. **Optimized Memory Context**: More efficient context saving and restoration
3. **Parameter Binding**: More efficient parameter passing
4. **Inlining Potential**: Future optimization could inline small functions

## Common Patterns and Idioms

### Function Return Value Handling

Since function return values are left on the stack:

1. **Single Value Return**: Functions should leave exactly one value on the stack
2. **Multiple Value Return**: To return multiple values, combine them into a single value or use memory
3. **No Return Value**: Push a dummy value (e.g., 0.0) if the function doesn't have a meaningful return

### Error Handling in Functions

Without exception handling, functions use these patterns for errors:

1. **Return Code**: Push a success/error code value (0.0 for success, error code for failures)
2. **Error Checking**: Caller checks the return value before proceeding
3. **Assertions**: Use `AssertTop` or `AssertMemory` to validate critical assumptions

### Function Parameter Ordering

Parameters are pushed onto the stack in reverse order (last parameter first), which can affect code readability. Common approaches include:

1. **Clear Naming**: Use descriptive comments to clarify parameter order
2. **Consistent Order**: Establish conventions for parameter ordering
3. **Limited Parameters**: Keep parameter counts small when possible

## Future Directions

Future enhancements to the bytecode system could include:

- **JIT Compilation**: Compile hot code paths to native code
- **Register-Based VM**: Transition from stack-based to register-based for better performance
- **Optimizing Compiler**: Implement more sophisticated optimizations
- **Bytecode Verification**: Add safety checks for loaded bytecode
- **Cross-Platform Bytecode**: Ensure bytecode compatibility across platforms
- **Function Inlining**: Automatically inline small functions for performance
- **Recursive Tail Calls**: Optimize tail-recursive function calls 