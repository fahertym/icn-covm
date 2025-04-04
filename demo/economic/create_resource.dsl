# Economic Resource Creation Demo
# This program demonstrates how to create a new economic resource and use it

# Define resource parameters
# In a real implementation, these would be parameters to a resource creation operation
# For now, we'll use memory variables and manual creation through the storage API

# Resource metadata
store "community_token" resource_id
store "Community Token" resource_name
store "currency" resource_type
store "coop:test_community" resource_issuer
store "A token for community governance and resource sharing" resource_description
store "COMM" resource_symbol

# Create the resource JSON (simplified for demo)
storep "resources/community_token" {
  "id": "community_token",
  "name": "Community Token",
  "description": "A token for community governance and resource sharing",
  "resource_type": "currency",
  "issuer_namespace": "coop:test_community",
  "created_at": 1618531200000,
  "metadata": {
    "symbol": "COMM",
    "decimals": "2"
  },
  "transferable": true,
  "divisible": true
}

emitevent "economic" "Created new economic resource: Community Token (COMM)"

# Set up some accounts
store "founder1" account1
store "founder2" account2
store "community_fund" account3

# Initial allocation - mint tokens to founders and community fund
mint community_token founder1 1000.0 "Founder allocation"
mint community_token founder2 1000.0 "Founder allocation"
mint community_token community_fund 8000.0 "Community fund initial allocation"

# Check initial balances
loadp "balances/founder1/community_token"
store founder1_balance
loadp "balances/founder2/community_token"
store founder2_balance
loadp "balances/community_fund/community_token"
store fund_balance

# Display initial allocations
emit "Initial token allocation:"
emit "Founder 1: "
load founder1_balance
emit "Founder 2: "
load founder2_balance
emit "Community Fund: "
load fund_balance

# Calculate total supply
load founder1_balance
load founder2_balance
add
load fund_balance
add
store total_supply

emit "Total supply: "
load total_supply

# Demonstrate a community decision to allocate resources
# The community fund transfers tokens to a project
store "project_team" recipient
transfer community_token community_fund project_team 500.0 "Funding for Project Alpha"

# Check final state
loadp "balances/community_fund/community_token"
emit "Community Fund (after project funding): "
loadp "balances/project_team/community_token"
emit "Project Team: "

emitevent "economic" "Completed resource creation and initial allocation demo" 