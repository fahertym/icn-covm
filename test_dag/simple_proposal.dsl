Governance {
    title = "Test DAG Linking"
    description = "This is a test proposal to verify DAG node linking"
    options {
        quorum = 0.1
        threshold = 0.5
    }
}

EmitEvent "governance" "Test proposal executed"
Push 42 