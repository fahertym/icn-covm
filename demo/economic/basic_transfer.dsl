# Economic Operations Demo
# This program demonstrates the economic operations: mint, transfer, and burn

# First, we need to set up our resource and accounts
# Normally, these would be created through other means (admin UI, etc.)
# For this demo, we'll assume that:
# - The resource 'community_coin' already exists in the system
# - The identity executing this has permission to mint
# - Account IDs 'treasury', 'alice', and 'bob' represent different identities

# Mint initial tokens to the treasury
mint community_coin treasury 1000.0 "Initial coin allocation for community treasury"
emitevent "economic" "Minted 1000 coins to treasury"

# Check the treasury balance (should store 1000.0 in memory)
storep "balances/treasury/community_coin"
loadp "balances/treasury/community_coin"
store treasury_balance
dumpstack

# Transfer some coins from treasury to Alice
transfer community_coin treasury alice 250.0 "Monthly community contribution"
emitevent "economic" "Transferred 250 coins from treasury to Alice"

# Transfer some coins from treasury to Bob
transfer community_coin treasury bob 150.0 "Project bounty reward"
emitevent "economic" "Transferred 150 coins from treasury to Bob"

# Alice sends some coins to Bob
transfer community_coin alice bob 75.0 "Payment for design work"
emitevent "economic" "Transferred 75 coins from Alice to Bob"

# Check all balances
loadp "balances/treasury/community_coin"
store treasury_balance
loadp "balances/alice/community_coin"
store alice_balance
loadp "balances/bob/community_coin" 
store bob_balance

# Display current balances
emit "Current Treasury Balance: "
load treasury_balance
emit "Current Alice Balance: "
load alice_balance
emit "Current Bob Balance: "
load bob_balance

# Bob burns some tokens (e.g., redeeming them for a service)
burn community_coin bob 50.0 "Redeemed for community workshop"
emitevent "economic" "Bob burned 50 coins for community workshop"

# Final balances
loadp "balances/treasury/community_coin"
store treasury_balance
loadp "balances/alice/community_coin"
store alice_balance
loadp "balances/bob/community_coin" 
store bob_balance

# Display final balances
emit "Final Treasury Balance: "
load treasury_balance
emit "Final Alice Balance: "
load alice_balance
emit "Final Bob Balance: "
load bob_balance

# Expected totals:
# Treasury: 1000 - 250 - 150 = 600
# Alice: 250 - 75 = 175
# Bob: 150 + 75 - 50 = 175
# Total in circulation: 950 (1000 minted - 50 burned) 