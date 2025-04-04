# Persistent Counter Demo
# This demonstrates using the StoreP and LoadP operations
# to maintain a persistent counter that survives within a program run.
# Note: The current InMemoryStorage implementation doesn't persist data between program runs,
# so each time this script is executed, it starts from scratch.
# A real persistent storage implementation would save to disk.

# Attempt to load the counter value from persistent storage
emit "Attempting to load counter:"
loadp counter
dumpstack
# Check if counter exists (might not exist on first run)
push 0.0
eq
if:
    # First run: initialize counter to 0
    emit "Counter doesn't exist yet, initializing to 0"
    push 0.0
    storep counter
    # Output message
    emit "Initialized counter to 0"
    # Load the counter for following operations
    loadp counter
    dumpstack
else:
    # Counter already exists, show current value
    emit "Loaded counter value:"
    dumpstack

# Show counter value
dup
emit "Current counter value:"
dumpstack

# Increment the counter
push 1.0
add
emit "New counter value:"
dumpstack

# Store the incremented value back to persistent storage
storep counter

# Final message
emit "Counter updated successfully."
emit "Note: With InMemoryStorage, the counter resets when the program exits."
emit "To maintain persistence between runs, use the file backend:"
emit "cargo run -- run --program demo/storage/persistent_counter.dsl --storage-backend file --storage-path ./filestorage" 