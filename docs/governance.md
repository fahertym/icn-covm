# Governance System

The ICN Cooperative Virtual Machine (ICN-COVM) provides a comprehensive governance system that enables democratic decision-making within cooperatives and federations. This document describes the governance primitives, proposal lifecycle, and operations available in the system.

## Proposal Lifecycle

Proposals in the ICN-COVM follow a defined lifecycle with the following states:

1. **Draft** - Initial creation state where the proposal can be edited
2. **OpenForFeedback** (Deliberation) - Published for community feedback and discussion
3. **Active** - Ready for voting after minimum deliberation period
4. **Voting** - Accepting votes from eligible members
5. **Executed** - Proposal passed and executed
6. **Rejected** - Proposal failed to meet required threshold
7. **Expired** - Voting period ended without required participation

### State Transitions

Valid state transitions follow this progression:
- Draft → OpenForFeedback (Deliberation)
- OpenForFeedback → Active (after minimum deliberation period)
- Active → Voting
- Voting → Executed/Rejected/Expired

Each transition is recorded in the proposal's history with a timestamp, allowing for complete audit trails of the proposal's journey through the governance system.

## Voting Models

The ICN-COVM supports multiple voting models to accommodate different governance structures:

### RankedVote

```
Signature: rankedvote(proposal_id: String, ...ranks: Vec<u32>) -> bool
```

**Description:**  
Casts a ranked-choice vote for options in a proposal.

**Stack Behavior:**
- Before: [ ... ]
- After:  [ ... result ]

**Parameters:**
- `proposal_id`: The unique identifier of the proposal
- `ranks`: A sequence of integers representing ranked choices (1st, 2nd, 3rd, etc.)

**Errors:**
- Invalid proposal ID
- Proposal not in voting state
- Invalid rank values
- Unauthorized voter

**Real-world Application:**
Used for selecting between multiple options where preference order matters, such as electing board members or deciding between competing proposals.

### LiquidDelegate

```
Signature: liquid_delegate(delegate_to: String, scope: String) -> bool
```

**Description:**  
Delegates voting power to another member for a specific proposal scope.

**Stack Behavior:**
- Before: [ ... ]
- After:  [ ... success ]

**Parameters:**
- `delegate_to`: The DID of the member receiving delegation
- `scope`: The scope of delegation (e.g., "budgeting", "membership", "all")

**Errors:**
- Invalid delegate identity
- Self-delegation
- Circular delegation
- Unauthorized operation

**Real-world Application:**
Allows members to delegate their voting power to trusted representatives for domains they have less expertise in, while maintaining direct voting on areas they are most knowledgeable about.

### VoteThreshold

```
Signature: vote_threshold(proposal_id: String, choice: String) -> bool
```

**Description:**  
Casts a simple threshold-based vote (yes/no/abstain) on a proposal.

**Stack Behavior:**
- Before: [ ... ]
- After:  [ ... success ]

**Parameters:**
- `proposal_id`: The unique identifier of the proposal
- `choice`: The vote choice ("yes", "no", or "abstain")

**Errors:**
- Invalid proposal ID
- Proposal not in voting state
- Invalid choice value
- Unauthorized voter

**Real-world Application:**
Used for binary decisions or simple threshold voting, such as approving budgets, changes to bylaws, or acceptance of new members.

## Identity System

Identity management is a crucial component of the governance system, ensuring that only eligible members can participate in governance activities.

### VerifyMembership

```
Signature: verify_membership(identity: String, group: String) -> bool
```

**Description:**  
Verifies if an identity belongs to a specific membership group.

**Stack Behavior:**
- Before: [ ... ]
- After:  [ ... is_member ]

**Parameters:**
- `identity`: The DID of the member to verify
- `group`: The membership group to check against

**Errors:**
- Invalid identity format
- Unknown group

**Real-world Application:**
Used to check eligibility for voting or proposal creation based on membership status, roles, or other group affiliations.

## Proposal Storage Structure

Proposals and related data are stored in the following structure:

- `governance/proposals/<id>` - The main proposal object
- `governance/proposals/<id>/attachments/<name>` - Attached files
- `governance/proposals/<id>/votes/<user_did>` - Individual votes
- `governance/proposals/<id>/comments/<comment_id>` - Comments on the proposal
- `governance/logic/<id>.dsl` - Executable proposal logic

## Comments and Deliberation

The deliberation phase of proposals is supported through a threaded comment system:

- Comments can be nested (replies to other comments)
- Comments are tagged with author identity and timestamp
- Comments can be sorted by time or author
- A minimum deliberation period can be enforced before proceeding to voting
- Each comment has a unique ID, allowing for threaded discussions
- Comments support flexible storage paths, including both modern and legacy formats
- Comments can be displayed in a hierarchical threaded format showing reply relationships

## Proposal Attachments

Proposals support the attachment of files and documents:

- Attachments are stored at `governance/proposals/<id>/attachments/<n>`
- Each attachment has a unique name and file reference
- Attachments can be added at any stage of the proposal lifecycle
- Attachments provide supporting documentation for proposal evaluation
- Command line interface provides tools for adding, listing, and retrieving attachments

## Quorum and Threshold Requirements

Each proposal includes configurable parameters for:

- **Quorum**: The minimum number of participants required for a vote to be valid
- **Threshold**: The minimum proportion of "yes" votes required for a proposal to pass
- **Required Participants**: Optional minimum number of unique participants in deliberation

## Governance Templates

ICN-COVM supports reusable governance templates that allow organizations to define standardized governance configurations. These templates can specify common parameters such as quorum thresholds, voting thresholds, deliberation periods, and required roles.

Templates provide several benefits:
- Consistency across similar types of proposals
- Simplified proposal creation with predefined settings
- Standardized governance processes for different organizational contexts
- Reduced configuration errors

For more information on governance templates, see the [Governance Templates documentation](governance_templates.md).

## Federation Governance

For multi-cooperative federations, additional governance features are available:

- **ProposalScope**: Defines which cooperatives can participate in voting
  - SingleCoop: Only members of the specified cooperative
  - MultiCoop: Only members of the listed cooperatives
  - GlobalFederation: All federation members regardless of cooperative

- **VotingModel**: Defines how votes are counted
  - OneMemberOneVote: Traditional direct democracy
  - OneCoopOneVote: Federated representation

## Executing Proposals

When a proposal reaches the "Executed" state, associated logic can be automatically executed. This logic is defined using the DSL (Domain Specific Language) and stored in the `governance/logic/<id>.dsl` path.

Example logic might:
- Allocate budgets
- Update system parameters
- Record decisions
- Trigger external actions

## Auditing and Transparency

All governance actions are recorded in the audit log, ensuring transparency and accountability:

- Proposal creation, transitions, and execution
- Votes cast and by whom
- Delegations made
- Comments and deliberation

This comprehensive record allows for both real-time monitoring and retrospective analysis of governance activity. 