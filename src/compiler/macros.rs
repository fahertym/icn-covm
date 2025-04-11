use crate::vm::Op;
use std::collections::HashMap;

#[derive(Debug)]
pub struct ProposalLifecycleMacro {
    pub proposal_id: String,
    pub quorum: f64,
    pub threshold: f64,
    pub execution_block: Vec<String>,
    pub title: String,
    pub created_by: String,
    pub created_at: f64,
}

impl ProposalLifecycleMacro {
    pub fn new(
        proposal_id: String, 
        quorum: f64, 
        threshold: f64, 
        execution_block: Vec<String>,
        title: String,
        created_by: String,
        created_at: f64,
    ) -> Self {
        Self {
            proposal_id,
            quorum,
            threshold,
            execution_block,
            title,
            created_by,
            created_at,
        }
    }

    pub fn expand(&self) -> Vec<Op> {
        let mut ops = Vec::new();

        // Create proposal
        ops.push(Op::StoreP(self.proposal_id.clone()));

        // Store proposal metadata
        ops.push(Op::StoreP(format!("proposals/{}/metadata/title", self.proposal_id)));
        ops.push(Op::StoreP(format!("proposals/{}/metadata/created_by", self.proposal_id)));
        ops.push(Op::StoreP(format!("proposals/{}/metadata/created_at", self.proposal_id)));

        // Set quorum threshold
        ops.push(Op::QuorumThreshold(self.quorum));

        // Set vote threshold
        ops.push(Op::VoteThreshold(self.threshold));

        // Add execution block
        for line in &self.execution_block {
            ops.push(Op::Emit(line.clone()));
        }

        ops
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proposal_lifecycle_expansion() {
        let macro_block = ProposalLifecycleMacro::new(
            "prop-001".to_string(),
            0.6,
            0.5,
            vec!["Executing proposal prop-001...".to_string()],
            "Test Proposal".to_string(),
            "alice".to_string(),
            1625097600.0, // 2021-07-01 00:00:00 UTC
        );

        let expanded = macro_block.expand();
        
        assert_eq!(expanded.len(), 7); // StoreP + 3 metadata StoreP + QuorumThreshold + VoteThreshold + Emit
        
        // Verify StoreP operation for proposal ID
        match &expanded[0] {
            Op::StoreP(key) => assert_eq!(key, "prop-001"),
            _ => panic!("Expected StoreP operation"),
        }
        
        // Verify metadata StoreP operations
        match &expanded[1] {
            Op::StoreP(key) => assert_eq!(key, "proposals/prop-001/metadata/title"),
            _ => panic!("Expected StoreP operation"),
        }
        
        match &expanded[2] {
            Op::StoreP(key) => assert_eq!(key, "proposals/prop-001/metadata/created_by"),
            _ => panic!("Expected StoreP operation"),
        }
        
        match &expanded[3] {
            Op::StoreP(key) => assert_eq!(key, "proposals/prop-001/metadata/created_at"),
            _ => panic!("Expected StoreP operation"),
        }
        
        // Verify QuorumThreshold operation
        match &expanded[4] {
            Op::QuorumThreshold(value) => assert_eq!(*value, 0.6),
            _ => panic!("Expected QuorumThreshold operation"),
        }
        
        // Verify VoteThreshold operation
        match &expanded[5] {
            Op::VoteThreshold(value) => assert_eq!(*value, 0.5),
            _ => panic!("Expected VoteThreshold operation"),
        }
        
        // Verify Emit operation
        match &expanded[6] {
            Op::Emit(message) => assert_eq!(message, "Executing proposal prop-001..."),
            _ => panic!("Expected Emit operation"),
        }
    }
} 