# TypedValue Integration Guide

## Overview

The TypedValue system extends the ICN-COVM to work with multiple data types beyond simple numeric values. This allows for more expressive and type-safe DSL programs.

## Supported Types

The TypedValue system supports the following types:

- **Numbers**: `TypedValue::Number(f64)` - Numeric values (e.g., `42.0`)
- **Strings**: `TypedValue::String(String)` - Text values (e.g., `"hello"`)
- **Booleans**: `TypedValue::Boolean(bool)` - Logical values (`true` or `false`)
- **Null**: `TypedValue::Null` - Represents absence of a value

## DSL Syntax for TypedValues

In DSL programs, you can specify different types directly:

```
# Numbers (unchanged)
push 42.0
push -3.14

# Strings (enclosed in quotes)
push "hello"
push "this is a string"

# Booleans
push true
push false

# Null
push null
```

## Type Safety and Coercion

Operations will attempt to maintain type safety while providing convenient coercion where appropriate:

1. Arithmetic operations (`add`, `sub`, `mul`, `div`, `mod`):
   - Work directly on numbers
   - `add` also works for string concatenation
   - Mixed types will be coerced if possible

2. Comparison operations (`eq`, `gt`, `lt`):
   - Compare numbers directly
   - Compare strings lexicographically
   - Compare booleans
   - Mixed types will be coerced if possible

3. Logical operations (`not`, `and`, `or`):
   - Convert values to boolean first
   - Numbers: `0` is false, everything else is true
   - Strings: Empty string is false, everything else is true
   - Null is always false

## Examples

### String Manipulation

```
# Concatenate strings
push "Hello, "
push "World!"
add
emit  # Outputs: Hello, World!

# String + number concatenation
push "Count: "
push 42
add
emit  # Outputs: Count: 42
```

### Boolean Logic

```
# Boolean operations
push true
push false
or
emit  # Outputs: true

# Truthiness of values
push "non-empty string"
if:
    emit "Strings are truthy"
    
push 0
if:
    emit "Never reached"
else:
    emit "Zero is falsy"
```

### Type-Specific Operations

```
# String comparison (lexicographical)
push "apple"
push "banana"
lt
emit  # Outputs: true

# Numeric operations
push 10
push 20
mul
emit  # Outputs: 200
```

## Type Errors

If an operation can't be performed due to incompatible types, a `TypeMismatch` error will be raised.

Example error scenario:
```
push "hello"
push true
div  # Error: Can't divide a string by a boolean
```

## Advanced Type Usage

### Memory with Mixed Types

```
# Store different types in memory
push 42
store number_val

push "hello"
store string_val

push true
store bool_val

# Later retrieve by name
load string_val
load number_val
add
emit  # Outputs: hello42
```

### Control Flow with Booleans

```
# Direct boolean control flow
push true
if:
    emit "This will execute"
else:
    emit "This won't execute"
``` 