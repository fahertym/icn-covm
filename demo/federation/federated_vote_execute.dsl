# Federated Proposal Voting Execution
# This demonstrates tallying votes for a federated proposal

# Emit information about the proposal
emit "Federated Proposal Voting"
emit "========================="
emit "Proposal ID: prop-2023-07-15"
emit "Namespace: membership"
emit "Creator: coopA"
emit ""

# Display the options
emit "Options:"
emit "1. Accept 5 new members from the applicant pool"
emit "2. Create a mentorship program for new members"
emit "3. Keep membership closed until next quarter"
emit ""

# Display the votes
emit "Votes:"
emit "Alice: 3, 2, 1 (prefers option 3, then 2, then 1)"
emit "Bob: 1, 2, 3 (prefers option 1, then 2, then 3)"
emit "Carol: 2, 3, 1 (prefers option 2, then 3, then 1)"
emit ""

# Set up the ballots for voting
# Alice's ballot
push 0.0  # Third choice: option 1
push 1.0  # Second choice: option 2
push 2.0  # First choice: option 3

# Bob's ballot
push 2.0  # Third choice: option 3
push 1.0  # Second choice: option 2
push 0.0  # First choice: option 1

# Carol's ballot
push 0.0  # Third choice: option 1
push 2.0  # Second choice: option 3
push 1.0  # First choice: option 2

# Perform the ranked vote calculation
emit "Performing ranked choice voting..."
rankedvote 3 3

# Store the result
store "winner"

# Announce the winner
emit "Voting complete!"
emit "Winning option:"

# Check which option won
load "winner"
push 0.0
eq
if:
    emit "Option 1: Accept 5 new members from the applicant pool"
    
load "winner"
push 1.0
eq
if:
    emit "Option 2: Create a mentorship program for new members"
    
load "winner"
push 2.0
eq
if:
    emit "Option 3: Keep membership closed until next quarter"

# Store the winner in persistent storage
load "winner"
storep "federation/proposals/prop-2023-07-15/winner"

# Store a timestamp for when the vote was completed
push 1689447600.0  # 2023-07-15 12:00:00 UTC
storep "federation/proposals/prop-2023-07-15/executed_at"

emit ""
emit "Result has been recorded in the storage system."
emit "All federation nodes can now access the voting outcome." 