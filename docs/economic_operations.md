# Economic Operations

This document describes the economic operations available in the ICN Cooperative Virtual Machine (ICN-COVM). These operations provide the foundation for creating, managing, and transferring economic resources within cooperatives.

## Table of Contents

1. [Overview](#overview)
2. [CreateResource](#createresource)
3. [Mint](#mint)
4. [Transfer](#transfer)
5. [Burn](#burn)
6. [Balance](#balance)
7. [Storage Integration](#storage-integration)
8. [Usage Examples](#usage-examples)

## Overview

Economic operations enable cooperatives to manage resources, track balances, and facilitate transfers between accounts. These operations provide the foundation for:

- Creating and managing tokens or other digital resources
- Tracking members' contributions and allocations
- Implementing participatory budgeting systems
- Building reward and recognition mechanisms
- Supporting mutual credit and exchange systems within a community

All economic operations integrate with the storage system to maintain state between VM executions and ensure data integrity.

## CreateResource

The `CreateResource` operation creates a new economic resource that can be minted, transferred, and burned.

### Signature

```
createresource "resource_id"
```

- `resource_id`: A unique identifier for the resource

### Description

This operation registers a new economic resource in the storage system. Resource details should include:
- Descriptive information (name, description)
- Technical parameters (divisibility, transferability)
- Governance rules (issuance policies, authorized operators)

Resources are typically stored at path `resources/{resource_id}` with an empty balances record initialized at `resources/{resource_id}/balances`.

### Stack Behavior

This operation doesn't affect the stack.

### Example

```
# Create a new community token resource
createresource "community_token"

# Event is emitted automatically
# [EVENT] economic: Created new resource: community_token
```

### Error Handling

The operation will fail with an error if:
- A resource with the same ID already exists
- The storage system is unavailable
- The user doesn't have permission to create resources

## Mint

The `Mint` operation creates new units of a resource and assigns them to an account.

### Signature

```
mint "resource_id" "account_id" amount [reason]
```

- `resource_id`: The resource identifier
- `account_id`: The account receiving the new units
- `amount`: The quantity to mint (must be positive)
- `reason`: Optional reason for the mint operation

### Description

This operation increases the balance of an account for a specific resource by creating new units. It's typically used for:
- Initial allocation of a resource
- Rewarding members for contributions
- Implementing monetary policy within a cooperative economy

### Stack Behavior

This operation doesn't affect the stack.

### Example

```
# Mint 1000 units of community_token to the treasury account
mint "community_token" "treasury" 1000.0 "Initial allocation"

# Mint without a reason
mint "community_token" "member1" 100.0
```

### Error Handling

The operation will fail with an error if:
- The specified resource doesn't exist
- The amount is zero or negative
- The storage system is unavailable
- The user doesn't have permission to mint the resource

## Transfer

The `Transfer` operation moves units of a resource from one account to another.

### Signature

```
transfer "resource_id" "from_account" "to_account" amount [reason]
```

- `resource_id`: The resource identifier
- `from_account`: The source account
- `to_account`: The destination account
- `amount`: The quantity to transfer (must be positive)
- `reason`: Optional reason for the transfer

### Description

This operation decreases the balance of the source account and increases the balance of the destination account by the specified amount. It's used for:
- Member-to-member exchanges
- Budget allocations
- Reward distributions
- Project funding

### Stack Behavior

This operation doesn't affect the stack.

### Example

```
# Transfer 50 units from treasury to a project account
transfer "community_token" "treasury" "project_alpha" 50.0 "Initial project funding"

# Simple transfer between members
transfer "community_token" "alice" "bob" 25.0
```

### Error Handling

The operation will fail with an error if:
- The specified resource doesn't exist
- The source account has insufficient balance
- The amount is zero or negative
- The storage system is unavailable
- The user doesn't have permission to transfer from the source account

## Burn

The `Burn` operation destroys units of a resource, removing them from circulation.

### Signature

```
burn "resource_id" "account_id" amount [reason]
```

- `resource_id`: The resource identifier
- `account_id`: The account from which units will be burned
- `amount`: The quantity to burn (must be positive)
- `reason`: Optional reason for the burn operation

### Description

This operation decreases the balance of an account for a specific resource by destroying existing units. It's typically used for:
- Redemption of value for goods or services
- Implementing demurrage or decay in certain token systems
- Controlling supply in cooperative economic systems

### Stack Behavior

This operation doesn't affect the stack.

### Example

```
# Burn 100 units from a project account after completion
burn "community_token" "project_alpha" 100.0 "Project completed, remaining budget returned"

# Burn without a reason
burn "community_token" "member1" 10.0
```

### Error Handling

The operation will fail with an error if:
- The specified resource doesn't exist
- The account has insufficient balance
- The amount is zero or negative
- The storage system is unavailable
- The user doesn't have permission to burn from the account

## Balance

The `Balance` operation retrieves the current balance of a resource for an account.

### Signature

```
balance "resource_id" "account_id"
```

- `resource_id`: The resource identifier
- `account_id`: The account to check

### Description

This operation queries the storage system for the current balance of a specific resource for an account and pushes the result onto the stack.

### Stack Behavior

**Before operation**:
```
[]
```

**After operation**:
```
[account_balance]
```

Where `account_balance` is the current balance of the specified resource for the account.

### Example

```
# Get the balance of community_token for treasury
balance "community_token" "treasury"

# Use the balance in a calculation
balance "community_token" "treasury"
push 0.0
gt
if:
    emit "Treasury has funds"
else:
    emit "Treasury is empty"
```

### Error Handling

The operation will fail with an error if:
- The specified resource doesn't exist
- The storage system is unavailable
- The user doesn't have permission to read the account balance

If the account doesn't have a balance record, the operation returns 0.0 rather than failing.

## Storage Integration

Economic operations are tightly integrated with the storage system to maintain persistent state. The following storage paths are used:

- `resources/{resource_id}`: Resource metadata (JSON)
- `resources/{resource_id}/balances`: Account balances (JSON)

All economic operations generate events in the "economic" category for auditing and transparency.

## Usage Examples

### Create a Community Token

```
# Create the token resource
createresource "community_token"

# Initial allocation to founding members
mint "community_token" "founder1" 1000.0 "Founder allocation"
mint "community_token" "founder2" 1000.0 "Founder allocation"

# Create a community treasury
mint "community_token" "treasury" 8000.0 "Community fund"

# Fund a project from the treasury
transfer "community_token" "treasury" "project_team" 500.0 "Project Alpha funding"

# Check balances
balance "community_token" "treasury"
emit "Treasury balance:"
dumpstack

balance "community_token" "project_team"
emit "Project team balance:"
dumpstack
```

### Reward Distribution System

```
# Function to distribute rewards based on contributions
def distribute_rewards(total_amount):
    # Get contribution scores from storage
    loadp "contributions/alice"
    store alice_score
    
    loadp "contributions/bob"
    store bob_score
    
    loadp "contributions/carol"
    store carol_score
    
    # Calculate total score
    load alice_score
    load bob_score
    add
    load carol_score
    add
    store total_score
    
    # Calculate and distribute rewards proportionally
    load alice_score
    load total_score
    div
    load total_amount
    mul
    store alice_reward
    
    load bob_score
    load total_score
    div
    load total_amount
    mul
    store bob_reward
    
    load carol_score
    load total_score
    div
    load total_amount
    mul
    store carol_reward
    
    # Transfer rewards from treasury
    transfer "community_token" "treasury" "alice" alice_reward "Monthly contribution reward"
    transfer "community_token" "treasury" "bob" bob_reward "Monthly contribution reward"
    transfer "community_token" "treasury" "carol" carol_reward "Monthly contribution reward"
    
    # Emit completion event
    emitevent "economic" "Monthly rewards distributed"
    return

# Distribute 300 tokens as rewards
push 300.0
call distribute_rewards
``` 