# Example of using standard library functions
# This program uses several functions from the standard library

# Calculate the absolute value of -5
push -5
store x
load x  # Make sure to push the value on the stack before the call
call abs
emit "Absolute value of -5 is:"

# Find the maximum of two numbers
push 10
store a
push 20
store b
load a  # Push first parameter
load b  # Push second parameter
call max
emit "Maximum of 10 and 20 is:"

# Find the minimum of two numbers
push 15
store a
push 7
store b
load a  # Push first parameter
load b  # Push second parameter
call min
emit "Minimum of 15 and 7 is:"

# Sum numbers 1 to 10
push 10
store n
load n  # Push parameter
call sum_n
emit "Sum of numbers 1 to 10 is:"

# Demonstrate XOR
push 1  # true
store a
push 0  # false
store b
load a  # Push first parameter
load b  # Push second parameter
call xor
emit "XOR of true and false is:"

# Using the debug/introspection opcode
dumpstate 