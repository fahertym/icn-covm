// ICN Cooperative Virtual Machine - Identity Registration Demo
// This demo shows how a member registers and creates their identity

// Define member information
define MEMBER_ID "alice_cooper"
define COOP_ID "musicians_coop"
define PUBLIC_KEY "ed25519:dGhpcyBpcyBhIHNhbXBsZSBwdWJsaWMga2V5" // Base64 encoded

// Start the registration process
emit "Starting identity registration for member " + MEMBER_ID + " in cooperative " + COOP_ID

// Step 1: Create the core identity
call create_identity(MEMBER_ID, "member", PUBLIC_KEY, "ed25519") -> identity_result
if identity_result.success:
    emit "Identity created: " + identity_result.id
    emit "✅ Step 1: Core identity created"
else:
    emit "❌ Failed to create identity: " + identity_result.error
    exit

// Step 2: Add metadata to the identity
call add_identity_metadata(MEMBER_ID, "coop_id", COOP_ID) -> metadata_result
call add_identity_metadata(MEMBER_ID, "display_name", "Alice Cooper") -> _
call add_identity_metadata(MEMBER_ID, "email", "alice@example.com") -> _
call add_identity_metadata(MEMBER_ID, "joined_date", "2023-04-15") -> _

if metadata_result.success:
    emit "✅ Step 2: Identity metadata added successfully"
else:
    emit "❌ Failed to add metadata: " + metadata_result.error
    exit

// Step 3: Create member profile
define JOIN_DATE 1681516800  // Unix timestamp for 2023-04-15
call create_member_profile(MEMBER_ID, JOIN_DATE) -> profile_result
if profile_result.success:
    emit "✅ Step 3: Member profile created"
else:
    emit "❌ Failed to create member profile: " + profile_result.error
    exit

// Step 4: Add roles to the member profile
call add_member_role(MEMBER_ID, "voter") -> role_result
call add_member_role(MEMBER_ID, "contributor") -> _

if role_result.success:
    emit "✅ Step 4: Member roles added"
else:
    emit "❌ Failed to add roles: " + role_result.error
    exit

// Step 5: Store credential issued by the cooperative
define CRED_ID MEMBER_ID + "_membership"
define CURRENT_TIME now()
define EXPIRY_TIME CURRENT_TIME + 31536000  // 1 year later

call create_credential(
    CRED_ID,                 // Credential ID
    "membership",            // Type
    COOP_ID,                 // Issuer
    MEMBER_ID,               // Holder
    CURRENT_TIME,            // Issued at
    EXPIRY_TIME,             // Expires at
    "This credential certifies membership in the Musicians Cooperative"  // Description
) -> credential_result

if credential_result.success:
    emit "✅ Step 5: Membership credential issued"
    emit "✓ Credential ID: " + CRED_ID
    emit "✓ Issued by: " + COOP_ID
    emit "✓ Valid until: " + EXPIRY_TIME
else:
    emit "❌ Failed to issue credential: " + credential_result.error
    exit

// Step 6: Sign the credential (in a real system, this would be done via crypto signing)
call sign_credential(CRED_ID, "SAMPLE_SIGNATURE_FROM_COOPERATIVE") -> sign_result
if sign_result.success:
    emit "✅ Step 6: Credential signed by cooperative"
else:
    emit "❌ Failed to sign credential: " + sign_result.error
    exit

// Step 7: Store the identity in persistent storage
call store_identity(MEMBER_ID) -> storage_result
if storage_result.success:
    emit "✅ Step 7: Identity stored in persistent storage"
    emit "👤 Identity namespace: " + storage_result.namespace
else:
    emit "❌ Failed to store identity: " + storage_result.error
    exit

// Registration complete
emit "✨ Identity registration complete for " + MEMBER_ID
emit "Member is now registered with the " + COOP_ID + " cooperative"
emit "Member can now participate in cooperative governance activities" 