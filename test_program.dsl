# Sample program to demonstrate VM features
# This will use parameters and emit events

# Load parameters from command line
load count

# Print the initial count
emitevent "program" "Starting countdown from count"

# Create a countdown loop
while:
    load count
    dup
    push 0
    gt

    # Emit the current count
    dup
    emitevent "countdown" "Current value: "
    emit
    
    # Decrement count
    push 1
    sub
    
    # Store updated count
    store count
    
# Final message
emitevent "program" "Countdown complete!"

# Check if threshold parameter exists
load threshold
dup
push 0
eq
if:
    pop
    emitevent "params" "No threshold parameter provided"
else:
    emitevent "params" "Threshold parameter found: "
    emit

# Clean up stack
push 0 