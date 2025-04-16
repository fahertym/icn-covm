# Test demo of proposal lifecycle with concrete example
# This example creates a proposal for allocating funds to a project

proposal_lifecycle "example-proposal" quorum=0.5 threshold=0.6 title="Fund Allocation" author="alice" {
    emit "Executing Fund Allocation proposal..."
    
    # Transfer funds from the treasury to the project account
    transfer "treasury" "project_fund" 1000.0 "Funding allocation"
    
    # Record the transfer in the project ledger
    storep "project/ledger/example-proposal" "Funded: 1000.0"
    
    emit "Project funding complete"
    
    # Return success value for testing
    push 1.0
} 