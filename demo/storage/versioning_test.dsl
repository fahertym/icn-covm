# Storage Versioning Test
# This demonstrates how the FileStorage backend maintains version history

# Store initial value (100)
emit "1. Storing initial value (v1 = 100)..."
push 100.0
storep product_key
emit "Initial value stored: 100"

# Update to version 2 (200)
emit "2. Updating to value (v2 = 200)..."
loadp product_key  # Load current value to verify
emit "Current value before update:"
dumpstack
push 200.0
storep product_key
emit "Updated to 200"

# Update to version 3 (300)  
emit "3. Updating to value (v3 = 300)..."
push 300.0
storep product_key
emit "Updated to 300"

# Update to version 4 (400)
emit "4. Updating to value (v4 = 400)..."
push 400.0
storep product_key
emit "Updated to 400"

# Load current value (should be 400)
emit "5. Loading the current value..."
loadp product_key
emit "Current value (should be 400):"
dumpstack

# Load version 1 (the initial value - 100)
emit "6. Loading version 1..."
loadversionp product_key 1
emit "Version 1 value (should be 100):"
dumpstack

# Load version 2 (200)
emit "7. Loading version 2..."
loadversionp product_key 2
emit "Version 2 value (should be 200):"
dumpstack

# Load version 3 (300)
emit "8. Loading version 3..."
loadversionp product_key 3
emit "Version 3 value (should be 300):"
dumpstack

# Load version 4 (400)
emit "9. Loading version 4..."
loadversionp product_key 4
emit "Version 4 value (should be 400):"
dumpstack

emit "Versioning test completed successfully" 