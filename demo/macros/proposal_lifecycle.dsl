# Demo of the proposal_lifecycle macro
# This shows a simple proposal that allocates funds from the treasury

proposal_lifecycle "prop-001" quorum=0.6 threshold=0.5 {
    emit "Executing proposal prop-001..."
    mint community_coin "project_fund" 1000.0 "Allocated from treasury"
}

# Another example with different parameters
proposal_lifecycle "prop-002" quorum=0.75 threshold=0.66 {
    emit "Executing proposal prop-002..."
    transfer "treasury" "development_fund" 500.0 "Funding development initiatives"
} 