# VoteThreshold Walkthrough

## Introduction

The `VoteThreshold` operation is a powerful governance primitive that enables conditional execution based on voting support. It serves as a "gatekeeper" for governance actions, only allowing them to execute when they have received sufficient backing from stakeholders.

This tutorial explains how to use the `VoteThreshold` operation and how it complements the `LiquidDelegate` and `RankedVote` primitives to form a complete governance suite.

## What is VoteThreshold?

`VoteThreshold` is a conditional operation that:

1. Takes a threshold value as a parameter (e.g., `votethreshold 3.0`)
2. Compares the top value on the stack (total voting power in favor) with the threshold
3. Pushes a truthy value (`0.0`) if the threshold is met, falsey (`1.0`) if not
4. Is typically used within an `if:` block to conditionally execute proposal actions

## When to Use VoteThreshold

The `VoteThreshold` operation is ideal for:

- **Policy Approval**: Require a minimum level of support before enacting policies
- **Fund Distribution**: Ensure sufficient backing before releasing funds
- **Constitutional Changes**: Enforce super-majority requirements for critical changes
- **Quorum Validation**: Check that enough members participated in a vote

## Basic Syntax

```dsl
# Calculate total support votes
push 5.0  # Example: 5 votes in favor

# Check against threshold
push 3.0  # Require at least 3 votes
votethreshold 3.0

# Conditional execution
if:
    emit "Proposal passed! Executing..."
    # Execution logic here
else:
    emit "Proposal failed to meet threshold."
    # Failure handling here
```

## Walkthrough of the Demo

The `vote_threshold.dsl` demo demonstrates various scenarios using the `VoteThreshold` operation. Let's walk through each step:

### 1. Setting Up Members and Voting Power

```dsl
# Initialize voting power for members
push 1.0
store "alice_power"

push 1.0
store "bob_power"

# ... (more member initializations)
```

Each member starts with 1.0 voting power. This represents one vote per member in the simplest case.

### 2. Establishing Delegation Relationships

```dsl
# Set up delegations for liquid democracy
liquiddelegate "alice" "bob"
liquiddelegate "dave" "carol"
```

Using the `LiquidDelegate` operation, we create a delegation graph:
- Alice delegates to Bob (Bob now has 2.0 effective voting power)
- Dave delegates to Carol (Carol now has 2.0 effective voting power)

### 3. Scenario 1: Proposal with Simple Majority

```dsl
# Calculate total support
push 2.0  # Bob's effective voting power
push 2.0  # Carol's effective voting power
add       # 4.0 total support

# Check if it meets the threshold
push 3.0
votethreshold 3.0
```

Here, we:
1. Calculate the total voting power in favor (4.0)
2. Check against a threshold of 3.0
3. Since 4.0 >= 3.0, the condition evaluates to true (pushing `0.0`)
4. The `if:` block executes, approving the proposal

### 4. Scenario 2: Higher Threshold Proposal

```dsl
# Calculate total support 
push 2.0  # Bob's effective voting power
push 2.0  # Carol's effective voting power
add       # 4.0 total support

# Check if it meets the higher threshold
push 4.5
votethreshold 4.5
```

This time:
1. The same total support (4.0) is calculated
2. But the threshold is higher (4.5)
3. Since 4.0 < 4.5, the condition evaluates to false (pushing `1.0`)
4. The `else:` block executes, rejecting the proposal

### 5. Scenario 3: Adding More Support

Adding Eve's vote changes the outcome:

```dsl
# Calculate total support with Eve
push 2.0  # Bob's effective voting power
push 2.0  # Carol's effective voting power
push 1.0  # Eve's voting power
add       # 4.0
add       # 5.0 total support

# Check against the same threshold
push 4.5
votethreshold 4.5
```

Now with 5.0 total support exceeding the 4.5 threshold, the proposal passes.

### 6. Scenario 4: Percentage-Based Threshold

```dsl
# Calculate total voting power
push 5.0  # Total possible voting power

# Calculate threshold (60%)
push 0.6  # 60% 
mul       # 5.0 * 0.6 = 3.0
```

This demonstrates how to calculate a percentage-based threshold:
1. Determine the total possible voting power (5.0)
2. Multiply by the desired percentage (0.6 for 60%)
3. Compare actual support against this calculated threshold

## Integration with Other Governance Primitives

The `VoteThreshold` operation is designed to work seamlessly with:

### 1. LiquidDelegate

`LiquidDelegate` determines the effective voting power of each member, accounting for delegations. This voting power is then used in threshold calculations:

```dsl
# Alice delegates to Bob
liquiddelegate "alice" "bob"

# Calculate Bob's effective voting power
# (assuming get_power is a helper function that uses VM.get_effective_voting_power)
call "get_power" "bob"  # Returns 2.0 (Bob's own 1.0 + Alice's 1.0)

# Use this power in a threshold check
push 1.5
votethreshold 1.5
```

### 2. RankedVote

`RankedVote` can determine a winner among multiple options, and then `VoteThreshold` can verify if that winning option has sufficient support:

```dsl
# Run a ranked-choice vote with 3 candidates and 5 ballots
# (ballot values already on stack)
rankedvote 3 5  # Returns the winner's ID (e.g., 1)

# Store the winner
store "winning_option"

# Check support for the winning option
load "option_1_support"  # Get support for option 1
push 3.0  # Threshold
votethreshold 3.0

if:
    emit "Option 1 wins with sufficient support!"
    # Execute the winning proposal
else:
    emit "Option 1 wins but lacks required support."
    # Handle insufficient support
```

### 3. Quorum Validation

```dsl
# Check quorum first
load "total_votes_cast"
push 5.0  # Minimum required participation
votethreshold 5.0

if:
    # Quorum met, now check support threshold
    load "votes_in_favor"
    push 3.0
    votethreshold 3.0
    
    if:
        emit "Proposal passes with quorum and support!"
    else:
        emit "Quorum met but support threshold not reached."
else:
    emit "Quorum not reached. Vote invalid."
```

## Best Practices

1. **Clear Thresholds**: Document and communicate threshold values clearly
2. **Appropriate Levels**: Set thresholds based on decision importance:
   - Critical decisions: higher thresholds (e.g., 2/3 or 3/4)
   - Routine decisions: lower thresholds (e.g., simple majority)

3. **Quorum Considerations**: Sometimes check both participation and support:
   ```dsl
   # Check quorum first
   load "total_votes_cast"
   push 5.0  # Minimum required participation
   votethreshold 5.0
   
   if:
       # Quorum met, now check support threshold
       load "votes_in_favor"
       push 3.0
       votethreshold 3.0
       
       if:
           emit "Proposal passes with quorum and support!"
       else:
           emit "Quorum met but support threshold not reached."
   else:
       emit "Quorum not reached. Vote invalid."
   ```

4. **Percentage Thresholds**: Calculate based on total possible voting power for consistency

## Extensions and Advanced Uses

- **Multiple Thresholds**: Different actions in a proposal could have different thresholds
- **Time-Based Thresholds**: Adjust thresholds based on voting duration
- **Weighted Threshold Systems**: Combine with stake weighting for economic governance
- **Graduated Execution**: Execute different parts of a proposal as different thresholds are met

## Conclusion

The `VoteThreshold` operation provides a simple but powerful way to implement governance rules in your cooperative. By combining it with other primitives like `LiquidDelegate` and `RankedVote`, you can create sophisticated governance systems that ensure:

- Democratic decision-making
- Proper representation of stakeholder interests
- Protection of minority stakeholder rights
- Appropriate barriers for significant decisions

Through these primitives, the ICN Cooperative Virtual Machine enables programmable, transparent, and configurable governance as code. 