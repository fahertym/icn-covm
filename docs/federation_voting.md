# Federated Proposal Voting

This document describes the federation proposal voting system implemented in ICN-COVM.

## Overview

The Federated Proposal Voting system allows proposals to be broadcast across the federation network, voted on by members of different nodes, and tallied to determine the winning option using ranked-choice voting. The system enforces voting eligibility based on cooperative membership, supports different voting models, and includes time-based expiry for voting periods.

## Key Components

The system consists of the following key components:

1. **FederatedProposal**: A data structure representing a proposal with multiple options, scope, voting model, and optional expiry time
2. **FederatedVote**: A data structure representing a ranked-choice vote with signature verification
3. **ProposalScope**: Defines which cooperatives can participate in voting
4. **VotingModel**: Defines how votes are tallied (one-member-one-vote or one-coop-one-vote)
5. **Proposal Expiry**: Time-based limits on voting periods and execution timing
6. **NetworkMessage variants**: Message types for broadcasting proposals and submitting votes
7. **CLI Commands**: User interface for interacting with the federation voting system
8. **Storage**: Persistence of proposals and votes across nodes
9. **Identity Verification**: Cryptographic signature validation for votes

## Message Types

### ProposalScope

```rust
pub enum ProposalScope {
    /// Only members of the specified cooperative can vote
    SingleCoop(String),
    
    /// Only members of the listed cooperatives can vote
    MultiCoop(Vec<String>),
    
    /// All federation members can vote regardless of cooperative
    GlobalFederation,
}
```

### VotingModel

```rust
pub enum VotingModel {
    /// Each member gets one vote (traditional direct democracy)
    OneMemberOneVote,
    
    /// Each cooperative gets one vote (federated representation)
    OneCoopOneVote,
}
```

### FederatedProposal

```rust
pub struct FederatedProposal {
    pub proposal_id: String,
    pub namespace: String,
    pub options: Vec<String>,
    pub creator: String,
    pub created_at: i64,
    pub scope: ProposalScope,
    pub voting_model: VotingModel,
    pub expires_at: Option<i64>,
}
```

- `proposal_id`: Unique identifier for the proposal
- `namespace`: Categorization of the proposal (e.g., "membership", "funding")
- `options`: List of available voting options
- `creator`: Identifier of the proposal creator
- `created_at`: Timestamp when the proposal was created
- `scope`: Defines which cooperatives can participate in voting
- `voting_model`: Defines how votes are tallied
- `expires_at`: Optional Unix timestamp when voting closes and execution becomes available

### FederatedVote

```rust
pub struct FederatedVote {
    pub proposal_id: String,
    pub voter: String,
    pub ranked_choices: Vec<f64>,
    pub message: String,
    pub signature: String,
}
```

- `proposal_id`: ID of the proposal being voted on
- `voter`: Identifier of the person voting
- `ranked_choices`: Numeric preference values for each option
- `message`: The canonical message that was signed
- `signature`: Cryptographic signature to verify vote authenticity

## Voting Eligibility

Vote eligibility is enforced based on the proposal scope:

1. **SingleCoop**: Only members of the specific cooperative can vote
2. **MultiCoop**: Only members of the listed cooperatives can vote
3. **GlobalFederation**: All federation members can vote

A vote will be rejected if the voter doesn't meet the eligibility requirements for the proposal's scope.

## Voting Models

The system supports two voting models:

1. **OneMemberOneVote**: Each member's vote is counted individually (direct democracy)
2. **OneCoopOneVote**: Only one vote per cooperative is counted - the latest vote from each cooperative (federated representation)

## Identity and Signature Verification

Each vote must be cryptographically signed to ensure authenticity:

1. Each voter must have a registered identity with:
   - A unique `id`
   - A `public_key` stored in the identity record
   - A `crypto_scheme` defining the signature algorithm (e.g., "ed25519", "secp256k1")

2. When submitting a vote, the voter:
   - Creates a canonical message
   - Signs it with their private key
   - Includes both the message and signature in the vote

3. When a vote is received:
   - The system loads the voter's identity from storage
   - Verifies the signature using the stored public key
   - Checks that the voter is eligible based on proposal scope
   - Ensures the voter hasn't already voted on this proposal
   - If everything checks out, the vote is recorded

## CLI Commands

The system provides command-line tools for participating in federated voting:

### Broadcasting a Proposal

```bash
cargo run -- federation broadcast-proposal proposal.icn --scope global --model member --expires-in 86400
```

Available scope options:
- `single`: Only members of the creator's cooperative can vote
- `multi`: Only members of the specified cooperatives can vote (requires `--coops`)
- `global`: All federation members can vote (default)

Available model options:
- `member`: Each member gets one vote (default)
- `coop`: Each cooperative gets one vote

Time-related options:
- `--expires-in SECONDS`: Set the proposal to expire after the specified number of seconds (e.g., 86400 for 24 hours)

For multi-coop scope:
```bash
cargo run -- federation broadcast-proposal proposal.icn --scope multi --coops coopA,coopB,coopC --expires-in 604800
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
cargo run -- federation submit-vote vote.icn
```

The vote file should have this format:
```
proposal_id
voter
ranked_choice1,ranked_choice2,...
canonical_message_to_sign
base64_encoded_signature
```

The message and signature fields are optional for testing but required in production.
If not provided, a canonical message will be generated automatically, and the signature
will be set to "valid" for testing purposes only.

### Executing a Proposal

```bash
cargo run -- federation execute-proposal prop-2023-07-15
```

Or to force execution before the expiry time:

```bash
cargo run -- federation execute-proposal prop-2023-07-15 --force
```

This command:
1. Checks if the proposal has expired (refuses execution unless --force is used)
2. Collects all votes for the specified proposal
3. Filters votes based on eligibility and voting model
4. Tabulates the results using ranked-choice voting
5. Announces the winning option

## Proposal Expiry

Proposals can have an optional expiry time which enforces two constraints:

1. **Vote Submission**: Votes cannot be submitted after a proposal has expired
2. **Execution Timing**: Proposal execution (vote tallying) is only allowed after expiry unless forced

This ensures that:
- All voters have a clear deadline for submitting their votes
- Results aren't tallied prematurely before all votes are in
- The system has a clear transition from voting phase to execution phase

When a proposal has expired:
- Any attempt to submit a vote will be rejected with an error message
- The proposal can be executed to tally votes and determine the winner

## Storage

Proposals and votes are stored both in-memory (for performance) and in persistent storage (for durability).

### Storage Keys

- Proposals: `federation/proposals/{proposal_id}`
- Votes: `federation/votes/{proposal_id}`
- Identities: `identity/identities/{identity_id}`

## Federation Network Flow

1. **Proposal Creation**: A node creates a proposal with multiple options, specifying scope and voting model
2. **Broadcast**: The proposal is broadcast to all connected federation nodes
3. **Vote Collection**: Each node collects signed votes from eligible members
4. **Vote Validation**: Each vote is validated for authenticity and eligibility
5. **Execution**: Any node can trigger the vote counting process
6. **Result Calculation**: Votes are tallied according to the specified voting model
7. **Result Announcement**: The winning option is announced

## Programming Interface

The system provides VM operations for vote validation:

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
- **Scope Enforcement**: Only eligible voters (based on proposal scope) can participate
- **Vote Counting Integrity**: Votes are tallied according to the specified voting model

## Example Workflow

1. Cooperative A creates a membership proposal with 3 options, scope set to MultiCoop for coops A and B
2. The proposal is broadcast to all federation nodes
3. Members of Cooperatives A and B submit their cryptographically signed, ranked votes
4. The votes are validated against each member's identity and eligibility
5. Cooperative C executes the vote tally
6. If the voting model is OneCoopOneVote, only the latest vote from each cooperative is counted
7. The result shows which option won based on ranked-choice voting
8. All cooperatives implement the winning option

## Testing

For testing purposes:
- The signature "valid" or "mock_signature" will be accepted as valid without cryptographic verification
- Identity checks can be bypassed in development mode
- Default values are provided for scope (global) and voting model (member)

## Future Enhancements

- Liquid democracy with vote delegation
- Proposal templates and inheritance
- Time-limited voting periods
- Quorum requirements
- Real-time voting updates
- Multi-signature proposals
- Off-chain verification of vote results

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