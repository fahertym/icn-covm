use crate::governance::traits::GovernanceOpHandler;
use crate::storage::traits::Storage;
use crate::vm::execution::ExecutorOps;
use crate::vm::stack::StackOps;
use crate::vm::types::Op;
use crate::vm::{VMError, VM};
use std::fmt::Debug;
use std::marker::{Send, Sync};

/// Handler for VoteThreshold operations
pub struct VoteThresholdHandler;

impl GovernanceOpHandler for VoteThresholdHandler {
    fn handle<S>(vm: &mut VM<S>, op: &Op) -> Result<(), VMError>
    where
        S: Storage + Send + Sync + Clone + Debug + 'static,
    {
        if let Op::VoteThreshold(threshold) = op {
            // Validate threshold (must be non-negative)
            if *threshold < 0.0 {
                return Err(VMError::GovernanceError(
                    "VoteThreshold must be non-negative".into(),
                ));
            }

            // Pop the total voting power from the stack
            let total_votes = vm.pop_one("VoteThreshold")?;

            // Log the calculation
            vm.executor.emit_event(
                "governance",
                &format!(
                    "Vote threshold check: {:.2} votes, threshold: {:.2}",
                    total_votes, threshold
                ),
            );

            // Push result to stack: 0.0 (truthy) if threshold met, 1.0 (falsey) if not
            if total_votes >= *threshold {
                vm.stack.push(0.0); // Threshold met (truthy in VM)
                vm.executor.emit_event("governance", "Vote threshold met");
            } else {
                vm.stack.push(1.0); // Threshold not met (falsey in VM)
                vm.executor
                    .emit_event("governance", "Vote threshold not met");
            }

            Ok(())
        } else {
            Err(VMError::UndefinedOperation(
                "Expected VoteThreshold operation".into(),
            ))
        }
    }
}
