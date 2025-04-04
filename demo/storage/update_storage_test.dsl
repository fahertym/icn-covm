# Storage Update Test
# This tests loading and updating values

# Load the existing value
emit "1. Loading existing value..."
loadp test_key
emit "Current value:"
dumpstack

# Update the value
emit "2. Updating the value..."
push 1.0
add
emit "New value:"
dumpstack

# Store the new value
dup
storep test_key
emit "Value updated and stored"

# Load again to verify
emit "3. Loading updated value..."
loadp test_key
emit "Updated value in storage:"
dumpstack

emit "Test complete!" 