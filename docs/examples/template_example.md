# Governance Templates Example

This document provides examples of how to use the governance template system in ICN-COVM DSL.

## Basic Template Usage

Here's a basic example of defining and using a governance template:

```
# Define a standard template for budget proposals
template "standard_budget" {
    quorumthreshold 0.5      # 50% quorum
    votethreshold 0.6        # 60% approval threshold
    mindeliberation 48h      # 2 days of deliberation
    expiresin 7d             # 7 days to vote
    require_role "member"    # Only members can vote
}

# Use the template for this proposal
governance use "standard_budget"

# Proposal logic to execute if passed
push 1000
push "fund:operations"
mint
```

In this example:
1. We define a template named "standard_budget" with preset governance parameters
2. We apply the template using the `governance use` directive
3. The rest of the DSL contains the actual operations to execute if the proposal passes

## Multiple Templates

You can define multiple templates for different types of proposals:

```
# Define a standard template for routine decisions
template "standard" {
    quorumthreshold 0.5
    votethreshold 0.6
    mindeliberation 48h
    expiresin 7d
    require_role "member"
}

# Define an emergency template for urgent decisions
template "emergency" {
    quorumthreshold 0.3      # Lower quorum for quick action
    votethreshold 0.8        # Higher threshold for approval
    mindeliberation 1h       # Minimal deliberation
    expiresin 1d             # Short voting period
    require_role "guardian"  # Restricted to guardians
}

# Use the emergency template for this urgent action
governance use "emergency"

# Emergency allocation logic
push 500
push "fund:emergency"
mint
```

## Overriding Template Values

You can override specific template values with an explicit governance block:

```
# Define a standard template
template "standard" {
    quorumthreshold 0.5
    votethreshold 0.6
    mindeliberation 24h
    expiresin 7d
    require_role "member"
}

# Use the standard template
governance use "standard"

# Override specific settings
governance {
    quorumthreshold 0.7        # Higher quorum than the template
    mindeliberation 48h        # Longer deliberation than the template
}

# Proposal logic
push 1
push 2
add
```

In this example:
- The quorum threshold is overridden to 0.7 (instead of 0.5 from template)
- The minimum deliberation period is overridden to 48h (instead of 24h from template)
- Other parameters (threshold, expiration, required roles) are kept from the template

## Partial Templates

Templates can contain a subset of governance parameters:

```
# Define a minimal template with just threshold values
template "high_threshold" {
    quorumthreshold 0.7
    votethreshold 0.8
}

# Use the minimal template
governance use "high_threshold"

# Add additional parameters in explicit governance block
governance {
    mindeliberation 72h
    expiresin 14d
    require_role "verified"
}

# Proposal logic
push "Hello"
push "World"
concat
```

In this example:
- The template only sets quorum and voting thresholds
- The explicit governance block adds other parameters without affecting the thresholds

## Programmatic Usage

Templates are parsed at proposal creation time. When creating a proposal, the DSL code is analyzed to extract operations and governance configuration.

The following pseudocode shows how templates are processed:

```rust
// Parse DSL source
let (ops, config) = parse_dsl(dsl_source)?;

// The config now contains the merged result of all template applications
// and governance blocks

// Use the config for proposal creation
let proposal = ProposalLifecycle::new(
    proposal_id,
    creator,
    title,
    config.quorum.unwrap_or(0.6) * 100.0, // Default if not specified
    config.threshold.unwrap_or(0.5) * 100.0, // Default if not specified
    config.min_deliberation,
    None,
);

// Store the proposal with its configuration
store_proposal(proposal);

// Store the operations to execute if the proposal passes
store_proposal_logic(proposal_id, ops);
```

## Best Practices

1. **Define templates at the organization level** for consistent governance across proposals
2. **Name templates clearly** to reflect their purpose (e.g., "budget", "emergency", "membership")
3. **Use overrides sparingly** to maintain consistent governance
4. **Document your templates** so proposers understand when to use each template
5. **Consider role restrictions carefully** to ensure the right stakeholders can vote

## Common Templates

Here are some common templates you might want to define for your organization:

### Standard Decision Template

```
template "standard" {
    quorumthreshold 0.5
    votethreshold 0.6
    mindeliberation 72h
    expiresin 14d
    require_role "member"
}
```

### Budget Approval Template

```
template "budget" {
    quorumthreshold 0.6
    votethreshold 0.7
    mindeliberation 120h
    expiresin 10d
    require_role "member"
}
```

### Emergency Response Template

```
template "emergency" {
    quorumthreshold 0.3
    votethreshold 0.8
    mindeliberation 1h
    expiresin 24h
    require_role "guardian"
}
```

### Membership Change Template

```
template "membership" {
    quorumthreshold 0.7
    votethreshold 0.75
    mindeliberation 168h
    expiresin 14d
    require_role "member"
}
``` 