# Q3 Budget Proposal Logic
# This is a test DSL file for the proposal create CLI command

# Define budget allocation
set_budget 15000 token="ICN"
allocate 5000 category="Marketing" recipient="marketing-team"
allocate 7000 category="Development" recipient="dev-team"
allocate 3000 category="Community" recipient="community-fund"

if_passed {
  # Execute budget allocation when proposal passes
  transfer 5000 from="treasury" to="marketing-team" token="ICN" memo="Q3 Marketing Budget"
  transfer 7000 from="treasury" to="dev-team" token="ICN" memo="Q3 Development Budget"
  transfer 3000 from="treasury" to="community-fund" token="ICN" memo="Q3 Community Budget"
  
  # Log the execution
  log "Q3 budget proposal executed successfully"
} else {
  # Log if the proposal fails
  log "Q3 budget proposal was rejected"
} 