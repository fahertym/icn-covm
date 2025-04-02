# Governance Primitives in ICN-COVM

ICN-COVM provides specialized operations for cooperative governance and democratic decision-making. These primitives enable cooperatives to implement various governance models directly within the virtual machine.

## RankedVote

The `RankedVote` operation implements instant-runoff voting (IRV), also known as ranked-choice voting, which allows voters to rank candidates in order of preference.

### Operation Signature

```
RankedVote {
    candidates: usize,
    ballots: usize
}
```

- `candidates`: The number of candidates in the election
- `ballots`: The number of ballots to process

### Description

The `RankedVote` operation:

1. Pops `candidates × ballots` values from the stack, representing all ballot data
2. Each ballot contains ranked preferences for each candidate (ordered from first to last choice)
3. Implements instant-runoff voting to determine a winner
4. Pushes the winner's ID (candidate number) onto the stack

### Algorithm

The instant-runoff voting algorithm implemented by `RankedVote` works as follows:

1. Count first-preference votes for each candidate
2. If a candidate has a majority (>50%), they win immediately
3. Otherwise, eliminate the candidate with the fewest first-preference votes
4. Redistribute votes from the eliminated candidate to each ballot's next preferred choice
5. Repeat until a candidate achieves a majority

### Usage in DSL

```
# Push ballots onto stack (each a series of candidate IDs in preference order)
# For 3 candidates (0, 1, 2) and 5 ballots:

# Ballot 1: Preferences [0, 1, 2]
push 2.0  # Third choice
push 1.0  # Second choice
push 0.0  # First choice

# Ballot 2: Preferences [1, 0, 2]
push 2.0  # Third choice
push 0.0  # Second choice
push 1.0  # First choice

# ... more ballots ...

# Run the ranked vote with 3 candidates and 5 ballots
rankedvote 3 5

# Store the result
store winner
```

### Stack Effects

Before:
```
[... ballot1_pref1, ballot1_pref2, ..., ballot1_prefN, ballot2_pref1, ... ballotM_prefN]
```

After:
```
[... winner_id]
```

### Example

The following example demonstrates a ranked-choice vote with 3 candidates and 5 ballots:

```
# Push 5 ballots (3 candidates each)
# Ballot 1 [0, 1, 2] - Candidate 0 is first choice
push 2.0
push 1.0
push 0.0

# Ballot 2 [0, 1, 2] - Candidate 0 is first choice
push 2.0
push 1.0
push 0.0

# Ballot 3 [0, 1, 2] - Candidate 0 is first choice
push 2.0
push 1.0
push 0.0

# Ballot 4 [1, 0, 2] - Candidate 1 is first choice
push 2.0
push 0.0
push 1.0

# Ballot 5 [2, 0, 1] - Candidate 2 is first choice
push 1.0
push 0.0
push 2.0

# Run ranked vote
rankedvote 3 5

# At this point, candidate 0 wins with 3 first-choice votes (majority)
# The result (0.0) is now on top of the stack
```

### Error Handling

The `RankedVote` operation will fail with an error if:

- There are fewer than 2 candidates
- There are fewer than 1 ballot
- The stack doesn't contain enough values to satisfy `candidates × ballots`

### Practical Applications

Ranked-choice voting is particularly valuable for cooperative governance because it:

1. Eliminates the "spoiler effect" in elections with multiple candidates
2. Ensures the winner has broader support (majority vs. plurality)
3. Allows members to express nuanced preferences
4. Reduces strategic voting and encourages honest preference expression
5. Helps build consensus by considering secondary preferences

A complete working example can be found in `demo/governance/ranked_vote.dsl`. 