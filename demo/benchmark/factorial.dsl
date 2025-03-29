# Factorial benchmark
# Calculates n! using recursion
# This is another computation to benchmark VM performance

def factorial(n):
    # Base case
    load n
    push 0
    eq
    if:
        push 1
        return
    
    # Recursive case: n! = n * (n-1)!
    load n
    
    load n
    push 1
    sub
    call factorial
    
    mul
    return

# Calculate 10!
push 10
call factorial

# Assert the result is correct
# 10! = 3628800
push 3628800
eq 