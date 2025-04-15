# DSL Governance Features

This document describes the governance-related features available in the ICN-COVM Domain Specific Language (DSL).

## Governance Blocks

The DSL supports defining governance parameters using special blocks and directives.

### Basic Governance Block

The basic governance block defines parameters for a proposal:

```
governance {
    quorumthreshold 0.6      # Minimum participation required (as fraction)
    votethreshold 0.5        # Threshold for approval (as fraction)
    mindeliberation 72h      # Minimum deliberation period (hours)
    expiresin 14d            # Voting period expiration (days)
    require_role "member"    # Required role to vote
}
```

### Governance Templates

Templates allow defining reusable governance configurations:

```
template "standard_budget" {
    quorumthreshold 0.5
    votethreshold 0.6
    mindeliberation 48h
    expiresin 7d
    require_role "member"
}
```

Templates can be referenced using the `governance use` directive:

```
governance use "standard_budget"
```

When both a template and explicit governance block are used, the explicit settings override the template:

```
governance use "standard_budget"

governance {
    quorumthreshold 0.7      # Overrides template value
    require_role "finance"   # Overrides template value
}
```

For detailed information about templates, see the [Governance Templates documentation](governance_templates.md).

## Duration Format

Durations in governance blocks use a simple format:

- `Nh` - N hours (e.g., `48h` for 48 hours)
- `Nd` - N days (e.g., `7d` for 7 days)
- `Nw` - N weeks (e.g., `2w` for 2 weeks)

## Governance Parameters

### quorumthreshold

Sets the minimum participation required for a valid vote.

```
quorumthreshold 0.6  # 60% of eligible voters must participate
```

### votethreshold

Sets the minimum proportion of "yes" votes required for a proposal to pass.

```
votethreshold 0.5    # More than 50% of votes must be "yes"
```

### mindeliberation

Sets the minimum time a proposal must be in the deliberation phase before voting.

```
mindeliberation 72h  # 72 hours (3 days) of deliberation required
```

### expiresin

Sets how long the voting period lasts before the proposal expires.

```
expiresin 14d        # 14 days of voting before expiration
```

### require_role

Specifies which role(s) are eligible to vote on the proposal.

```
require_role "member"     # Only members can vote
require_role "guardian"   # Only guardians can vote
```

Multiple roles can be specified in a single governance block:

```
governance {
    require_role "member"
    require_role "verified"
}
```

## Integration with Proposal Execution

The governance parameters defined in the DSL are extracted during parsing and applied to the proposal when it's created:

1. The DSL is parsed with `parse_dsl()`, which returns both the operations and the governance config
2. The governance configuration is merged with command-line arguments when creating the proposal
3. The resulting configuration determines the proposal's lifecycle behavior

## Example: Complete Governance Flow

Here's a complete example showing a template definition, its usage, and additional logic:

```
# Define a standard budget template
template "standard_budget" {
    quorumthreshold 0.5
    votethreshold 0.6
    mindeliberation 48h
    expiresin 7d
    require_role "member"
}

# Use the template for this proposal
governance use "standard_budget"

# Override specific settings
governance {
    quorumthreshold 0.7
}

# Logic to execute if proposal passes
push 1000
push "fund:ops"
mint
```

## Proposal Execution Logic

After the governance blocks and template directives, the DSL contains the actual operations to execute if the proposal passes. These operations follow the standard DSL syntax and can use any available opcodes.

For more information on the full DSL syntax, see the [DSL Reference](dsl_reference.md) and [Standard Library](stdlib.md) documentation. 