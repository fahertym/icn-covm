# TypedValue System

## Overview

The TypedValue system provides rich data types beyond simple numeric values for the Cooperative Value Network. It allows programs to work with strings, booleans, and null values in addition to numbers, enabling more expressive and type-safe operations.

## Core Types

The `TypedValue` enum represents different types of values that can be manipulated by the VM:

```rust
pub enum TypedValue {
    /// Numeric value (floating point)
    Number(f64),
    
    /// String value
    String(String),
    
    /// Boolean value
    Bool(bool),
    
    /// Null value (absence of a value)
    Null,
}
```

## Features

1. **Type Safety**
   - Runtime type checking
   - Proper error handling for type mismatches
   - Safe conversions between compatible types

2. **Rich Operations**
   - Type-aware arithmetic operations
   - String manipulation
   - Boolean logic
   - Serialization/deserialization support

3. **Integration**
   - Stack operations with typed values
   - Memory storage of typed values
   - Type-aware conditionals and control flow
   - Typed storage operations

4. **Pattern Matching**
   - Match on value types
   - Type-based conditional execution
   - Default cases for type handling

## API Examples

### Creating TypedValues

```rust
// Create typed values
let num_val = TypedValue::Number(42.0);
let str_val = TypedValue::String("Hello, world!".to_string());
let bool_val = TypedValue::Bool(true);
let null_val = TypedValue::Null;

// Create from literals using From traits
let num_val: TypedValue = 42.0.into();
let str_val: TypedValue = "Hello".into();
let bool_val: TypedValue = true.into();
```

### Operations

```rust
// Arithmetic
let sum = num_val.add(&TypedValue::Number(8.0))?;  // TypedValue::Number(50.0)

// String concatenation
let concat = str_val.add(&TypedValue::String(" Cooperative".to_string()))?;
// TypedValue::String("Hello, world! Cooperative")

// Boolean operations
let and_result = bool_val.and(&TypedValue::Bool(false))?;  // TypedValue::Bool(false)
let not_result = bool_val.not()?;  // TypedValue::Bool(false)
```

### Comparisons

```rust
// Equality
let eq_result = num_val.equals(&TypedValue::Number(42.0))?;  // TypedValue::Bool(true)

// Greater/Less than
let gt_result = num_val.greater_than(&TypedValue::Number(30.0))?;  // TypedValue::Bool(true)
let lt_result = num_val.less_than(&TypedValue::Number(50.0))?;  // TypedValue::Bool(true)
```

### Type Conversion

```rust
// Explicit conversion
let num_as_str = num_val.to_string()?;  // TypedValue::String("42")
let str_as_num = TypedValue::String("123".to_string()).to_number()?;  // TypedValue::Number(123.0)
let bool_as_num = bool_val.to_number()?;  // TypedValue::Number(1.0)

// Get underlying values
if let TypedValue::Number(n) = num_val {
    println!("Got number: {}", n);
}

// Try to get as a specific type
let n = num_val.as_number()?;  // 42.0
let s = str_val.as_string()?;  // "Hello, world!"
```

### Error Handling

```rust
// Type mismatch
match str_val.to_number() {
    Ok(num) => println!("Converted to number: {}", num),
    Err(err) => println!("Conversion error: {}", err),  // Will show error
}

// Invalid operation
match str_val.divide(&num_val) {
    Ok(result) => println!("Result: {}", result),
    Err(err) => println!("Operation error: {}", err),  // Will show error
}
```

## VM Integration

When the `typed-values` feature is enabled, the VM operations work with TypedValue instead of raw f64 values:

```rust
// VM with typed values
let mut vm = VM::new();

// Push different types onto the stack
vm.push(TypedValue::String("hello".to_string()));
vm.push(TypedValue::Number(42.0));
vm.push(TypedValue::Bool(true));

// Execute typed operations
let ops = vec![
    Op::Push(TypedValue::Number(5.0)),
    Op::Push(TypedValue::Number(3.0)),
    Op::Add,  // Works with numbers, strings, etc.
    Op::Push(TypedValue::String(" World".to_string())),
    Op::Store("greeting".to_string()),
];

vm.execute(&ops)?;
```

## DSL Integration

The DSL parser can emit typed values when the feature is enabled:

```json
{
  "ops": [
    {"push": 42},
    {"push": "hello"},
    {"push": true},
    {"push": null},
    {"add": null},
    {"store": "result"}
  ]
}
```

This parses to:

```rust
vec![
    Op::Push(TypedValue::Number(42.0)),
    Op::Push(TypedValue::String("hello".to_string())),
    Op::Push(TypedValue::Bool(true)),
    Op::Push(TypedValue::Null),
    Op::Add,
    Op::Store("result".to_string()),
]
```

## Type Coercion

TypedValue implements automatic coercion for certain operations:

```rust
// String + Number concatenates as strings
let result = TypedValue::String("Count: ".to_string())
    .add(&TypedValue::Number(5.0))?;
// Result: TypedValue::String("Count: 5")

// Boolean conditions in if/while statements
// Any non-zero number or non-empty string is truthy
let condition = TypedValue::String("hello".to_string());
if vm.is_truthy(&condition) {
    // This will execute
}

// Null is always falsy
let null_condition = TypedValue::Null;
if !vm.is_truthy(&null_condition) {
    // This will execute
}
```

## Storage Integration

TypedValue can be stored and retrieved from storage:

```rust
// Store different types
storage.store_typed("key1", TypedValue::Number(42.0), auth, "namespace")?;
storage.store_typed("key2", TypedValue::String("value".to_string()), auth, "namespace")?;
storage.store_typed("key3", TypedValue::Bool(true), auth, "namespace")?;

// Load a typed value
let value = storage.load_typed("key1", auth, "namespace")?;
```

## Example: Rich Type Operations

```rust
let mut vm = VM::new();

// A simple program using different types
let ops = vec![
    // Push a greeting string
    Op::Push(TypedValue::String("Hello, ".to_string())),
    
    // Push a name
    Op::Push(TypedValue::String("Alice".to_string())),
    
    // Concatenate them
    Op::Add,
    
    // Store the greeting
    Op::Store("greeting".to_string()),
    
    // Calculate age next year
    Op::Push(TypedValue::Number(30.0)),
    Op::Push(TypedValue::Number(1.0)),
    Op::Add,
    Op::Store("next_year_age".to_string()),
    
    // Check if age > 18
    Op::Load("next_year_age".to_string()),
    Op::Push(TypedValue::Number(18.0)),
    Op::Gt,
    
    // Store the result as a boolean
    Op::Store("is_adult".to_string()),
    
    // Combine string and number in a message
    Op::Load("greeting".to_string()),
    Op::Push(TypedValue::String(", you will be ".to_string())),
    Op::Add,
    Op::Load("next_year_age".to_string()),
    Op::Add, // Auto-converts number to string
    Op::Push(TypedValue::String(" next year.".to_string())),
    Op::Add,
    
    // Store the final message
    Op::Store("message".to_string()),
];

vm.execute(&ops)?;

// Output: "Hello, Alice, you will be 31 next year."
println!("{}", vm.load("message")?.as_string()?);
``` 