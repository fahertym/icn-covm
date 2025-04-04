# Basic Storage Test
# This tests the very basics of storage backends

# Store a simple value
emit "1. Storing a value..."
push 123.0
storep test_key
emit "Stored value: 123"

# Retrieve the value
emit "2. Loading the value..."
loadp test_key
emit "Value loaded:"
dumpstack

emit "Test complete! Value should be 123" 