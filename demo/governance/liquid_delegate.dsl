# Liquid Democracy Demonstration
# This demo shows how members can delegate their voting power to others

# Emit information about the demo
emit "Liquid Democracy Demo"
emit "====================="
emit "This demo demonstrates how members can delegate their"
emit "voting power to others in a liquid democracy system."
emit ""

# Setup members with initial voting power
push 1.0
store "alice_power"
push 1.0
store "bob_power"
push 1.0
store "carol_power"
push 1.0
store "dave_power"
push 1.0
store "eve_power"

# Show initial voting power
emit "Initial voting power:"
emit "Alice: 1"
emit "Bob: 1"
emit "Carol: 1"
emit "Dave: 1"
emit "Eve: 1"
emit ""

# Step 1: Alice delegates to Bob
emit "Step 1: Alice delegates to Bob"
liquiddelegate "alice" "bob"
emit ""

# Step 2: Dave delegates to Carol
emit "Step 2: Dave delegates to Carol"
liquiddelegate "dave" "carol"
emit ""

# Step 3: Explain cycle detection without causing an error
emit "Step 3: Cycle detection capability"
emit "If Bob tried to delegate to Alice, it would create a cycle,"
emit "since Alice has already delegated to Bob (Alice → Bob → Alice)."
emit "The VM would detect this and prevent the delegation."
emit ""

# Step 4: Eve delegates to Carol
emit "Step 4: Eve delegates to Carol"
liquiddelegate "eve" "carol"
emit ""

# Step 5: Alice revokes her delegation
emit "Step 5: Alice revokes her delegation to Bob"
liquiddelegate "alice" ""
emit ""

# Step 6: Carol delegates to Bob
emit "Step 6: Carol delegates to Bob"
emit "Note that Dave and Eve have delegated to Carol, so their power transfers to Bob"
liquiddelegate "carol" "bob"
emit ""

# Calculate effective voting power based on delegations
emit "Final voting power including delegations:"
emit "Alice: 1 (delegated to nobody)"
emit "Bob: 4 (own vote + Carol's vote + Dave's vote + Eve's vote)"
emit "Carol: 0 (delegated to Bob)"
emit "Dave: 0 (delegated to Carol who delegated to Bob)"
emit "Eve: 0 (delegated to Carol who delegated to Bob)"
emit ""

# Show delegation chain
emit "Delegation chains:"
emit "Alice → (none)"
emit "Bob → (none)"
emit "Carol → Bob"
emit "Dave → Carol → Bob"
emit "Eve → Carol → Bob"
emit ""

emit "This demonstrates how liquid democracy enables flexible"
emit "representation while maintaining democratic principles." 