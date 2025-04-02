# Ranked-Choice Voting Tutorial

This tutorial walks through the Ranked-Choice Voting demonstration in `demo/governance/ranked_vote.dsl`. It explains how cooperatives can use the ICN-COVM to implement democratic decision-making processes.

## What is Ranked-Choice Voting?

Ranked-choice voting (also known as instant-runoff voting) is an electoral system where voters rank candidates in order of preference. Rather than selecting just one candidate, voters can express their 1st choice, 2nd choice, 3rd choice, and so on.

This system has several advantages:
- Eliminates the "spoiler effect" in elections with multiple candidates
- Ensures winners have broader support
- Allows members to express nuanced preferences
- Reduces strategic voting
- Helps build consensus

## How the Demo Works

The demonstration is structured in the following steps:

1. Set up a scenario with 3 candidates and 5 voters
2. Explain the voting scenario
3. Prepare ballots (ranked preferences for each voter)
4. Run the ranked-choice voting algorithm
5. Determine and announce the winner

Let's explore each step in detail.

### 1. Scenario Setup

```
# Ranked Choice Voting Demonstration
# This demonstrates using the RankedVote operation for cooperative governance

# Emitting some information about the demo
emit "Ranked Choice Voting Demo"
emit "============================"
emit "This demo demonstrates instant-runoff voting with 3 candidates and 5 voters."
emit "Each voter ranks the candidates in order of preference."
emit ""
```

The demo begins with an introduction explaining what we're demonstrating. This might be displayed to users in a console or GUI.

### 2. Explaining the Voting Scenario

```
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
```

Here, we're setting up a realistic scenario with:

- **3 Candidates** representing different cooperative initiatives:
  - Candidate 0: Cooperative Sustainability Initiative
  - Candidate 1: Community Resource Allocation Plan
  - Candidate 2: Solidarity Economy Framework

- **5 Voters** with different preference rankings:
  - Two voters prefer candidate 1 first
  - Two voters prefer candidate 0 first
  - One voter prefers candidate 2 first

### 3. Preparing the Ballots

```
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
```

In this section, we prepare the ballot data on the stack. Note that:

1. Each ballot is pushed onto the stack in reverse order (third choice, second choice, first choice)
2. This is because the stack is a last-in-first-out (LIFO) data structure
3. The `dumpstack` operation displays the current state of the stack for debugging

For example, for Voter 1:
- First choice: Candidate 1 (`push 1.0`)
- Second choice: Candidate 0 (`push 0.0`)
- Third choice: Candidate 2 (`push 2.0`)

All five ballots are prepared in the same way.

### 4. Running the Ranked-Choice Vote

```
# Run the ranked vote operation with 3 candidates and 5 ballots
emit "Running instant-runoff voting..."
rankedvote 3 5

# Store the result
store "winner"
```

This is where the magic happens:

1. We call `rankedvote 3 5` which:
   - Takes 3 candidates and 5 ballots as parameters
   - Pops all 15 values (3 preferences × 5 ballots) from the stack
   - Runs the instant-runoff voting algorithm internally
   - Pushes the winner's ID (0, 1, or 2) back onto the stack

2. We store the winner's ID in a variable called "winner" for later use

Behind the scenes, the ranked-vote algorithm:
- Counts first-preference votes for each candidate
- Checks if any candidate has a majority (>50%)
- If not, eliminates the candidate with the fewest first-preference votes
- Redistributes votes based on next preferences
- Repeats until a winner emerges

### 5. Announcing the Winner

```
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
```

Finally, we announce the winner:

1. Load the winner's ID from the "winner" variable
2. Compare it with each candidate ID (0, 1, and 2)
3. When we find a match, emit the name of the winning initiative
4. End with a message about democratic decision-making in cooperatives

## Expected Outcome

When you run this demo, the following will happen:

1. Based on the voter preferences:
   - Candidate 0 has 2 first-choice votes
   - Candidate 1 has 2 first-choice votes
   - Candidate 2 has 1 first-choice vote

2. Since no candidate has a majority (3 or more votes), candidate 2 is eliminated as having the fewest first-choice votes.

3. The voter who chose candidate 2 first has their vote transferred to their second choice, candidate 1.

4. The updated count becomes:
   - Candidate 0: 2 votes
   - Candidate 1: 3 votes (2 original + 1 transferred)

5. Candidate 1 now has a majority (3 out of 5 votes) and wins the election.

6. The announcement will display: "Candidate 1: Community Resource Allocation Plan"

## Extending the Demo

You can modify this demo to experiment with different voting scenarios:

1. Change the number of candidates
2. Alter voter preferences
3. Add more voters
4. Try edge cases like ties

Remember to update both the explanation text and the actual ballot data if you make changes.

## Real-World Applications

In a cooperative setting, ranked-choice voting can be used for:

1. Board elections
2. Budget allocation decisions
3. Strategic planning priorities
4. Policy adoption
5. Resource allocation

By implementing this as a VM primitive, cooperatives can conduct secure, transparent, and programmable voting processes as part of their governance operations. 