# Parser Demo: Testing improved modular parser capabilities
# This demonstrates nested blocks, indentation, and flow control

# Basic operations
push 10
push 20
add
emit "10 + 20 = "

# Variable manipulation
push 42
store answer
load answer
emit "The answer is:"

# Simple conditional - we need to initialize x first
push 0
store x

push 1  # true value
if:
    emit "Condition is true"
    push 100
    store x
    
load x
emit "Value of x:"

# Nested conditionals
push 5
push 8
gt  # 5 > 8 ?
if:
    emit "This won't execute (5 is not > 8)"
else:
    emit "5 is not greater than 8"
    
    push 3
    push 3
    eq  # 3 == 3 ?
    if:
        emit "Nested: 3 equals 3"

# Loop demo
push 5
store counter

while:
    condition:
        load counter
        push 0
        gt  # counter > 0
    
    emit "Counter value:"
    load counter
    
    # Nested if inside while loop
    load counter
    push 3
    eq
    if:
        emit "Counter is exactly 3!"
    
    # Decrement counter
    load counter
    push 1
    sub
    store counter

# Match statement
push 2
match:
    value:
        push 2  # Value to match against
    case 1:
        emit "Value is 1"
    case 2:
        emit "Value is 2"
    case 3:
        emit "Value is 3"
    default:
        emit "Unknown value"

# Loop with break - initialize i first
push 10
store i

while:
    condition:
        load i
        push 0
        gt  # i > 0
    
    load i
    push 5
    eq  # i == 5
    if:
        emit "Breaking at i=5"
        break
    
    emit "Loop iteration:"
    load i
    
    load i
    push 1
    sub
    store i

# Use the debug/introspection opcode
dumpstate 