# icn-covm Typed Value System

This document describes the typed value system in icn-covm, an optional feature that extends the VM with a richer type system beyond simple floating-point numbers.

## Overview

By default, icn-covm operates with a simple stack of 64-bit floating-point numbers (`f64`). The typed value system extends this to support multiple data types, including:

- Numbers (f64)
- Booleans (true/false)
- Strings (text)
- Null (absence of a value)

This extension provides several benefits:

- More natural representation of different kinds of data
- Type-specific operations (e.g., string concatenation)
- Clearer semantics for logical operations
- Improved error messages for type-related issues

## Enabling the Typed Value System

The typed value system is available behind a feature flag to maintain backward compatibility. To enable it, compile with the `typed-values` feature:

```bash
cargo build --features typed-values
```

When running icn-covm with the typed value system enabled, the VM automatically uses the typed execution model.

## Type Semantics

### Type Representation

| Type     | Internal Representation | Example Literal |
|----------|-------------------------|----------------|
| Number   | f64                     | `42.0`         |
| Boolean  | bool                    | `true`         |
| String   | String                  | `"Hello"`      |
| Null     | Unit                    | `null`         |

### Type Coercion

The typed value system includes rules for coercing values between types when needed:

| From     | To Number | To Boolean | To String      |
|----------|-----------|------------|----------------|
| Number   | (same)    | `0` → false, others → true | String representation |
| Boolean  | `true` → `1.0`, `false` → `0.0` | (same) | `"true"` or `"false"` |
| String   | Parse if numeric, error otherwise | Empty → false, others → true | (same) |
| Null     | `0.0`     | `false`    | `"null"`       |

These coercion rules are applied automatically when operations require a specific type.

## Operations

### Arithmetic Operations

| Operation | Types                        | Behavior                                       |
|-----------|------------------------------|------------------------------------------------|
| Add       | Number + Number              | Numeric addition                               |
|           | String + String              | String concatenation                           |
|           | String + Any                 | Convert second operand to string and concatenate |
|           | Any + String                 | Convert first operand to string and concatenate |
|           | Other combinations           | Convert to numbers and add                     |
| Sub       | Any + Any                    | Convert to numbers and subtract                |
| Mul       | Number + Number              | Numeric multiplication                         |
|           | String + Number              | Repeat string (e.g., "a" * 3 → "aaa")         |
|           | Number + String              | Repeat string                                  |
|           | Other combinations           | Convert to numbers and multiply                |
| Div       | Any + Any                    | Convert to numbers and divide                  |
| Mod       | Any + Any                    | Convert to numbers and compute modulo          |

### Logical Operations

| Operation | Types                        | Behavior                                       |
|-----------|------------------------------|------------------------------------------------|
| Eq        | Number = Number              | Numeric equality                               |
|           | Boolean = Boolean            | Boolean equality                               |
|           | String = String              | String equality                                |
|           | Null = Null                  | Always true                                    |
|           | Null = Any                   | Always false                                   |
|           | Other combinations           | Convert to strings and compare                 |
| Gt        | Number > Number              | Numeric comparison                             |
|           | String > String              | Lexicographic comparison                       |
|           | Other combinations           | Convert to numbers and compare                 |
| Lt        | Number < Number              | Numeric comparison                             |
|           | String < String              | Lexicographic comparison                       |
|           | Other combinations           | Convert to numbers and compare                 |
| Not       | Any                          | Convert to boolean and negate                  |
| And       | Any and Any                  | Convert both to booleans and apply logical AND |
| Or        | Any or Any                   | Convert both to booleans and apply logical OR  |

## DSL Syntax Extensions

The typed value system extends the DSL syntax to support literals of different types:

```
# Number literal (unchanged)
push 42

# Boolean literals (new)
push true
push false

# String literals (new)
push "Hello, world!"

# Null literal (new)
push null
```

## Error Handling

The typed value system introduces new error variants related to type operations:

- **TypeMismatch**: When an operation receives an unexpected type
- **InvalidOperationForType**: When an operation isn't valid for the given types
- **CoercionError**: When a value can't be converted to the required type
- **ValueOutOfBounds**: When a value is outside acceptable bounds (e.g., string repetition)

These errors provide more detailed diagnostics about type-related issues.

## Implementation Details

The typed value system is implemented in the `typed.rs` module and includes:

1. `TypedValue` enum representing the different value types
2. `TypedVM` struct, an extended VM that operates on typed values
3. Type-specific operation implementations
4. Coercion functions for converting between types

## Examples

### String Manipulation

```
# String concatenation
push "Hello, "
push "World!"
add
emit  # Outputs: Hello, World!

# String repetition
push "Na"
push 4
mul
push " Batman!"
add
emit  # Outputs: NaNaNaNa Batman!
```

### Boolean Logic

```
# Boolean operations
push true
push false
or
if:
    emit "Expression is true"
else:
    emit "Expression is false"
# Outputs: Expression is true

# Comparison with mixed types
push 10
push "10"
eq
if:
    emit "10 equals \"10\""
# Outputs: 10 equals "10"
```

### Type Coercion

```
# Number to string
push 42
push " is the answer"
add
emit  # Outputs: 42 is the answer

# Boolean to number
push true
push 5
mul
emit  # Outputs: 5 (true is coerced to 1.0)
```

## Performance Considerations

The typed value system adds some overhead compared to the basic numeric VM:

- Each value requires more memory to store its type tag
- Operations need to check types and potentially perform coercions
- More complex error handling is needed

However, for many applications, the benefits of clearer semantics and more expressive types outweigh the performance cost.

## Future Directions

Potential future enhancements to the typed value system include:

- **Arrays/Lists**: Support for collections of values
- **Objects/Maps**: Support for key-value collections
- **Custom Types**: User-defined types and type checking
- **Type Annotations**: Optional type declarations and checking
- **Type Optimizations**: Specialized code paths for known types 