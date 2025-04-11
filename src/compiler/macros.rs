use crate::compiler::common::{Op, OpType};
use std::collections::HashMap;

#[derive(Debug)]
pub struct ProposalLifecycleMacro {
    pub proposal_id: String,
    pub quorum: f64,
    pub threshold: f64,
    pub execution_block: Vec<String>,
}

impl ProposalLifecycleMacro {
    pub fn new(proposal_id: String, quorum: f64, threshold: f64, execution_block: Vec<String>) -> Self {
        Self {
            proposal_id,
            quorum,
            threshold,
            execution_block,
        }
    }

    pub fn expand(&self) -> Vec<Op> {
        let mut ops = Vec::new();

        // Create proposal
        ops.push(Op {
            op_type: OpType::StoreP,
            args: vec![self.proposal_id.clone()],
            metadata: HashMap::new(),
        });

        // Set quorum threshold
        ops.push(Op {
            op_type: OpType::QuorumThreshold,
            args: vec![self.quorum.to_string()],
            metadata: HashMap::new(),
        });

        // Set vote threshold
        ops.push(Op {
            op_type: OpType::VoteThreshold,
            args: vec![self.threshold.to_string()],
            metadata: HashMap::new(),
        });

        // Add execution block
        for line in &self.execution_block {
            ops.push(Op {
                op_type: OpType::Emit,
                args: vec![line.clone()],
                metadata: HashMap::new(),
            });
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
            vec!["emit \"Executing proposal prop-001...\"".to_string()],
        );

        let expanded = macro_block.expand();
        
        assert_eq!(expanded.len(), 4); // StoreP + QuorumThreshold + VoteThreshold + Emit
        
        // Verify StoreP operation
        assert_eq!(expanded[0].op_type, OpType::StoreP);
        assert_eq!(expanded[0].args[0], "prop-001");
        
        // Verify QuorumThreshold operation
        assert_eq!(expanded[1].op_type, OpType::QuorumThreshold);
        assert_eq!(expanded[1].args[0], "0.6");
        
        // Verify VoteThreshold operation
        assert_eq!(expanded[2].op_type, OpType::VoteThreshold);
        assert_eq!(expanded[2].args[0], "0.5");
        
        // Verify Emit operation
        assert_eq!(expanded[3].op_type, OpType::Emit);
        assert_eq!(expanded[3].args[0], "emit \"Executing proposal prop-001...\"");
    }
} 