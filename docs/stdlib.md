# icn-covm Standard Library

The icn-covm standard library provides common utility functions that can be used in your DSL programs. To use the standard library, run your program with the `--stdlib` flag.

## Usage

```bash
cargo run -- -p your_program.dsl --stdlib
```

## Available Functions

### Math Functions

#### `abs(x)`
Returns the absolute value of a number.

**Parameters:**
- `x`: The input number

**Returns:**
- The absolute value of x

**Example:**
```
push -5
store x
load x
call abs
# Stack now contains 5
```

#### `max(a, b)`
Returns the larger of two numbers.

**Parameters:**
- `a`: First number
- `b`: Second number

**Returns:**
- The larger value of a and b

**Example:**
```
push 10
store a
push 20
store b
load a
load b
call max
# Stack now contains 20
```

#### `min(a, b)`
Returns the smaller of two numbers.

**Parameters:**
- `a`: First number
- `b`: Second number

**Returns:**
- The smaller value of a and b

**Example:**
```
push 15
store a
push 7
store b
load a
load b
call min
# Stack now contains 7
```

### Stack Manipulation Functions

#### `swap3(a, b, c)`
Takes three values and returns them in reverse order.

**Parameters:**
- `a`: First value
- `b`: Second value
- `c`: Third value

**Returns:**
- The values in reverse order (c, b, a)

**Example:**
```
push 1
store a
push 2
store b
push 3
store c
load a
load b
load c
call swap3
# Stack now contains 3, 2, 1
```

#### `dup2()`
Duplicates the top two stack items.

**Example:**
```
push 1
push 2
call dup2
# Stack now contains 1, 2, 2, 1
```

#### `sum_n(n)`
Calculates the sum of numbers from 1 to n.

**Parameters:**
- `n`: The upper limit

**Returns:**
- The sum of numbers from 1 to n

**Example:**
```
push 10
store n
load n
call sum_n
# Stack now contains 55
```

### Boolean Logic Functions

#### `xor(a, b)`
Performs exclusive OR operation on two boolean values.

**Parameters:**
- `a`: First boolean (0 = true, any other value = false)
- `b`: Second boolean (0 = true, any other value = false)

**Returns:**
- 0 (true) if exactly one input is true, otherwise a non-zero value (false)

**Example:**
```
push 0  # true
store a
push 1  # false
store b
load a
load b
call xor
# Stack now contains 0 (true)
```

## Adding Custom Functions

To add your own functions to the standard library, modify the `stdlib.rs` file in the `src/compiler` directory. 