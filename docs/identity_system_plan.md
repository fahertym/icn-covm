# Identity System Implementation for ICN-COVM

## Overview

This document describes the implementation of the identity system in the ICN Cooperative Virtual Machine (ICN-COVM). The identity system is a critical component that enables secure authentication, authorization, and attribution of actions within cooperative governance systems.

## Design Goals

1. **Security**: Strong cryptographic identity verification
2. **Flexibility**: Support for different identity schemes and verification methods
3. **Simplicity**: Easy-to-use API for common identity operations
4. **Privacy**: Appropriate controls for sensitive identity information
5. **Compatibility**: Integration with existing governance primitives
6. **Auditability**: Clear attribution of actions to identities

## Core Components

### 1. Identity Structure

The core identity structure is defined as:

```rust
pub struct AuthContext {
    // Unique identifier for the user
    pub user_id: String,
    
    // Roles associated with this identity, organized by namespace
    pub roles: HashMap<String, HashSet<String>>,
}
```

This provides:
- Unique identification of users
- Namespace-aware role management
- Integration with storage operations

### 2. VM Integration

The VM includes authentication context:

```rust
pub struct VM {
    // Existing fields...
    memory: HashMap<String, f64>,
    
    // Identity and storage fields
    storage_backend: Option<Box<dyn StorageBackend>>,
    auth_context: AuthContext,
    namespace: String,
}
```

### 3. Identity Operations

The VM supports the following identity operations:

| Operation | Description |
|-----------|-------------|
| `GetCaller` | Push the current caller's ID onto the stack |
| `HasRole(role)` | Check if the caller has a specific role |
| `RequireRole(role)` | Abort if the caller lacks a specific role |
| `RequireIdentity(id)` | Abort if the caller isn't the specified identity |
| `VerifySignature` | Verify a cryptographic signature against a message |

## Implementation Details

### Role-Based Access Control

The identity system implements role-based access control with:

```rust
impl AuthContext {
    pub fn new(user_id: &str) -> Self { ... }
    
    pub fn add_role(&mut self, namespace: &str, role: &str) { ... }
    
    pub fn has_role(&self, namespace: &str, role: &str) -> bool { ... }
    
    pub fn require_role(&self, namespace: &str, role: &str) -> Result<(), String> { ... }
}
```

Roles are organized by namespace, allowing granular permissions:
- Global roles (e.g., "admin", "member")
- Namespace-specific roles (e.g., "writer" in "governance")

### Cryptographic Verification

The identity system includes signature verification:

```rust
pub fn verify_signature(
    public_key: &[u8], 
    message: &[u8], 
    signature: &[u8], 
    scheme: &str
) -> Result<bool, String> { ... }
```

Currently supported cryptographic schemes:
- **ed25519**: Edwards-curve Digital Signature Algorithm
- **secp256k1**: (Planned) Elliptic Curve Digital Signature Algorithm

### Storage Integration

All storage operations include identity context:

```rust
fn get(&self, auth: &AuthContext, namespace: &str, key: &str) -> StorageResult<Vec<u8>>;
fn set(&mut self, auth: &AuthContext, namespace: &str, key: &str, value: Vec<u8>) -> StorageResult<()>;
```

This enables:
- Permission checking before storage access
- Audit logging with identity attribution
- Resource accounting per identity

## Usage Examples

### Basic Identity Operations

```
# Get the current caller's ID
getcaller
emit  # Outputs the caller's ID

# Check for a role
hasrole "admin"
if:
    emit "User is an admin"
else:
    emit "User is not an admin"

# Require a role (aborts if not present)
requirerole "treasurer"
# Proceed with treasury operations
```

### Signature Verification

```
# Verify a signature
push "message to verify"
push "base64_encoded_signature"
push "public_key_in_base64"
push "ed25519"
verifysignature
if:
    emit "Signature valid!"
else:
    emit "Invalid signature"
```

### Integration with Storage

```
# Check permission before storage operation
hasrole "writer"
if:
    push 100
    storep "treasury/balance"
else:
    emit "Permission denied"
```

## Security Considerations

The identity system includes these security features:

1. **Role Verification**: All operations verify appropriate roles
2. **Cryptographic Signatures**: Secure verification of external signatures
3. **Audit Trails**: Clear attribution of all operations
4. **Namespaced Roles**: Granular permission control by namespace

## Future Extensions

1. **Delegation Chains**: Support for action delegation with provenance
2. **Enhanced Crypto Schemes**: Support for additional signature schemes
3. **Identity Metadata**: Additional identity attributes and verification
4. **Federation Support**: Cross-VM identity verification

## Next Steps

1. Define the core `Identity`