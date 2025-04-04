# Suggested Follow-up Tests

## 1. Memory Leakage Test (`test_return_scope.dsl`)

```
# Test to ensure memory cleanup after function returns

def create_local_vars():
    # Store some values in function-local memory
    push 42
    store temp_var1
    
    push 99
    store temp_var2
    
    # Return a value
    push 123
    return

# Main program
call create_local_vars
emit "Function returned:"
dumpstack

# These should fail if memory scoping works correctly
emit "Attempting to access function-local variables:"
load temp_var1  # Should cause VariableNotFound error
load temp_var2  # Should cause VariableNotFound error
```

**Purpose**: Verify that local variables created inside a function are not accessible after the function returns.

## 2. Recursive Function Test (`test_recursion.dsl`)

```
# Test for proper recursion handling

# Recursive factorial function
def factorial(n):
    # Base case
    load n
    push 1
    eq
    if:
        push 1
        return
    end
    
    # Recursive case
    load n
    push 1
    sub
    
    # Recursive call with n-1
    call factorial
    
    # Multiply by n
    load n
    mul
    return

# Main program
push 5  # Calculate 5!
call factorial
emit "Factorial of 5 should be 120:"
dumpstack
```

**Purpose**: Test memory isolation and stack management during recursive function calls.

## 3. Function Parameter Shadowing Test (`test_param_shadowing.dsl`)

```
# Test for parameter shadowing behavior

# Global variable
push 100
store x

def shadow_test(x):
    # This should load the parameter x, not the global x
    load x
    emit "Parameter x value:"
    dumpstack
    
    # Load the global x
    # This is challenging with current VM design
    # and demonstrates a limitation
    
    # Return the parameter
    load x
    return

# Main program
push 42  # Different value from global x
call shadow_test

emit "Global x should remain 100:"
load x
dumpstack
```

**Purpose**: Verify that function parameters properly shadow global variables with the same name.

## 4. Early Return Test (`test_early_return.dsl`)

```
# Test for early return behavior

def conditional_return(x):
    load x
    push 10
    gt
    if:
        emit "Early return path (x > 10)"
        push 1
        return
    end
    
    emit "Normal path execution continues"
    push 0
    return

# Test with value that causes early return
push 15
call conditional_return
emit "Should be 1 (early return path):"
dumpstack

# Test with value that doesn't cause early return
push 5
call conditional_return
emit "Should be 0 (normal path):"
dumpstack
```

**Purpose**: Verify that early returns from different control flow paths work correctly.

## 5. Complex Parameter Test (`test_complex_params.dsl`)

```
# Test with a large number of parameters

def many_params(a, b, c, d, e, f):
    # Sum all parameters
    load a
    load b
    add
    load c
    add
    load d
    add
    load e
    add
    load f
    add
    return

# Main program
push 1  # a
push 2  # b
push 3  # c
push 4  # d
push 5  # e
push 6  # f
call many_params
emit "Sum of 1+2+3+4+5+6 should be 21:"
dumpstack
```

**Purpose**: Test the VM's ability to handle functions with many parameters.

## Implementation Suggestions

1. **Recursion Limit**: Consider adding a maximum recursion depth check to prevent stack overflow.

2. **Parameter Handling Enhancement**: Consider storing parameters in a separate structure from the main memory to better handle shadowing.

3. **Function Tracing**: Add a debug mode that traces function calls and returns with memory snapshots.

4. **Return Value Documentation**: Clarify in documentation that functions return by leaving the value on the stack.

5. **Memory Optimization**: Consider implementing copy-on-write semantics for memory contexts to optimize performance in deeply nested function calls. 