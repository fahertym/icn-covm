# Loop performance benchmark
# Performs a simple loop with counter increments

# Initialize counter
push 0
store counter

# Perform 1000 iterations
loop 1000:
    # Increment counter
    load counter
    push 1
    add
    store counter

# Check final result
load counter
push 1000
eq 