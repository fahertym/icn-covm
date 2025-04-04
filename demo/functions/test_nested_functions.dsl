# Test for nested function calls
# This tests both parameter handling and memory isolation

# Inner function that returns a*b
def multiply(a, b):
    emit "In multiply function with a="
    load a
    dumpstack
    emit "and b="
    load b
    dumpstack
    
    # Return product
    load a
    load b
    mul
    emit "Multiply result (a*b):"
    dumpstack
    return

# Outer function that adds x + y + multiply result
def calc_total(x, y):
    emit "In calc_total with x="
    load x
    dumpstack
    emit "and y="
    load y
    dumpstack
    
    # Call multiply with new values
    push 100
    push 200
    call multiply
    emit "Back in calc_total with multiply result on stack:"
    dumpstack
    
    # Add x + y + multiply result
    load x  # Stack: [multiply_result, x]
    load y  # Stack: [multiply_result, x, y]
    add     # Stack: [multiply_result, x+y]
    add     # Stack: [multiply_result + x+y]
    
    emit "Final calculation in calc_total:"
    dumpstack
    return

# Main program
push 10  # x value
push 20  # y value
call calc_total

emit "Final result (should be 10 + 20 + (100 * 200) = 20030):"
dumpstack 