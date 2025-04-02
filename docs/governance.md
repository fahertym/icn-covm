# Governance Operations

This document describes the governance primitives available in the ICN Cooperative Virtual Machine (ICN-COVM). These operations provide building blocks for implementing democratic decision-making processes within cooperatives.

## Table of Contents

1. [RankedVote](#rankedvote)
2. [LiquidDelegate](#liquiddelegate)

## RankedVote

The `RankedVote` operation implements instant-runoff voting (also known as ranked-choice voting) for democratic elections with multiple candidates.

### Signature

```
rankedvote <candidates> <ballots>
```

- `candidates`: Number of candidates in the election (minimum 2)
- `ballots`: Number of ballots to process (minimum 1)

### Description

Ranked-choice voting allows voters to rank candidates in order of preference. The algorithm works as follows:

1. First-choice votes are counted for each candidate
2. If a candidate has a majority (>50%), they win
3. Otherwise, the candidate with the fewest votes is eliminated
4. Votes for the eliminated candidate are redistributed to the voters' next choices
5. This process repeats until a candidate has a majority

The `RankedVote` operation expects ballots to be pushed onto the stack before it is called. Each ballot consists of `candidates` number of values representing the voter's ranked preferences.

### Stack Behavior

**Before operation**:
```
[ballot1_pref1, ballot1_pref2, ..., ballot1_prefN, ballot2_pref1, ..., ballotM_prefN]
```

**After operation**:
```
[winner]
```

Where `winner` is the candidate ID of the winning candidate (0-indexed).

### Example

```
# Push 3 ballots for an election with 3 candidates
# Each line is one ballot with ordered preferences (first-choice last)
push 2.0 push 1.0 push 0.0  # Ballot 1: Prefers candidate 0, then 1, then 2
push 2.0 push 1.0 push 0.0  # Ballot 2: Prefers candidate 0, then 1, then 2
push 0.0 push 1.0 push 2.0  # Ballot 3: Prefers candidate 2, then 1, then 0

# Run the ranked vote with 3 candidates and 3 ballots
rankedvote 3 3

# The winning candidate ID is now on top of the stack
store "winner"
```

### Error Handling

The operation will fail with an error if:
- There are fewer than 2 candidates
- There are fewer than 1 ballots
- There aren't enough values on the stack for all ballots

### Real-world Applications

- Board member elections in cooperatives
- Policy proposal selection where multiple options exist
- Budget allocation decisions among competing priorities

## LiquidDelegate

The `LiquidDelegate` operation implements liquid democracy by allowing members to delegate their voting power to others.

### Signature

```
liquiddelegate "from" "to"
```

- `from`: The member delegating their voting power
- `to`: The member receiving the delegation (or empty string to revoke)

### Description

Liquid democracy combines direct and representative democracy by allowing members to:
1. Vote directly on issues themselves, or
2. Delegate their voting power to a trusted representative, or
3. Revoke their delegation at any time

The `LiquidDelegate` operation creates a delegation relationship between two members. If a member has already delegated their voting power, they must revoke that delegation before delegating to someone else.

### Stack Behavior

This operation doesn't affect the stack directly.

### Example

```
# Alice delegates her voting power to Bob
liquiddelegate "alice" "bob"

# Dave delegates to Carol
liquiddelegate "dave" "carol"

# Carol delegates to Bob (creating a delegation chain)
liquiddelegate "carol" "bob"

# Alice revokes her delegation
liquiddelegate "alice" ""
```

### Error Handling

The operation will fail with an error if:
- The `from` parameter is empty
- The delegation would create a cycle (e.g., A→B→C→A)
- A member tries to delegate to themselves

### Real-world Applications

- Cooperative decision-making where not all members can participate directly
- Expert delegation for specialized decisions
- Inclusive governance that accommodates varying levels of involvement
- Dynamic representation that can adapt to changing circumstances

## Combining Governance Operations

These governance primitives can be combined to create sophisticated democratic systems. For example:

1. Use `LiquidDelegate` to establish a delegation network
2. Use `RankedVote` to conduct an election, with delegates casting votes according to their delegated voting power

Additional governance operations will be added in future releases to further enhance the cooperative governance capabilities of ICN-COVM. 