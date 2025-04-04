# Identity System Documentation

## Overview

The ICN-COVM Identity System provides secure authentication and authorization for VM operations. It enables role-based access control, cryptographic verification, and delegation chains for cooperative governance.

This document describes the identity system architecture, key components, and integration with other VM systems.

## Core Components

### Identity Structure

The `Identity` structure represents a participant in the system:

```rust
pub struct Identity {
    // Unique identifier for this identity
    pub id: String,
    
    // Optional public key for cryptographic verification
    pub public_key: Option<Vec<u8>>,
    
    // Cryptographic scheme used (e.g., "ed25519")
    pub crypto_scheme: Option<String>,
    
    // Metadata associated with this identity
    pub metadata: HashMap<String, String>,
    
    // Roles assigned to this identity
    pub roles: Vec<String>,
}
```

### Authentication Context

The `AuthContext` provides the authentication and authorization context for VM operations:

```rust
pub struct AuthContext {
    // The current caller identity
    pub caller: Option<Identity>,
    
    // Registry of identities in the current context
    pub identity_registry: HashMap<String, Identity>,
    
    // Role assignments for namespaces
    pub roles: HashMap<String, HashSet<String>>,
    
    // Member profiles
    pub members: HashMap<String, MemberProfile>,
    
    // Credentials in this context
    pub credentials: HashMap<String, Credential>,
    
    // Delegations in this context
    pub delegations: HashMap<String, DelegationLink>,
}
```

### Member Profile

Extends an identity with member-specific information:

```rust
pub struct MemberProfile {
    // Core identity information
    pub identity: Identity,
    
    // When the member joined
    pub joined_at: i64,
    
    // Member-specific roles
    pub roles: Vec<String>,
    
    // Member-specific metadata
    pub profile: HashMap<String, String>,
}
```

### Credential

Represents a verifiable credential:

```rust
pub struct Credential {
    // Unique identifier for this credential
    pub id: String,
    
    // Type of credential (e.g., "membership")
    pub credential_type: String,
    
    // Identity that issued this credential
    pub issuer: String,
    
    // Identity that holds this credential
    pub holder: String,
    
    // When this credential was issued
    pub issued_at: i64,
    
    // When this credential expires (if applicable)
    pub expires_at: Option<i64>,
    
    // Claims made in this credential
    pub claims: HashMap<String, String>,
    
    // Cryptographic signature (if verified)
    pub signature: Option<Vec<u8>>,
}
```

### Delegation Link

Represents a delegation of authority:

```rust
pub struct DelegationLink {
    // Unique identifier for this delegation
    pub id: String,
    
    // Identity delegating authority
    pub delegator: String,
    
    // Identity receiving authority
    pub delegate: String,
    
    // Type of delegation (e.g., "voting")
    pub delegation_type: String,
    
    // When this delegation was created
    pub created_at: i64,
    
    // When this delegation expires (if applicable)
    pub expires_at: Option<i64>,
    
    // Permissions granted by this delegation
    pub permissions: Vec<String>,
    
    // Cryptographic signature (if verified)
    pub signature: Option<Vec<u8>>,
}
```

## VM Operations

The identity system adds several operations to the VM:

### Identity Verification

```
# Get the current caller's identity
getcaller

# Check if the caller has a specific role
hasrole "admin"

# Require a specific role (aborts if not present)
requirerole "treasurer"

# Verify that the caller is a specific identity
requireidentity "member1"

# Verify a cryptographic signature
verifysignature
```

### Membership Operations

```
# Check if an identity is a member of a namespace
checkmembership "member1" "coops/example_coop"

# Check if a delegation exists
checkdelegation "member1" "member2"
```

## DSL Examples

### Basic Authentication

```
# Get the current caller and store it
getcaller
store "current_user"

# Check if caller has admin role
hasrole "admin"
if:
    emit "Welcome, admin!"
else:
    emit "Access denied."
endif
```

### Role-Based Access Control

```
# Try to access protected resource
begintx
    # This will abort the transaction if the caller doesn't have the role
    requirerole "treasurer"
    
    # Perform protected operation
    push 100.0
    storep "org/treasury/balance"
    
    emit "Treasury updated"
committx

# Handle aborted transactions gracefully
onerror:
    emit "Permission denied: treasurer role required"
enderr
```

### Delegation Verification

```
# Check if member2 has delegated to member1
checkdelegation "member2" "member1"
if:
    # Perform delegated action
    push "Voting on behalf of member2"
    storep "votes/proposal1/member2"
else:
    emit "No delegation found"
endif
```

## Integration with Storage

The identity system integrates with the storage system to enable:

1. **Permission Checking**: Storage operations verify permissions based on identity and roles
2. **Persistent Identity Data**: Store identity information in persistent storage
3. **Namespace Isolation**: Use identity-specific namespaces for data isolation

Example:

```
# Store data in an identity-specific namespace
begintx
    requirerole "member"
    getcaller
    store "user"
    
    push 100
    load "user"
    concat "/profile/balance"
    storep
committx
```

## Testing and Mocking

The identity system includes testing utilities:

1. **Mock Identities**: Create identities for testing
2. **Mock Signatures**: Test without real cryptography
3. **Role Assignment**: Quickly assign roles for testing

Example test setup:

```rust
// Create a test auth context
let mut auth = AuthContext::new();

// Add an identity
let mut identity = Identity::new("user1", "test");
identity.add_role("admin");
auth.register_identity(identity);

// Set as caller
auth.set_caller("user1");

// Use in VM
let mut vm = VM::new();
vm.set_auth_context(auth);
```

## Security Considerations

1. **Cryptographic Verification**: Always verify signatures for security-critical operations
2. **Role Assignment**: Carefully manage role assignments
3. **Delegation Chains**: Watch for delegation cycles or excessive chain length
4. **Expiration**: Consider expiring credentials and delegations

## Best Practices

1. **Namespaced Roles**: Use namespace-prefixed roles like `coops/example_coop/admin`
2. **Least Privilege**: Assign the minimum necessary roles
3. **Audit Trail**: Log identity information for all operations
4. **Verification**: Always verify credentials cryptographically in production

## Future Extensions

1. **Multi-signature support**: Require multiple identities to approve operations
2. **Credential revocation**: Support for revoking credentials
3. **Identity recovery**: Methods for recovering lost identities
4. **Decentralized identifiers**: Support for DIDs and verifiable credentials
