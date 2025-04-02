# Persistent Counter Demo
# This demonstrates using the StoreP and LoadP operations
# to maintain a persistent counter that survives between program runs.

# Attempt to load the counter value from persistent storage
# If it doesn't exist (first run), use a default value
loadp counter
# Check if anything is on the stack (if not, this is the first run)
dup 0.0 eq 
if
  # First run: initialize counter to 0
  pop
  push 0.0
  storep counter
  # Output message
  emit "Initialized counter to 0"
else
  # Counter already exists, show current value
  emit "Loaded counter value:"
endif

# Show counter value
dup
emit "Current counter value:"

# Increment the counter
push 1.0
add
emit "New counter value:"

# Store the incremented value back to persistent storage
storep counter

# Final message
emit "Counter updated successfully. Run this program again to continue incrementing." 