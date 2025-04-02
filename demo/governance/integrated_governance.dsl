# Integrated Governance Demo
emit "COOPERATIVE GOVERNANCE SYSTEM DEMO"
emit "==================================="
emit ""
emit "This demo demonstrates three governance primitives:"
emit "1. Liquid Democracy (delegation)"
emit "2. Ranked-Choice Voting"
emit "3. Vote Thresholds"
emit ""

# Set up initial voting power for all members
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

emit "Initial voting power:"
emit "Alice: 1.0, Bob: 1.0, Carol: 1.0, Dave: 1.0, Eve: 1.0"
emit ""

# PHASE 1: LIQUID DEMOCRACY SETUP
emit "PHASE 1: DELEGATION SETUP"
emit "========================="

# Alice delegates to Bob
liquiddelegate "alice" "bob"
emit "Alice delegates to Bob"

# Dave delegates to Carol
liquiddelegate "dave" "carol"  
emit "Dave delegates to Carol"

emit "Effective voting power after delegations:"
emit "Bob: 2.0 (own + Alice's), Carol: 2.0 (own + Dave's), Eve: 1.0"
emit ""

# PHASE 2: PROPOSAL SELECTION WITH RANKED-CHOICE VOTING
emit "PHASE 2: PROPOSAL SELECTION"
emit "==========================="
emit "Selecting between three proposals:"
emit "0: Expand operations, 1: Upgrade technology, 2: Fund outreach"
emit ""

# Bob's ballot (for 2 votes, preference order: 1, 0, 2)
# Push in reverse order (3rd choice, 2nd choice, 1st choice)
push 2.0  # Third choice
push 0.0  # Second choice
push 1.0  # First choice

# Carol's ballot (for 2 votes, preference order: 2, 1, 0)
# Push in reverse order (3rd choice, 2nd choice, 1st choice)
push 0.0  # Third choice
push 1.0  # Second choice 
push 2.0  # First choice

# Eve's ballot (for 1 vote, preference order: 1, 2, 0)
# Push in reverse order (3rd choice, 2nd choice, 1st choice)
push 0.0  # Third choice
push 2.0  # Second choice
push 1.0  # First choice

emit "Running ranked-choice voting with 3 candidates and 3 ballots..."
rankedvote 3 3
store "winning_proposal"

emit "RESULT: Proposal 0 (Expand operations) wins!"
emit ""

# PHASE 3: THRESHOLD CHECK FOR EXECUTION
emit "PHASE 3: EXECUTION THRESHOLD CHECK"
emit "=================================="
emit "Checking if threshold of 3.0 votes is met..."

# Calculate first-choice support (Bob + Eve = 3.0 votes)
push 3.0
push 3.0
votethreshold 3.0

if:
    emit "✓ Threshold met! First-choice support equals 3.0 votes."
    emit "Executing Proposal 0: Expand operations to a new location"
    emit "- Budget allocated: 45,000 credits"
    emit "- Timeline: 6 months"
    emit "- Team assigned: Bob (lead), Carol, Eve"
else:
    emit "✗ Threshold not met! Proposal cannot be executed."

emit ""
emit "GOVERNANCE FLOW COMPLETE"
emit ""
emit "This demo showed a complete democratic process:"
emit "1. Members delegated voting power (liquid democracy)"
emit "2. Ranked-choice voting selected the best option"
emit "3. Threshold check verified sufficient support" 