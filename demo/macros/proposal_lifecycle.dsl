# Demo of the proposal_lifecycle macro
# This shows proposals with metadata and different parameters

# Basic proposal with default metadata
proposal_lifecycle "prop-001" quorum=0.6 threshold=0.5 {
    emit "Executing proposal prop-001..."
    mint community_coin "project_fund" 1000.0 "Allocated from treasury"
}

# Proposal with custom metadata
proposal_lifecycle "prop-002" quorum=0.75 threshold=0.66 title="Development Fund Allocation" author="alice" {
    emit "Executing proposal prop-002..."
    transfer "treasury" "development_fund" 500.0 "Funding development initiatives"
}

# Proposal with all parameters
proposal_lifecycle "prop-003" quorum=0.8 threshold=0.7 title="Community Grant Program" author="bob" {
    emit "Executing proposal prop-003..."
    transfer "treasury" "community_grants" 2000.0 "Funding community projects"
    emit "Grant program initialized"
}

# Proposal with conditional execution
proposal_lifecycle "prop-004" quorum=0.6 threshold=0.75 title="Repair Budget" author="matt" {
    emit "Executing proposal prop-004..."
    if passed:
        emit "Proposal passed — allocating repair funds"
        transfer "treasury" "repair_fund" 1000.0 "Allocated for repairs"
    else:
        emit "Proposal failed — no funds allocated"
} 