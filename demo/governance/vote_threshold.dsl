# Vote Threshold Demonstration
# =========================
# This demo shows how to use the VoteThreshold operation to execute
# actions only when sufficient voting power supports them.

# Initialize voting power for members
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

# Emit initial state
emit "Vote Threshold Demo"
emit "==================="
emit ""
emit "Initial voting power:"
emit "Alice: 1"
emit "Bob: 1"
emit "Carol: 1"
emit "Dave: 1"
emit "Eve: 1"
emit ""
emit "Total voting power: 5"
emit ""

# Set up delegations for liquid democracy
emit "Step 1: Setting up delegations"
liquiddelegate "alice" "bob"
emit "Alice delegates to Bob"
liquiddelegate "dave" "carol"
emit "Dave delegates to Carol"
emit ""

# Display effective voting power after delegations
emit "Effective voting power after delegations:"
emit "Alice: 0 (delegated to Bob)"
emit "Bob: 2 (own + Alice's power)"
emit "Carol: 2 (own + Dave's power)"
emit "Dave: 0 (delegated to Carol)"
emit "Eve: 1 (no delegation)"
emit ""

# Scenario 1: Proposal with 3.0 threshold
emit "Scenario 1: Proposal with threshold of 3.0"
emit "Assume Bob and Carol vote in favor"
emit "Total support: 2 + 2 = 4"

# Calculate total support
push 2.0
push 2.0
add

# Check if it meets the threshold
push 3.0
votethreshold 3.0

# Conditional execution based on threshold
if:
    emit "Proposal PASSED: 4.0 votes exceeds threshold of 3.0"
    emit "Executing proposal actions..."
    emit "- Funds disbursed"
    emit "- Policy updated"
    emit "- Notification sent to stakeholders"
else:
    emit "Proposal FAILED: Did not meet threshold"
    emit "No actions executed"
emit ""

# Scenario 2: Higher threshold proposal
emit "Scenario 2: Proposal with threshold of 4.5"
emit "Assume only Bob and Carol vote in favor again"
emit "Total support: 2 + 2 = 4"

# Calculate total support 
push 2.0
push 2.0
add

# Check if it meets the higher threshold
push 4.5
votethreshold 4.5

# Conditional execution based on threshold
if:
    emit "Proposal PASSED: Votes exceed threshold of 4.5"
    emit "Executing proposal actions..."
else:
    emit "Proposal FAILED: 4.0 votes is below threshold of 4.5"
    emit "Proposal rejected"
    emit "No actions executed"
emit ""

# Scenario 3: Eve changes the outcome 
emit "Scenario 3: Eve joins the second vote"
emit "Assume Bob, Carol, and Eve vote in favor"
emit "Total support: 2 + 2 + 1 = 5"

# Calculate total support with Eve
push 2.0
push 2.0
push 1.0
add
add

# Check against the same threshold
push 4.5
votethreshold 4.5

# Conditional execution based on threshold
if:
    emit "Proposal PASSED: 5.0 votes exceeds threshold of 4.5" 
    emit "Executing proposal actions..."
    emit "This shows how a single vote can tip the balance"
    emit "when using precise thresholds."
else:
    emit "Proposal FAILED: Did not meet threshold"
    emit "No actions executed"
emit ""

# Demonstrate percentage-based threshold
emit "Scenario 4: Percentage-based threshold (60%)"
emit "Total voting power: 5"
emit "Required votes: 5 * 0.6 = 3"
emit "Assume only Bob votes in favor"
emit "Support: 2 votes (40%)"

# Calculate total voting power
push 5.0

# Calculate threshold (60%)
push 0.6
mul

# Store threshold
dup
store "threshold_60_percent"

# Display threshold 
emit "60% threshold = 3.0 votes"

# Calculate support (only Bob)
push 2.0

# Check against percentage threshold
load "threshold_60_percent"
votethreshold 3.0

# Conditional execution based on threshold
if:
    emit "Proposal PASSED: Votes exceed 60% threshold"
else:
    emit "Proposal FAILED: 2.0 votes (40%) is below 60% threshold (3.0)"
    emit "Proposal rejected"
emit ""

emit "This demo shows how VoteThreshold enables:"
emit "1. Threshold-based decision execution"
emit "2. Combination with LiquidDelegate for voting power"
emit "3. Support for both absolute and percentage thresholds"
emit "4. Conditional control flow in governance" 