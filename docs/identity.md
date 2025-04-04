# Identity System Documentation

## Overview

The ICN-COVM Identity System provides secure authentication and authorization for VM operations. It enables role-based access control, cryptographic verification, and delegation chains for cooperative governance.

This document describes the identity system architecture, key components, and integration with other VM systems.

## Core Components

### Identity Structure

The `Identity` structure represents a participant in the system:

```rust
pub struct Identity {
    /// Unique identifier for the identity
    pub id: String,
    
    /// Public key for cryptographic verification (optional)
    pub public_key: Option<Vec<u8>>,
    
    /// Type of identity (e.g., "cooperative", "member", "service")
    pub identity_type: String,
    
    /// Cryptographic scheme used (e.g., "ed25519", "secp256k1")
    pub crypto_scheme: Option<String>,
    
    /// Additional metadata about this identity
    pub metadata: HashMap<String, String>,
    
    /// Version information tracking this identity's history
    pub version_info: Option<VersionInfo>,
}
```

### Authentication Context

The `AuthContext` provides the authentication and authorization context for VM operations:

```rust
pub struct AuthContext {
    pub user_id: String,
    
    /// Namespace -> Roles mapping
    roles: HashMap<String, Vec<String>>,
    
    /// Delegate ID -> Delegator ID mapping
    delegations: HashMap<String, String>,
    
    /// The current identity being used for operations
    pub current_identity: Option<Identity>,
    
    /// Registry of known identities
    pub identity_registry: Option<HashMap<String, Identity>>,
    
    /// Registry of known delegations
    pub delegation_registry: Option<HashMap<String, DelegationLink>>,
    
    /// Registry of member profiles
    pub member_registry: Option<HashMap<String, MemberProfile>>,
    
    /// Registry of credentials
    pub credential_registry: Option<HashMap<String, Credential>>,
    
    /// The cooperative ID context for execution
    pub executing_cooperative_id: Option<String>,
}
```

### Member Profile

Extends an identity with member-specific information:

```rust
pub struct MemberProfile {
    /// The core identity this profile is associated with
    pub identity: Identity,
    
    /// Member-specific roles within their cooperative
    pub roles: Vec<String>,
    
    /// Reputation score (if used by the cooperative)
    pub reputation: Option<f64>,
    
    /// Joined timestamp
    pub joined_at: u64,
    
    /// Additional profile attributes
    pub attributes: HashMap<String, String>,
    
    /// Version information for this profile
    pub version_info: Option<VersionInfo>,
}
```

### Credential

Represents a verifiable credential:

```rust
pub struct Credential {
    /// Unique identifier for this credential
    pub id: String,
    
    /// Type of credential (e.g., "membership", "voting_right", "admin_access")
    pub credential_type: String,
    
    /// Identity ID that issued this credential
    pub issuer_id: String,
    
    /// Identity ID that holds this credential
    pub holder_id: String,
    
    /// Timestamp when issued
    pub issued_at: u64,
    
    /// Optional expiration timestamp
    pub expires_at: Option<u64>,
    
    /// Cryptographic signature from the issuer
    pub signature: Option<Vec<u8>>,
    
    /// Claims associated with this credential
    pub claims: HashMap<String, String>,
    
    /// Version information for this credential
    pub version_info: Option<VersionInfo>,
}
```

### Delegation Link

Represents a delegation of authority:

```rust
pub struct DelegationLink {
    /// Unique identifier for this delegation
    pub id: String,
    
    /// Identity ID of the delegator
    pub delegator_id: String,
    
    /// Identity ID of the delegate
    pub delegate_id: String,
    
    /// Type of delegation (e.g., "voting", "admin", "full")
    pub delegation_type: String,
    
    /// Permissions granted through this delegation
    pub permissions: Vec<String>,
    
    /// When the delegation was created
    pub created_at: u64,
    
    /// When the delegation expires (if temporary)
    pub expires_at: Option<u64>,
    
    /// Cryptographic signature from the delegator
    pub signature: Option<Vec<u8>>,
    
    /// Additional attributes for this delegation
    pub attributes: HashMap<String, String>,
    
    /// Version information for this delegation
    pub version_info: Option<VersionInfo>,
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

# Verify an identity's signature 
verifyidentity "member1" "message" "signature"
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

### Integration with Storage

The identity system integrates with the storage system to enable:

1. **Permission Checking**: The VM uses `AuthContext` as the active permission context during execution.
2. **Identity Enforcement**: Identity and role checks are enforced at the point of sensitive operations (e.g., persistent storage, governance actions).
3. **Control Flow**: DSL identity operations influence control flow but not data injection—no identity data is directly injected into the stack unless explicitly stored using `getcaller`.

Example of identity integration with storage:

```
# Store data in an identity-specific namespace
begintx
    # Verify the caller has the required role
    requirerole "member"
    
    # Get the current caller ID
    getcaller
    store "user_id"
    
    # Create a user-specific storage key
    push "users/"
    load "user_id"
    concat
    push "/profile/balance"
    concat
    store "user_key"
    
    # Store data at the user-specific key
    push 100.0
    load "user_key"
    storep
committx
```

## Testing and Mocking

The identity system includes testing utilities for identity-based systems:

```rust
// Create a test authentication context
let mut auth = AuthContext::new("user1");

// Add roles to the user
auth.add_role("default", "admin");
auth.add_role("coop/test_coop", "member");

// Create and register an identity
let mut identity = Identity::new("user1", "member");
identity.add_metadata("coop_id", "test_coop");
auth.register_identity(identity);

// Set up the VM with this auth context
let mut vm = VM::new();
vm.set_auth_context(auth);

// Now VM operations will be executed in this identity context
```

## Security Considerations

1. **Cryptographic Verification**: Always verify signatures for security-critical operations
2. **Role Assignment**: Carefully manage role assignments, especially for administrative access
3. **Delegation Chains**: Watch for delegation cycles or excessive chain length
4. **Expiration**: Consider expiring credentials and delegations to limit their lifetime

## Best Practices

1. **Namespaced Roles**: Use namespace-prefixed roles like `coops/example_coop/admin`
2. **Least Privilege**: Assign the minimum necessary roles for each identity
3. **Audit Trail**: Use the audit logs to track identity-based operations
4. **Verification**: Always verify credentials cryptographically in production systems

## Future Extensions

1. **Multi-signature support**: Require multiple identities to approve operations
2. **Credential revocation**: Support for revoking credentials
3. **Identity recovery**: Methods for recovering lost identities
4. **Decentralized identifiers**: Support for DIDs and verifiable credentials
