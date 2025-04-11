# DSL Macros

This document describes the available macros in the DSL (Domain Specific Language) and how to use them.

## Proposal Lifecycle Macro

The `proposal_lifecycle` macro provides a standardized way to create and manage cooperative proposals in the system. It abstracts the common pattern of proposal creation, voting, and execution into a single, easy-to-use construct.

### Syntax

```dsl
proposal_lifecycle "proposal_id" [quorum=X] [threshold=Y] {
    # Execution block
    # Commands to execute when proposal passes
}
```

### Parameters

- `proposal_id` (required): A unique identifier for the proposal
- `quorum` (optional): The minimum percentage of eligible voters that must participate (default: 0.6 or 60%)
- `threshold` (optional): The minimum percentage of "yes" votes required for the proposal to pass (default: 0.5 or 50%)

### Execution Block

The execution block contains the commands that will be executed if the proposal passes. These can include any valid DSL commands, such as:
- `emit` for logging
- `mint` for creating new tokens
- `transfer` for moving funds
- Other system operations

### Example

```dsl
proposal_lifecycle "prop-001" quorum=0.6 threshold=0.5 {
    emit "Executing proposal prop-001..."
    mint community_coin "project_fund" 1000.0 "Allocated from treasury"
}
```

### How It Works

When the macro is expanded, it generates the following sequence of operations:

1. Creates a new proposal with the given ID
2. Sets the quorum threshold for voting participation
3. Sets the vote threshold for proposal approval
4. Includes the execution block commands

This ensures consistent proposal handling across the system while reducing boilerplate code.

### Best Practices

1. Use descriptive proposal IDs that indicate the purpose
2. Set appropriate quorum and threshold values based on the proposal's importance
3. Include clear emit messages to track proposal execution
4. Keep execution blocks focused and minimal
5. Document the purpose of the proposal in comments 