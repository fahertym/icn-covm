# Federated Proposal Voting

This document describes the federation proposal voting system implemented in ICN-COVM.

## Overview

The Federated Proposal Voting system allows proposals to be broadcast across the federation network, voted on by members of different nodes, and tallied to determine the winning option using ranked-choice voting.

## Key Components

The system consists of the following key components:

1. **FederatedProposal**: A data structure representing a proposal with multiple options
2. **FederatedVote**: A data structure representing a ranked-choice vote
3. **NetworkMessage variants**: Message types for broadcasting proposals and submitting votes
4. **CLI Commands**: User interface for interacting with the federation voting system
5. **Storage**: Persistence of proposals and votes across nodes

## Message Types

### FederatedProposal

```rust
pub struct FederatedProposal {
    pub proposal_id: String,
    pub namespace: String,
    pub options: Vec<String>,
    pub creator: String,
    pub created_at: i64,
}
```

- `proposal_id`: Unique identifier for the proposal
- `namespace`: Categorization of the proposal (e.g., "membership", "funding")
- `options`: List of available voting options
- `creator`: Identifier of the proposal creator
- `created_at`: Timestamp when the proposal was created

### FederatedVote

```rust
pub struct FederatedVote {
    pub proposal_id: String,
    pub voter: String,
    pub ranked_choices: Vec<f64>,
    pub signature: String,
}
```

- `proposal_id`: ID of the proposal being voted on
- `voter`: Identifier of the person voting
- `ranked_choices`: Numeric preference values for each option
- `signature`: Cryptographic signature to verify authenticity (placeholder for now)

## CLI Commands

The system provides command-line tools for participating in federated voting:

### Broadcasting a Proposal

```bash
cargo run -- federation broadcast-proposal demo/federation/expand.icn
```

The proposal file should have this format:
```
proposal_id
namespace
creator
option1
option2
...
```

### Submitting a Vote

```bash
cargo run -- federation submit-vote demo/federation/vote_alice.icn
```

The vote file should have this format:
```
proposal_id
voter
ranked_choice1,ranked_choice2,...
```

### Executing a Proposal

```bash
cargo run -- federation execute-proposal prop-2023-07-15
```

This command:
1. Collects all votes for the specified proposal
2. Tabulates the results using ranked-choice voting
3. Announces the winning option

## Storage

Proposals and votes are stored both in-memory (for performance) and in persistent storage (for durability).

### Storage Keys

- Proposals: `federation/proposals/{proposal_id}`
- Votes: `federation/votes/{proposal_id}`
- Results: `federation/proposals/{proposal_id}/winner`

## Federation Network Flow

1. **Proposal Creation**: A node creates a proposal with multiple options
2. **Broadcast**: The proposal is broadcast to all connected federation nodes
3. **Vote Collection**: Each node collects votes from its local members
4. **Vote Submission**: Votes are submitted to the network
5. **Execution**: Any node can trigger the vote counting process
6. **Result Propagation**: The result is stored and accessible to all nodes

## Example Workflow

1. Cooperative A creates a membership proposal with 3 options
2. The proposal is broadcast to Cooperatives B and C
3. Members of all three cooperatives submit their ranked votes
4. Cooperative B executes the vote tally
5. The result shows "Create mentorship program" won
6. All cooperatives implement the winning option

## Future Enhancements

- Cryptographic signatures for vote verification
- Delegation of voting power
- Proposal templates and inheritance
- Time-limited voting periods
- Quorum requirements
- Real-time voting updates

## Demo Files

The system includes several demo files to illustrate the workflow:

- `demo/federation/expand.icn`: Example proposal for expanding membership
- `demo/federation/vote_alice.icn`: Alice's vote on the proposal
- `demo/federation/vote_bob.icn`: Bob's vote on the proposal
- `demo/federation/vote_carol.icn`: Carol's vote on the proposal
- `demo/federation/federated_vote_execute.dsl`: DSL script demonstrating vote execution

# Federated Voting with Digital Signatures

This document explains the signature-based verification flow for voting in the ICN-COVM federation system.

## Overview

The federated voting system uses cryptographic signatures to ensure:

1. **Authenticity** - Votes come from legitimate federation members
2. **Integrity** - Votes haven't been tampered with
3. **Non-repudiation** - Voters cannot deny having cast their vote

## Identity Requirements

Each voter must have a registered identity in the federation with:

- A unique `id`
- A `public_key` stored in the identity record
- A `crypto_scheme` defining the signature algorithm (e.g., "ed25519", "secp256k1")

## Voting Process

### 1. Creating a Vote

When a member wants to vote on a proposal:

```
# 1. Create a canonical message that includes:
message = "Vote from {voter_id} on proposal {proposal_id} with choices {choice_values}"

# 2. Sign the message with their private key
signature = sign(message, private_key)

# 3. Create the vote structure with:
vote = {
  proposal_id: "prop-id",
  voter: "voter-id",
  ranked_choices: [2.0, 1.0, 0.0],
  message: message,
  signature: signature
}
```

### 2. Submitting a Vote

Votes can be submitted using the CLI:

```bash
icn-covm federation submit-vote my_vote.icn
```

Where `my_vote.icn` contains:

```
prop-2023-07-15
alice
2.0,1.0,0.0
vote for prop-2023-07-15 by alice
<base64-signature>
```

### 3. Verification Process

When the vote is submitted:

1. The system loads the voter's identity from storage
2. It verifies the signature using the stored public key
3. It checks that the voter is eligible based on proposal scope
4. If everything checks out, the vote is recorded

## Programming Interface

The system provides new VM operations for validation:

- `GetIdentity` - Loads an identity from storage
- `RequireValidSignature` - Verifies a signature against an identity's public key

Example DSL code for validation:

```
# Get the identity
GetIdentity "alice"

# Verify the signature
RequireValidSignature {
  voter: "alice",
  message: "vote for prop-2023-07-15 by alice",
  signature: "base64-encoded-signature"
}
```

## Security Considerations

- **Private Key Security**: Voters must keep their private keys secure
- **Public Key Verification**: Federation nodes should verify the authenticity of public keys
- **Replay Protection**: The message format includes specific proposal and voter IDs to prevent replay attacks
- **Multiple Votes**: The system prevents a voter from voting more than once on the same proposal

## Testing

For testing purposes, the signature "valid" or "mock_signature" will be accepted as valid without cryptographic verification. 