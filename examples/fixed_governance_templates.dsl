# Define governance templates for different types of decisions

# Standard governance template for regular decisions
template "standard" {
    quorumthreshold 0.5
    votethreshold 0.6
    mindeliberation 72h
    expiresin 14d
    require_role "member"
}

# Governance template for budget decisions
template "budget" {
    quorumthreshold 0.6
    votethreshold 0.7
    mindeliberation 96h
    expiresin 10d
    require_role "member"
    require_role "finance"
}

# Governance template for emergency decisions
template "emergency" {
    quorumthreshold 0.3
    votethreshold 0.8
    mindeliberation 1h
    expiresin 24h
    require_role "guardian"
}

# Function to demonstrate usage of different templates
def demonstrate_templates():
    # Store the original stack depth
    depth
    store depth_before
    
    # Use the standard template for a non-critical decision
    emit "Standard Decision Example"
    emit "=========================="
    governance use "standard"
    
    emit "This proposal will require:"
    emit "- 50% quorum"
    emit "- 60% approval"
    emit "- 72 hours deliberation"
    emit "- 14 days voting period"
    emit "- 'member' role to vote"
    emit ""
    
    # Use the budget template for a financial decision
    emit "Budget Decision Example"
    emit "======================="
    governance use "budget"
    
    # Override one parameter to demonstrate flexibility
    governance {
        mindeliberation 120h    # Extend deliberation for this specific budget
    }
    
    emit "This proposal will require:"
    emit "- 60% quorum"
    emit "- 70% approval"
    emit "- 120 hours deliberation (overridden from 96h in template)"
    emit "- 10 days voting period"
    emit "- 'member' and 'finance' roles to vote"
    emit ""
    
    # Use the emergency template for a time-sensitive decision
    emit "Emergency Decision Example"
    emit "========================="
    governance use "emergency"
    
    emit "This proposal will require:"
    emit "- 30% quorum"
    emit "- 80% approval"
    emit "- 1 hour deliberation"
    emit "- 24 hours voting period"
    emit "- 'guardian' role to vote"
    
    # Clean up the stack
    depth
    load depth_before
    sub
    iszero
    assert
    
    emit "Templates demonstration completed successfully."

# Run the demonstration
demonstrate_templates 