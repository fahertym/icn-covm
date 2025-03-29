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
    load c
    dup
    over
    lt
    if:
        # If c < max(a,b), keep max(a,b)
        pop
    else:
        # If max(a,b) <= c, keep c
        swap
        pop
    
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