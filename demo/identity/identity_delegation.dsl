///////////////////////////////////////////////////////////////////////
// IMPORTANT: This demo requires the typed-values feature to function.
// Run with: cargo run --features typed-values -- run --program demo/identity/identity_delegation.dsl
//
// This demo showcases some planned features that are not yet fully supported
// in the base DSL without the typed-values feature flag.
///////////////////////////////////////////////////////////////////////

// ICN Cooperative Virtual Machine - Identity Delegation Demo
// This demo shows how a member delegates authority to another member

// Define member information
define DELEGATOR_ID "alice_cooper"
define DELEGATE_ID "bob_dylan"
define COOP_ID "musicians_coop"
define DELEGATION_ID "delegation_" + DELEGATOR_ID + "_to_" + DELEGATE_ID
define DELEGATION_TYPE "voting"

// Start the delegation process
emit "Starting delegation process: " + DELEGATOR_ID + " delegates to " + DELEGATE_ID

// Step 1: Verify both identities exist
call verify_identity_exists(DELEGATOR_ID) -> delegator_check
call verify_identity_exists(DELEGATE_ID) -> delegate_check

if delegator_check.exists and delegate_check.exists:
    emit "✅ Step 1: Both delegator and delegate identities verified"
else:
    if not delegator_check.exists:
        emit "❌ Delegator identity not found: " + DELEGATOR_ID
    if not delegate_check.exists:
        emit "❌ Delegate identity not found: " + DELEGATE_ID
    exit

// Step 2: Check delegator authorization (ensure they can delegate)
call check_delegation_authority(DELEGATOR_ID, DELEGATION_TYPE) -> auth_check
if auth_check.authorized:
    emit "✅ Step 2: Delegator has authority to delegate " + DELEGATION_TYPE + " rights"
else:
    emit "❌ Delegator lacks authority to delegate: " + auth_check.reason
    exit

// Step 3: Create delegation with current timestamp
define CURRENT_TIME now()
define EXPIRY_TIME CURRENT_TIME + 2592000  // 30 days later

call create_delegation(
    DELEGATION_ID,        // Delegation ID
    DELEGATOR_ID,         // Delegator
    DELEGATE_ID,          // Delegate
    DELEGATION_TYPE,      // Type
    CURRENT_TIME,         // Created at
    EXPIRY_TIME           // Expires at
) -> delegation_result

if delegation_result.success:
    emit "✅ Step 3: Delegation created"
    emit "✓ Delegation ID: " + DELEGATION_ID
    emit "✓ Valid until: " + EXPIRY_TIME
else:
    emit "❌ Failed to create delegation: " + delegation_result.error
    exit

// Step 4: Add permissions to the delegation
call add_delegation_permission(DELEGATION_ID, "vote") -> perm_result
call add_delegation_permission(DELEGATION_ID, "propose") -> _
call add_delegation_permission(DELEGATION_ID, "comment") -> _

if perm_result.success:
    emit "✅ Step 4: Permissions added to delegation"
    emit "✓ Permissions: vote, propose, comment"
else:
    emit "❌ Failed to add permissions: " + perm_result.error
    exit

// Step 5: Add context information to the delegation
call add_delegation_attribute(DELEGATION_ID, "context", "quarterly_meeting") -> attr_result
call add_delegation_attribute(DELEGATION_ID, "reason", "Unable to attend in person") -> _
call add_delegation_attribute(DELEGATION_ID, "scope", "All agenda items") -> _

if attr_result.success:
    emit "✅ Step 5: Context attributes added to delegation"
else:
    emit "❌ Failed to add attributes: " + attr_result.error
    exit

// Step 6: Sign the delegation (in a real system, this would use cryptographic signatures)
define SIGNATURE "SAMPLE_SIGNATURE_FROM_DELEGATOR"  // This would be a real cryptographic signature
call sign_delegation(DELEGATION_ID, SIGNATURE) -> sign_result

if sign_result.success:
    emit "✅ Step 6: Delegation cryptographically signed by delegator"
else:
    emit "❌ Failed to sign delegation: " + sign_result.error
    exit

// Step 7: Store the delegation in persistent storage
call store_delegation(DELEGATION_ID) -> storage_result
if storage_result.success:
    emit "✅ Step 7: Delegation stored in persistent storage"
    emit "📜 Delegation namespace: " + storage_result.namespace
else:
    emit "❌ Failed to store delegation: " + storage_result.error
    exit

// Step 8: Notify the delegate
call notify_delegate(DELEGATE_ID, DELEGATION_ID) -> notify_result
if notify_result.success:
    emit "✅ Step 8: Delegate notified of new delegation"
else:
    emit "⚠️ Delegate notification failed, but delegation is still valid"

// Delegation complete
emit "✨ Delegation process complete"
emit DELEGATOR_ID + " has delegated " + DELEGATION_TYPE + " authority to " + DELEGATE_ID
emit "Delegation valid from " + CURRENT_TIME + " until " + EXPIRY_TIME
emit "Permissions delegated: vote, propose, comment" 