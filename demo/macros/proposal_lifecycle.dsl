# Demo of the proposal_lifecycle macro
# This shows proposals with metadata and different parameters

# Proposal Lifecycle Macro Examples
#
# CLI Equivalents:
# icn-covm proposal create --id prop-004 --title "Repair Budget" --author "matt"
# icn-covm proposal attach --id prop-004 --section summary --text "Fix the roof on 14B"
# icn-covm proposal vote --id prop-004 --ranked 1 2 3

# Example 1: Basic proposal with defaults
proposal_lifecycle "prop-001" {
    emit "Basic proposal created"
}

# Example 2: Proposal with custom thresholds
proposal_lifecycle "prop-002" quorum=0.75 threshold=0.6 {
    emit "Proposal with custom thresholds created"
}

# Example 3: Full proposal with metadata
proposal_lifecycle "prop-003" quorum=0.8 threshold=0.7 title="Important Decision" author="alice" {
    emit "Full proposal created"
    storep "proposals/prop-003/docs/summary" "This is a critical decision that needs attention"
}

# Example 4: Proposal with multiple document sections
proposal_lifecycle "prop-004" title="Repair Budget" author="matt" {
    emit "Repair budget proposal created"
    storep "proposals/prop-004/docs/summary" "Fix the roof on building 14B"
    storep "proposals/prop-004/docs/rationale" "The roof is leaking and needs immediate repair"
    storep "proposals/prop-004/docs/budget" "Estimated cost: $5000"
}

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