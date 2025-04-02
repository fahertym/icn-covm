# Function parameter example
# This demonstrates how function parameters work in the VM

# Define a simple function with a parameter
def add_five(x):
    # Parameters are automatically stored in the function's memory scope
    # We just need to load them from the scope
    load x
    push 5
    add
    return

# Define a function with multiple parameters
def max_of_three(a, b, c):
    dumpstack # DEBUG: Check stack on entry
    # First compare a and b
    load a
    load b
    dup
    over
    lt
    if:
        # If b < a, keep a
        pop
    else:
        # If a <= b, keep b
        swap
        pop
    
    # Now we have the max of a and b on the stack
    # Compare with c
    load c  # Stack: [..., max(a,b), c]
    dup     # Stack: [..., max(a,b), c, c]
    over    # Stack: [..., max(a,b), c, c, max(a,b)]
    lt      # pops max(a,b), c; compares max(a,b) < c; pushes 0.0(true) or 1.0(false). Stack: [..., max(a,b), c, result]
    if:
        # result is 0.0 (true), max(a,b) < c. We want c. Stack was [..., max(a,b), c].
        swap # Stack: [..., c, max(a,b)]
        pop  # Stack: [..., c]
    else:
        # result is 1.0 (false), c <= max(a,b). We want max(a,b). Stack was [..., max(a,b), c].
        pop  # Stack: [..., max(a,b)]
    
    return

# Main program
# To call a function, we push its parameters in order BEFORE the call
push 10  # Parameter for add_five
call add_five
emit "10 + 5 = "

# Call with multiple parameters
push 7   # First parameter (a)
push 15  # Second parameter (b)
push 3   # Third parameter (c)
call max_of_three
emit "Max of 7, 15, 3 is:"

# Final state inspection
dumpstate 