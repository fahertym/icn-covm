# Standard Library Demo
# This file demonstrates the use of standard library functions with our modular parser

# First, let's try the abs function
push -123
call abs
emit "Absolute value of -123 is:"

# Now let's use max to find the maximum of two numbers
push 42
push 100
call max
emit "Maximum of 42 and 100 is:"

# Let's use min to find the minimum of three numbers
push 15
push 7
call min  # Min of 15 and 7 = 7
push 10
call min  # Min of 7 and 10 = 7
emit "Minimum of 15, 7, and 10 is:"

# Let's calculate the sum of numbers from 1 to 10
push 10
call sum_n
emit "Sum of numbers 1 to 10 is:"

# Let's use our stack manipulation functions
push 1
push 2
call dup2
emit "After dup2, stack should have 1, 2, 2, 1:"

# Let's use swap3 to swap three values
push 100
push 200
push 300
call swap3  # Will return 300, 200, 100
emit "After swap3(100, 200, 300), stack has 300, 200, 100:"

# Now let's do some boolean logic with xor
push 0  # true in our VM
push 1  # false in our VM
call xor  # true xor false = true (0)
emit "XOR of true and false is (should be true/0):"

push 0  # true
push 0  # true
call xor  # true xor true = false (non-zero)
emit "XOR of true and true is (should be false/non-zero):"

# Let's combine standard library with normal control flow
push 5
store n

# Count down from n to 0, calculating absolute value of (n-10)
while:
    condition:
        load n
        push 0
        gt  # n > 0
    
    emit "Current n:"
    load n
    
    # Calculate abs(n-10)
    load n
    push 10
    sub  # n-10
    call abs
    emit "abs(n-10) ="
    
    # Decrement n
    load n
    push 1
    sub
    store n

# Show the final state
dumpstate 