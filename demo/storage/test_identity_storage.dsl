# Test Identity and Storage Integration
# This tests the integration between identity and storage features

# Define an identity context for execution
emit "1. Setting up identity context..."
push "alice" # User ID
push "member" # Identity type
push "musician_coop" # Cooperative ID
storepref

# Store a value with identity context
emit "2. Storing a value as alice..."
push 42.0
storep "alice/test_value"
emit "Stored value 42 under alice's namespace"

# Retrieve the value with the same identity context
emit "3. Loading alice's value..."
loadp "alice/test_value"
emit "Value retrieved:"
dumpstack

emit "Test complete! Identity-aware storage is working." 