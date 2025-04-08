#######################################################################
# IMPORTANT: This demo requires the typed-values feature to function.
# Run with: cargo run --features typed-values -- run --program demo/identity/identity_registration.dsl
#
# This demo showcases some planned features that are not yet fully supported
# in the base DSL without the typed-values feature flag.
#######################################################################

# ICN Cooperative Virtual Machine - Identity Registration Demo
# This demo shows how a member registers and creates their identity

# Define member information (using variables instead of define)
push "alice_cooper"
store MEMBER_ID

push "musicians_coop"
store COOP_ID

push "ed25519:dGhpcyBpcyBhIHNhbXBsZSBwdWJsaWMga2V5" # Base64 encoded
store PUBLIC_KEY

# Start the registration process
load MEMBER_ID
push " in cooperative "
load COOP_ID
push "Starting identity registration for member "
emit "Starting identity registration for member " + MEMBER_ID + " in cooperative " + COOP_ID

# Step 1: Create the core identity
load MEMBER_ID
push "member"
load PUBLIC_KEY
push "ed25519"
call create_identity -> identity_result

load identity_result
push "success"
eq

if:
    load identity_result
    push "id"
    call get_field
    push "Identity created: "
    emit "Identity created: " + identity_result.id
    emit "✅ Step 1: Core identity created"
else:
    load identity_result
    push "error"
    call get_field
    push "❌ Failed to create identity: "
    emit "❌ Failed to create identity: " + identity_result.error
    exit

# Step 2: Add metadata to the identity
load MEMBER_ID
push "coop_id"
load COOP_ID
call add_identity_metadata -> metadata_result

load MEMBER_ID
push "display_name"
push "Alice Cooper"
call add_identity_metadata

load MEMBER_ID
push "email"
push "alice@example.com"
call add_identity_metadata

load MEMBER_ID
push "joined_date"
push "2023-04-15"
call add_identity_metadata

load metadata_result
push "success"
eq

if:
    emit "✅ Step 2: Identity metadata added successfully"
else:
    load metadata_result
    push "error"
    call get_field
    push "❌ Failed to add metadata: "
    emit "❌ Failed to add metadata: " + metadata_result.error
    exit

# Step 3: Create member profile
push 1681516800  # Unix timestamp for 2023-04-15
store JOIN_DATE

load MEMBER_ID
load JOIN_DATE
call create_member_profile -> profile_result

load profile_result
push "success"
eq

if:
    emit "✅ Step 3: Member profile created"
else:
    load profile_result
    push "error"
    call get_field
    push "❌ Failed to create member profile: "
    emit "❌ Failed to create member profile: " + profile_result.error
    exit

# Step 4: Add roles to the member profile
load MEMBER_ID
push "voter"
call add_member_role -> role_result

load MEMBER_ID
push "contributor"
call add_member_role

load role_result
push "success"
eq

if:
    emit "✅ Step 4: Member roles added"
else:
    load role_result
    push "error"
    call get_field
    push "❌ Failed to add roles: "
    emit "❌ Failed to add roles: " + role_result.error
    exit

# Step 5: Store credential issued by the cooperative
load MEMBER_ID
push "_membership"
push "+"
call string_concat
store CRED_ID

call now
store CURRENT_TIME

load CURRENT_TIME
push 31536000  # 1 year later
add
store EXPIRY_TIME

load CRED_ID
push "membership"
load COOP_ID
load MEMBER_ID
load CURRENT_TIME
load EXPIRY_TIME
push "This credential certifies membership in the Musicians Cooperative"
call create_credential -> credential_result

load credential_result
push "success"
eq

if:
    emit "✅ Step 5: Membership credential issued"
    push "✓ Credential ID: "
    load CRED_ID
    emit "✓ Credential ID: " + CRED_ID
    push "✓ Issued by: "
    load COOP_ID
    emit "✓ Issued by: " + COOP_ID
    push "✓ Valid until: "
    load EXPIRY_TIME
    emit "✓ Valid until: " + EXPIRY_TIME
else:
    load credential_result
    push "error"
    call get_field
    push "❌ Failed to issue credential: "
    emit "❌ Failed to issue credential: " + credential_result.error
    exit

# Step 6: Sign the credential (in a real system, this would be done via crypto signing)
load CRED_ID
push "SAMPLE_SIGNATURE_FROM_COOPERATIVE"
call sign_credential -> sign_result

load sign_result
push "success"
eq

if:
    emit "✅ Step 6: Credential signed by cooperative"
else:
    load sign_result
    push "error"
    call get_field
    push "❌ Failed to sign credential: "
    emit "❌ Failed to sign credential: " + sign_result.error
    exit

# Step 7: Store the identity in persistent storage
load MEMBER_ID
call store_identity -> storage_result

load storage_result
push "success"
eq

if:
    emit "✅ Step 7: Identity stored in persistent storage"
    push "👤 Identity namespace: "
    load storage_result
    push "namespace"
    call get_field
    emit "👤 Identity namespace: " + storage_result.namespace
else:
    load storage_result
    push "error"
    call get_field
    push "❌ Failed to store identity: "
    emit "❌ Failed to store identity: " + storage_result.error
    exit

# Registration complete
push "✨ Identity registration complete for "
load MEMBER_ID
emit "✨ Identity registration complete for " + MEMBER_ID

push "Member is now registered with the "
load COOP_ID
push " cooperative"
emit "Member is now registered with the " + COOP_ID + " cooperative"

emit "Member can now participate in cooperative governance activities" 