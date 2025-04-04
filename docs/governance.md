# Governance Operations

This document describes the governance primitives available in the ICN Cooperative Virtual Machine (ICN-COVM). These operations provide building blocks for implementing democratic decision-making processes within cooperatives.

## Table of Contents

1. [RankedVote](#rankedvote)
2. [LiquidDelegate](#liquiddelegate)
3. [VoteThreshold](#votethreshold)
4. [Identity System](#identity-system)

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

## VoteThreshold

The `VoteThreshold` operation checks if the total voting power in favor of a proposal meets a specified threshold for execution.

### Signature

```
votethreshold <threshold>
```

- `threshold`: Minimum voting power required for the proposal to pass

### Description

The `VoteThreshold` operation acts as a governance gatekeeper, ensuring that proposals only execute when they have sufficient support. It compares the total voting power in favor (from the top of the stack) with the specified threshold, then pushes a truthy or falsey value to be used in conditional execution.

This operation is typically used after calculating total support, often in conjunction with `LiquidDelegate` to account for delegated voting power.

### Stack Behavior

**Before operation**:
```
[total_support]
```

**After operation**:
```
[result]
```

Where `result` is:
- `0.0` (truthy) if `total_support >= threshold`
- `1.0` (falsey) if `total_support < threshold`

### Example

```
# Calculate total supporting voting power
# (in a real scenario, this would come from actual votes)
push 3.5  # Example: 3.5 votes in favor

# Check against a threshold of 3.0
push 3.0
votethreshold

# Conditional execution based on the result
if:
    emit "Proposal approved! Executing..."
    # Execution logic here
else:
    emit "Proposal rejected. Insufficient support."
```

### Error Handling

The operation will fail with an error if:
- There are no values on the stack

### Real-world Applications

- Policy approval with minimum support requirements
- Fund distribution requiring sufficient stakeholder backing
- Constitutional changes needing super-majority support
- Quorum validation for vote legitimacy

## Identity System

The ICN-COVM identity system provides a foundation for secure, verifiable, and persistent cooperative identities. These primitives enable attributable decision-making, cryptographic verification, and delegated authority within cooperative governance.

### Core Identity Objects

#### Identity

The fundamental identity structure representing any entity in the system.

**Attributes**:
- `id`: Unique identifier
- `public_key`: Cryptographic public key (optional)
- `identity_type`: Type of identity (e.g., "cooperative", "member", "service")
- `crypto_scheme`: Cryptographic scheme used (e.g., "ed25519", "secp256k1")
- `metadata`: Additional information about this identity
- `version_info`: Version history for this identity

#### MemberProfile

Extended profile information for cooperative members.

**Attributes**:
- `identity`: The core identity this profile is associated with
- `roles`: Member-specific roles within their cooperative
- `reputation`: Optional reputation score
- `joined_at`: Timestamp when the member joined
- `attributes`: Additional profile fields
- `version_info`: Version history for this profile

#### Credential

Verifiable credentials that can be issued to identities.

**Attributes**:
- `id`: Unique identifier for this credential
- `credential_type`: Type of credential (e.g., "membership", "voting_right")
- `issuer_id`: Identity that issued this credential
- `holder_id`: Identity that holds this credential
- `issued_at`: Timestamp when issued
- `expires_at`: Optional expiration timestamp
- `signature`: Cryptographic signature from the issuer
- `claims`: Associated attributes/claims
- `version_info`: Version history

#### DelegationLink

A cryptographically signed delegation from one identity to another.

**Attributes**:
- `id`: Unique identifier for this delegation
- `delegator_id`: Identity of the delegator (who is delegating authority)
- `delegate_id`: Identity of the delegate (who receives the authority)
- `delegation_type`: Type of delegation (e.g., "voting", "admin")
- `permissions`: Specific permissions granted
- `created_at`: Timestamp when created
- `expires_at`: Optional expiration timestamp
- `signature`: Cryptographic signature from the delegator
- `attributes`: Additional context for this delegation
- `version_info`: Version history

### Identity Operations

#### `getcaller`

Returns the ID of the current caller.

**Stack Behavior**:
```
[] -> [caller_id]
```

#### `requirerole`

Aborts execution if the caller does not have the specified role.

**Signature**:
```
requirerole "role_name"
```

#### `hasrole`

Checks if the caller has a specific role.

**Signature**:
```
hasrole "role_name"
```

**Stack Behavior**:
```
[] -> [result]
```

Where `result` is:
- `0.0` (truthy) if the caller has the role
- `1.0` (falsey) if the caller does not have the role

#### `verifysignature`

Verifies a cryptographic signature against a message.

**Signature**:
```
verifysignature
```

**Stack Behavior**:
```
[message, signature, public_key, scheme] -> [result]
```

Where `result` is:
- `0.0` (truthy) if the signature is valid
- `1.0` (falsey) if the signature is invalid

### Namespaced Storage

The identity system uses hierarchical namespaces for persistent storage:

- **Cooperatives**: `coops/{coop_id}/`
- **Members**: `coops/{coop_id}/members/{member_id}/` or `members/{member_id}/`
- **Credentials**: `credentials/{credential_type}/{credential_id}/`
- **Delegations**: `delegations/{delegation_type}/{delegation_id}/`

Each namespace has configurable:
- Resource quotas
- Access control policies
- Versioning for all stored objects

### Integration with Governance Primitives

The identity system enhances existing governance primitives:

- **RankedVote**: Track who voted and verify voter eligibility
- **LiquidDelegate**: Use cryptographically signed delegation links
- **VoteThreshold**: Verify voter identities and credentials

### Real-world Applications

- Secure member onboarding and authentication
- Role-based access control for cooperative resources
- Transparent voting with cryptographic verification
- Delegation chains with accountable authority transfer
- Credential issuance and verification
- Key rotation and revocation

## Combining Governance Operations

These governance primitives can be combined to create sophisticated democratic systems:

1. Use `LiquidDelegate` to establish a delegation network
2. Use `RankedVote` to conduct an election, with delegates casting votes according to their delegated voting power
3. Use `VoteThreshold` to ensure the winning proposal has sufficient support before execution

### Complete Governance Flow Example

```
# 1. Set up delegations
liquiddelegate "alice" "bob"      # Alice delegates to Bob
liquiddelegate "dave" "carol"     # Dave delegates to Carol

# 2. Conduct ranked-choice vote with 3 candidates
# (Ballots are pushed onto the stack)
rankedvote 3 3

# 3. Store the winner
store "winning_proposal"

# 4. Calculate support for the winning proposal
push 4.0  # Example: 4.0 votes in favor

# 5. Check against threshold
push 3.0  # Require at least 3.0 votes
votethreshold

# 6. Conditionally execute the winning proposal
if:
    emit "Executing winning proposal..."
    # Implementation logic here
else:
    emit "Winning proposal lacks sufficient support."
    # Rejection handling here
```

Additional governance operations will be added in future releases to further enhance the cooperative governance capabilities of ICN-COVM. 