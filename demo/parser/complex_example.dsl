# Complex example with nested blocks and standard library functions
# This demonstrates the improved parsing capabilities

# Define a recursive factorial function
def factorial(n):
    load n
    push 1
    lt    # n < 1
    if:
        push 1
        return
    
    # Else calculate factorial recursively
    load n
    push 1
    sub          # n-1
    call factorial  # factorial(n-1)
    
    load n
    mul          # n * factorial(n-1)
    return

# Main program
push 5
call factorial
emit "Factorial of 5 is:"

# Demonstrate nested blocks with a countdown and conditionals
push 5
store counter

# Simple loop without else blocks
while:
    condition:
        load counter
        push 0
        gt    # counter > 0

    # Log the current value
    emit "Current counter:"
    load counter
    
    # Decrement the counter
    load counter
    push 1
    sub
    store counter

# Use nested if statements instead of if-else
push 10
push 20
push 30
store c
store b
store a

# First level if
load a
load b
gt  # a > b
if:
    emit "a is greater than b"
    
    # Second level if - nested inside first if
    load a
    load c
    gt  # a > c
    if:
        emit "a is also greater than c"
    
    emit "Done comparing a and c"

# Use the standard library
push -42
call abs
emit "Absolute value of -42 is:"

# Final state inspection
dumpstate 