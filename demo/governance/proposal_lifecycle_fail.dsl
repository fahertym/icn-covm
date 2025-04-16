# Test demo of proposal lifecycle that will fail quorum
# This example creates a proposal with a high quorum requirement that will fail

proposal_lifecycle "low-turnout" quorum=0.8 threshold=0.5 title="High Quorum Proposal" author="bob" {
    emit "This proposal requires high participation to pass"
    
    # This code should never execute if quorum isn't met
    emit "WARNING: Executing despite failing quorum!"
    
    # Return failure signal for testing
    push 0.0
} 