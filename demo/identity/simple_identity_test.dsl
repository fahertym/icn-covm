# Simple Identity and Storage Integration Test
# This test demonstrates basic identity and storage integration

# 1. Set up a basic storage value
emit "1. Setting up storage value with user ID 123..."
push 123.0
storep "user_data"
emit "Stored user ID value: 123"

# 2. Read it back
emit "2. Loading user data from storage..."
loadp "user_data"
emit "Loaded user ID:"
dumpstack

# 3. Store a versioned value
emit "3. Storing multiple versions of profile data..."
push 1.0
storep "profile_version"
emit "Stored profile version 1.0"

push 2.0
storep "profile_version"
emit "Updated profile version to 2.0"

push 3.0
storep "profile_version"
emit "Updated profile version to 3.0"

# 4. Read the latest version
emit "4. Reading latest version of profile..."
loadp "profile_version"
emit "Latest profile version:"
dumpstack

emit "Simple identity and storage test completed." 