# Simple test for memory operations
push 42
store test_value

# Read back the value
load test_value
emit "Value from memory:"
dumpstack

# Try storing and loading to persistent storage
push 100
emit "Storing to persistent storage:"
dumpstack
storep persistent_key

# Read back from persistent storage
loadp persistent_key
emit "Value from persistent storage:"
dumpstack

# Final memory dump
emit "Final memory state:"
dumpmemory 