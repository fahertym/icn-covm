# ICN Cooperative Governance Demo
# This demonstrates how to use the persistent storage system
# for cooperative governance operations with proper namespaces
# and role-based access control.

# Set up a test proposal in cooperative governance
# Format: governance/proposals/PROPOSAL_ID

# Create a new proposal with ID "prop-001"
# First, push the proposal data
push 1.0                # Proposal version
push 1234567890.0       # Creation timestamp
push "sustainable_energy_initiative"  # Title (simplified as stack can only hold numbers)
push 7.0                # Number of days the proposal is active
# Store the proposal in the governance namespace
storep "governance/proposals/prop-001"

# Log the action
emit "Created new proposal: prop-001 (Sustainable Energy Initiative)"

# Check proposal quorum requirements (e.g., 50% participation)
push 100.0  # Total possible votes
push 52.0   # Current vote count
quorumthreshold 0.5  # 50% threshold

# If the quorum threshold is met (result is 0.0 for true)
if
  emit "Quorum threshold met, proposal can proceed"
else
  emit "Quorum threshold not met, need more votes"
endif

# Cast a vote for the proposal
# Format: governance/votes/PROPOSAL_ID/VOTER_ID
push 1.0       # Vote: 1.0 = approve, 0.0 = abstain, -1.0 = reject
storep "governance/votes/prop-001/member001"

emit "Vote recorded for member001 on proposal prop-001"

# Check if the proposal has reached the required approval threshold
# (e.g., 66% approval)
push 70.0    # Approval votes
votethreshold 66.0  # 66% threshold

# If the vote threshold is met (result is 0.0 for true)
if
  # Proposal approved
  emit "Proposal approved! Executing actions..."
  
  # Store the approval status
  push 1.0  # 1.0 = approved
  storep "governance/proposals/prop-001/status"
  
  # Record approval timestamp
  push 1234569000.0  # Approval timestamp
  storep "governance/proposals/prop-001/approved_at"
else
  # Proposal rejected
  emit "Proposal does not have enough votes for approval yet"
endif

# Demonstrate loading values from the governance namespace
# Load the proposal status
loadp "governance/proposals/prop-001/status"

# Display the result
emit "Proposal status (1.0=approved, 0.0=pending, -1.0=rejected):"

# Demonstrate the liquid democracy delegation feature
# Format: governance/delegations/FROM/TO
push 1.0  # Delegation recorded (value doesn't matter, key is what counts)
storep "governance/delegations/member002/member001"

emit "Delegation recorded: member002 delegates to member001"

# Show all members who delegated to member001
# This would be done with list_keys in the VM, we simulate with a message
emit "Members who delegated to member001: member002"

# Demo accounting for resource usage
push 1234.0  # Amount of resources used
storep "governance/resource_usage/member001"

emit "Resource usage recorded for member001"

# Final status message
emit "Governance operations completed successfully!" 