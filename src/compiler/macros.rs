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
    pub passed_block: Vec<String>,
    pub failed_block: Option<Vec<String>>,
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
        passed_block: Vec<String>,
        failed_block: Option<Vec<String>>,
    ) -> Self {
        Self {
            proposal_id,
            quorum,
            threshold,
            execution_block,
            title,
            created_by,
            created_at,
            passed_block,
            failed_block,
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

        // Add passed block if present
        if !self.passed_block.is_empty() {
            let mut passed_ops = Vec::new();
            for line in &self.passed_block {
                passed_ops.push(Op::Emit(line.clone()));
            }
            ops.push(Op::IfPassed(passed_ops));
        }

        // Add failed block if present
        if let Some(failed_block) = &self.failed_block {
            let mut failed_ops = Vec::new();
            for line in failed_block {
                failed_ops.push(Op::Emit(line.clone()));
            }
            ops.push(Op::Else(failed_ops));
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
            vec!["Proposal passed!".to_string()],
            Some(vec!["Proposal failed.".to_string()]),
        );

        let expanded = macro_block.expand();
        
        assert_eq!(expanded.len(), 9); // StoreP + 3 metadata StoreP + QuorumThreshold + VoteThreshold + Emit + IfPassed + Else
        
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
        
        // Verify IfPassed operation
        match &expanded[7] {
            Op::IfPassed(block) => {
                assert_eq!(block.len(), 1);
                match &block[0] {
                    Op::Emit(message) => assert_eq!(message, "Proposal passed!"),
                    _ => panic!("Expected Emit operation in IfPassed block"),
                }
            },
            _ => panic!("Expected IfPassed operation"),
        }
        
        // Verify Else operation
        match &expanded[8] {
            Op::Else(block) => {
                assert_eq!(block.len(), 1);
                match &block[0] {
                    Op::Emit(message) => assert_eq!(message, "Proposal failed."),
                    _ => panic!("Expected Emit operation in Else block"),
                }
            },
            _ => panic!("Expected Else operation"),
        }
    }
} 