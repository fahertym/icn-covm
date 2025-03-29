# Fibonacci sequence benchmark
# Calculates the nth Fibonacci number using recursion
# This is an intentionally expensive calculation to test VM performance

def fibonacci(n):
    # Base cases
    load n
    push 0
    eq
    if:
        push 0
        return
        
    load n
    push 1
    eq
    if:
        push 1
        return
    
    # Recursive case: fib(n) = fib(n-1) + fib(n-2)
    load n
    push 1
    sub
    call fibonacci
    
    load n
    push 2
    sub
    call fibonacci
    
    add
    return

# Calculate the 20th Fibonacci number
push 20
call fibonacci

# Assert the result is correct
# The 20th Fibonacci number is 6765
push 6765
eq 