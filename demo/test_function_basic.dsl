# Simple function test

# Define a function that adds two numbers
def add_two_numbers(a, b):
    load a
    load b
    add
    return

# Call the function
push 5    # Value for a
push 10   # Value for b
call add_two_numbers

# Check the result
emit "Result of 5 + 10 should be 15:"
dumpstack 