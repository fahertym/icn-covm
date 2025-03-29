/// Get the standard library DSL code
pub fn get_stdlib_code() -> String {
    r#"
# Standard library functions for nano-cvm DSL
# These functions are automatically included in every program

# Math functions
def abs(x):
    load x
    dup
    push 0
    lt
    if:
        negate
    return

def max(a, b):
    load a
    load b
    dup
    over
    lt
    if:
        swap
        pop
    else:
        pop
    return

def min(a, b):
    load a
    load b
    dup
    over
    gt
    if:
        swap
        pop
    else:
        pop
    return

# Stack manipulation utilities
def swap3(a, b, c):
    # Takes three values and returns them in reverse order (c, b, a)
    load c
    load b
    load a
    return

def dup2():
    # Duplicates the top two stack items
    over
    over
    return

def sum_n(n):
    # Sum numbers 1 to n
    push 0  # accumulator
    push 1  # counter
    while:
        condition:
            dup         # counter
            load n
            gt
            not
        dup             # counter
        over            # accumulator
        add             # acc + counter
        swap
        pop
        swap
        push 1
        add             # counter + 1
    pop                 # remove counter
    return

# Boolean logic utilities
def xor(a, b):
    # Exclusive or: true if exactly one input is true
    load a
    load b
    eq
    not
    load a
    load b
    or
    and
    return
"#.to_string()
} 