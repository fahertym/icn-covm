# Ranked Choice Voting Demonstration
# This demonstrates using the RankedVote operation for cooperative governance

# Emitting some information about the demo
emit "Ranked Choice Voting Demo"
emit "============================"
emit "This demo demonstrates instant-runoff voting with 3 candidates and 5 voters."
emit "Each voter ranks the candidates in order of preference."
emit ""

# Setup: 3 candidates labeled as 0, 1, and 2
# We'll have 5 ballots (votes) with different preferences

# First, let's explain the voting scenario
emit "Election Scenario:"
emit "Candidate 0: Cooperative Sustainability Initiative"
emit "Candidate 1: Community Resource Allocation Plan"
emit "Candidate 2: Solidarity Economy Framework"
emit ""

emit "Voter preferences (in order of preference):"
emit "Voter 1: [1, 0, 2] - Prefers candidate 1, then 0, then 2"
emit "Voter 2: [1, 0, 2] - Prefers candidate 1, then 0, then 2"
emit "Voter 3: [0, 1, 2] - Prefers candidate 0, then 1, then 2"
emit "Voter 4: [0, 2, 1] - Prefers candidate 0, then 2, then 1"
emit "Voter 5: [2, 1, 0] - Prefers candidate 2, then 1, then 0"
emit ""

# Ballot 1: Preferences [1, 0, 2]
push 2.0  # Third choice
push 0.0  # Second choice
push 1.0  # First choice

# Ballot 2: Preferences [1, 0, 2]
push 2.0  # Third choice
push 0.0  # Second choice
push 1.0  # First choice

# Ballot 3: Preferences [0, 1, 2]
push 2.0  # Third choice
push 1.0  # Second choice
push 0.0  # First choice

# Ballot 4: Preferences [0, 2, 1]
push 1.0  # Third choice
push 2.0  # Second choice
push 0.0  # First choice

# Ballot 5: Preferences [2, 1, 0]
push 0.0  # Third choice
push 1.0  # Second choice
push 2.0  # First choice

# Display the stack (ballots) before voting
emit "Ballots prepared and ready for voting..."
dumpstack

# Run the ranked vote operation with 3 candidates and 5 ballots
emit "Running instant-runoff voting..."
rankedvote 3 5

# Store the result
store "winner"

# Announce the winner based on instant-runoff calculation
emit "Voting complete!"
emit "The winning candidate is:"

# Display the winner
load "winner"
push 0.0
eq
if:
    emit "Candidate 0: Cooperative Sustainability Initiative"

load "winner"
push 1.0
eq
if:
    emit "Candidate 1: Community Resource Allocation Plan"

load "winner"
push 2.0
eq
if:
    emit "Candidate 2: Solidarity Economy Framework"

emit ""
emit "This demonstrates how ranked-choice voting can be used"
emit "for democratic decision-making in cooperatives." 