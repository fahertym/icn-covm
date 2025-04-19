// Test proposal
// A simple proposal for DAG testing

Governance {
    title = "Test DAG Linking"
    description = "This is a test proposal to verify DAG node linking"
    options {
        quorum = 0.1     // 10% quorum
        threshold = 0.5  // Simple majority
    }
}

// Execution logic - just emit some events
EmitEvent "governance" "Test proposal executed"
Push 42 