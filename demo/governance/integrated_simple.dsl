# Simple integrated governance demo
emit "COOPERATIVE GOVERNANCE DEMO"

# Set up initial voting power
push 1.0
store "alice_power"
push 1.0
store "bob_power"
push 1.0
store "carol_power"

# Delegate votes
liquiddelegate "alice" "bob"
emit "Alice delegates to Bob"

# Calculate voting power
push 2.0  # Bob's power (own + Alice's)
push 1.0  # Carol's power
add       # Total support: 3.0

# Check threshold
push 3.0
votethreshold 3.0

# Execute based on threshold
if:
    emit "Threshold met - executing proposal"
else:
    emit "Threshold not met - rejecting proposal" 