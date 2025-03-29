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
    else:
        load n
        push 1
        sub
        store n_minus_1
        load n_minus_1
        call factorial
        load n
        mul
        return

# Define a function with complex nested blocks
def process_value(x):
    # Store the parameter in memory
    store x
    
    # Now load it for use
    load x
    
    # Nested match statement
    match:
        value:
            load x
        case 0:
            emit "Value is zero"
            push 0
        case 1:
            emit "Value is one"
            push 1
        case 2:
            emit "Value is two, using factorial"
            push 5
            store fact_input
            load fact_input
            call factorial
        default:
            emit "Unknown value, using while loop"
            load x
            # Nested while loop in the default case
            while:
                condition:
                    dup
                    push 0
                    gt
                push 1
                sub
            emit "Countdown complete"
    
    # Another level of nesting with an if/else condition
    dup  # Duplicate the result from the match block
    push 10
    gt
    if:
        emit "Result is greater than 10"
        # Nested loop within the if block
        loop 3:
            emit "Processing in loop"
            push 1
            add
    else:
        emit "Result is not greater than 10"
        # Nested if within the else block
        dup
        push 5
        gt
        if:
            emit "But it's greater than 5"
        else:
            emit "And it's less than or equal to 5"
    
    # Using standard library functions
    dup
    store abs_input
    load abs_input
    call abs
    emit "Absolute value calculated"
    
    return

# Main program
push 5
call process_value

# Final state inspection
dumpstate 