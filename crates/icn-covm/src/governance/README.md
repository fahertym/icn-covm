# Governance Module

## Overview

The Governance module provides mechanisms for cooperative decision-making, proposal management, and voting within the Cooperative Value Network. It implements transparent, auditable governance processes for community-driven decision making.

## Core Features

1. **Proposal Lifecycle Management**
   - Creation, deliberation, voting, and execution
   - Configurable voting periods and quorum requirements
   - Transparent status tracking

2. **Voting Mechanisms**
   - One-member-one-vote
   - Reputation-weighted voting
   - Ranked-choice voting
   - Liquid democracy (vote delegation)

3. **Template System**
   - Reusable governance process templates
   - Customizable parameters
   - Version tracking for templates

4. **Integration with Other Modules**
   - Identity verification for voting eligibility
   - Storage for proposals and votes
   - VM operations for proposal execution

## Architecture

The governance module is organized around the concept of "proposals" - formal requests for changes that follow a defined lifecycle:

```
                    ┌─────────────┐
                    │  Templates  │◄────┐
                    └─────┬───────┘     │
                          │             │
                          ▼             │
┌──────────┐      ┌─────────────┐      │
│  Author  │─────►│   Proposal  │──────┘
└──────────┘      └─────┬───────┘
                        │
                        ▼
┌──────────┐      ┌─────────────┐     ┌─────────────┐
│ Identity │◄────►│    Voting   │────►│  Execution  │
└──────────┘      └─────────────┘     └─────────────┘
       ▲                 ▲                  │
       │                 │                  │
       └─────────────────┴──────────────────┘
```

### Key Components

1. **ProposalManager**: Handles the creation and lifecycle management of proposals
2. **VotingStrategies**: Different voting mechanisms (simple, ranked, etc.)
3. **TemplateRegistry**: Manages reusable governance templates
4. **ProposalExecutor**: Executes approved proposals

## Core APIs

### Proposal Management

```rust
// Create a new proposal
let proposal_id = proposal_manager.create_proposal(
    "Resource allocation",
    "Allocate 1000 tokens to project X",
    template_id,
    parameters,
    author_id,
);

// Get proposal status
let status = proposal_manager.get_status(proposal_id);

// List active proposals
let active = proposal_manager.list_proposals(ProposalStatus::Active);

// Execute a passed proposal
proposal_manager.execute_proposal(proposal_id);
```

### Voting

```rust
// Cast a vote on a proposal
voting_manager.cast_vote(proposal_id, voter_id, vote_value, signature);

// Delegate voting power
voting_manager.delegate_vote(from_id, to_id, signature);

// Calculate results
let result = voting_manager.calculate_results(proposal_id);

// Check if a proposal has passed
let passed = voting_manager.has_passed(proposal_id);
```

### Template Management

```rust
// Create a new template
let template_id = template_registry.create_template(
    "Simple majority vote",
    template_definition,
    author_id,
);

// List available templates
let templates = template_registry.list_templates();

// Use a template for a proposal
let proposal_id = proposal_manager.create_from_template(
    template_id,
    proposal_params,
    author_id,
);
```

## Template System

Governance templates provide reusable patterns for decision-making processes. A template defines:

1. **Voting Parameters**
   - Required quorum (minimum participation)
   - Approval threshold (e.g., simple majority, 2/3 majority)
   - Voting period duration

2. **Eligibility Rules**
   - Who can vote (e.g., all members, specific roles)
   - Voting weight calculation

3. **Execution Logic**
   - What happens when the proposal passes
   - Conditional execution based on voting results

### Example Template

```json
{
  "name": "Resource Allocation",
  "version": "1.0",
  "parameters": {
    "quorum": 0.25,
    "threshold": 0.5,
    "deliberation_period": "3d",
    "voting_period": "7d",
    "execution_delay": "1d"
  },
  "eligibility": {
    "required_role": "member",
    "minimum_reputation": 10
  },
  "execution": {
    "resource": "${resource}",
    "amount": "${amount}",
    "recipient": "${recipient}"
  }
}
```

## DSL Integration

The governance module provides DSL operations for use in VM programs:

```
// Create a proposal from a template
proposal_create "Resource allocation" using "simple_majority" {
  description: "Allocate 1000 tokens to project X",
  resource: "community_token",
  amount: 1000,
  recipient: "project_x"
}

// Check voting status
proposal_status "proposal_123"

// Vote on a proposal
proposal_vote "proposal_123" approve

// Check if passed
if_passed "proposal_123" {
  // Execute when passed
}
```

## Example Usage Scenarios

### Community Fund Allocation

```rust
// Create a template for fund allocation
let template_id = template_registry.create_template(
    "Fund Allocation",
    fund_allocation_template,
    admin_id,
);

// Create a proposal using the template
let proposal_id = proposal_manager.create_from_template(
    template_id,
    {
        "amount": "1000",
        "recipient": "project_x",
        "justification": "For development of feature Y"
    },
    proposer_id,
);

// Community members vote
voting_manager.cast_vote(proposal_id, voter1_id, Vote::Approve, signature1);
voting_manager.cast_vote(proposal_id, voter2_id, Vote::Reject, signature2);
// ...

// After voting period ends, check if passed
if voting_manager.has_passed(proposal_id) {
    // Execute the fund transfer
    proposal_manager.execute_proposal(proposal_id);
}
```

### Governance Parameter Change

```rust
// Create a proposal to change a governance parameter
let proposal_id = proposal_manager.create_proposal(
    "Update quorum requirement",
    "Change the quorum requirement from 25% to 20%",
    "parameter_change",
    {
        "parameter": "quorum",
        "current_value": "0.25",
        "new_value": "0.20"
    },
    proposer_id,
);

// Voting and execution follows the same pattern
``` 