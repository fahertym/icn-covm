# Define some utility functions
def add(x, y):
    load x
    load y
    add
    return

def multiply_and_print(x, y):
    load x
    emit "First number:"
    load y
    emit "Second number:"
    load x
    load y
    mul
    emit "Product:"
    return

def countdown(n):
    load n
    push 0
    lt
    if:
        push 0
        return
    else:
        load n
        emit "Current value:"
        load n
        push 1
        sub
        store n
        load n
        push 0
        gt
        if:
            load n
            call countdown
        else:
            push 0
        return

# Fibonacci sequence implementation
def fib(n):
    load n
    push 1
    lt
    if:
        load n
        return
    else:
        load n
        push 1
        sub
        store n
        load n
        call fib
        load n
        push 2
        sub
        store n
        load n
        call fib
        add
        return

# Factorial implementation
def factorial(n):
    load n
    push 1
    lt
    if:
        push 1
        return
    else:
        load n
        push 1
        sub
        store n
        load n
        call factorial
        load n
        mul
        return

# GCD implementation using Euclidean algorithm
def gcd(a, b):
    load b
    push 0
    eq
    if:
        load a
        return
    else:
        load a
        load b
        mod
        load b
        store a
        store b
        call gcd
        return

# Main program
emit "Defining functions..."

emit "Testing add with 20 and 22:"
push 20
push 22
call add

emit "Testing multiply_and_print with 6 and 7:"
push 6
push 7
call multiply_and_print

emit "Starting countdown from 5:"
push 5
call countdown

emit "Calculating Fibonacci numbers..."
push 10
call fib
emit "Fibonacci(10) ="

emit "Calculating factorial..."
push 5
call factorial
emit "Factorial(5) ="

emit "Calculating GCD..."
push 48
push 18
call gcd
emit "GCD(48, 18) ="

emit "Stack manipulation demo:"
push 1
push 2
push 3
dup
swap
over
dumpstack

emit "Memory operations demo:"
push 42
store x
push 24
store y
load x
load y
add
emit "x + y ="
dumpmemory 